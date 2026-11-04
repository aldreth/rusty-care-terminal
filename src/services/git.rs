use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Duration, Local};
use walkdir::WalkDir;

use crate::error::{AppError, Result};
use crate::model::{CommitEntry, CommitStats, clean_text};

const FIELD: char = '\u{1f}';
const COAUTHOR_SEP: char = '\u{1e}';

/// Discover git repos under configured roots and collect commits where **you**
/// are sole or joint author: the author, committer or a `Co-authored-by` email
/// is one of yours (`RCT_GIT_AUTHOR_EMAILS`, else each repo's `user.email`).
pub fn collect_commits(
    roots: &[PathBuf],
    depth: usize,
    your_emails: &[String],
) -> (CommitStats, Vec<String>) {
    let mut stats = CommitStats::default();
    let mut warnings: Vec<String> = roots
        .iter()
        .filter(|r| !r.exists())
        .map(|r| format!("repo path missing: {}", r.display()))
        .collect();

    let day_ago = Local::now() - Duration::hours(24);
    for repo in discover_repos(roots, depth) {
        let emails = if your_emails.is_empty() {
            match git_config(&repo, "user.email") {
                Some(email) => vec![email.to_lowercase()],
                None => {
                    // Safer to show nothing than everyone else's history.
                    warnings.push(format!(
                        "{}: no git user.email — set RCT_GIT_AUTHOR_EMAILS to see your commits",
                        repo.display()
                    ));
                    continue;
                }
            }
        } else {
            your_emails.to_vec()
        };

        match log_week(&repo, &emails) {
            Ok(commits) => {
                for (entry, committed) in commits {
                    if committed >= day_ago {
                        stats.today.push(entry.clone());
                    }
                    stats.week.push(entry);
                }
            }
            Err(e) => warnings.push(e.to_string()),
        }
    }

    stats.today.sort_by_key(|c| std::cmp::Reverse(c.when));
    stats.week.sort_by_key(|c| std::cmp::Reverse(c.when));
    (stats, warnings)
}

pub fn discover_repos(roots: &[PathBuf], depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in roots.iter().filter(|r| r.exists()) {
        for entry in WalkDir::new(root)
            .follow_links(false)
            .max_depth(depth.max(1))
            .into_iter()
            .filter_entry(|e| e.file_name() != ".git")
            .flatten()
        {
            if entry.file_type().is_dir() && entry.path().join(".git").exists() {
                found.push(entry.into_path());
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

fn git(repo: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo);
    cmd
}

fn git_config(repo: &Path, key: &str) -> Option<String> {
    let output = git(repo).args(["config", "--get", key]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

/// Your commits from the last 7 days with their committer time (used for "today").
fn log_week(repo: &Path, your_emails: &[String]) -> Result<Vec<(CommitEntry, DateTime<Local>)>> {
    // hash, author date, committer date, author email, committer email, subject,
    // Co-authored-by values.
    let pretty = "%h%x1f%aI%x1f%cI%x1f%ae%x1f%ce%x1f%s%x1f\
                  %(trailers:key=Co-authored-by,valueonly,separator=%x1e)";
    let output = git(repo)
        .args([
            "log",
            "--exclude=refs/stash",
            "--all",
            "--no-color",
            "--no-show-signature",
            "--since=7.days.ago",
            &format!("--pretty=format:{pretty}"),
        ])
        .output()
        .map_err(|e| AppError::Git {
            path: repo.display().to_string(),
            message: format!("failed to run git: {e}"),
        })?;

    if !output.status.success() {
        return Err(AppError::Git {
            path: repo.display().to_string(),
            message: clean_text(&String::from_utf8_lossy(&output.stderr)),
        });
    }

    let repo_label = display_repo_path(repo);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(|line| parse_log_line(line, your_emails, &repo_label))
        .collect())
}

fn parse_log_line(
    line: &str,
    your_emails: &[String],
    repo_label: &str,
) -> Option<(CommitEntry, DateTime<Local>)> {
    let f: Vec<&str> = line.splitn(7, FIELD).collect();
    let [
        hash,
        authored,
        committed,
        author,
        committer,
        subject,
        coauthors,
    ] = f[..]
    else {
        return None;
    };
    let mut emails = vec![author, committer];
    emails.extend(coauthors.split(COAUTHOR_SEP).map(trailer_email));
    if !involves_you(your_emails, &emails) {
        return None;
    }
    let committed = parse_git_date(committed)?;
    Some((
        CommitEntry {
            repo: repo_label.to_string(),
            hash: clean_text(hash),
            subject: clean_text(subject),
            when: parse_git_date(authored).unwrap_or(committed),
        },
        committed,
    ))
}

/// The email in a `Name <email>` trailer value.
fn trailer_email(raw: &str) -> &str {
    raw.split_once('<')
        .map_or(raw, |(_, rest)| {
            rest.split_once('>').map_or(rest, |(email, _)| email)
        })
        .trim()
}

/// Exact, case-insensitive email match, so `ed@example.com` never matches
/// `fred@example.com`. `your_emails` are already lowercase.
fn involves_you(your_emails: &[String], emails: &[&str]) -> bool {
    emails.iter().any(|email| {
        your_emails
            .iter()
            .any(|yours| email.eq_ignore_ascii_case(yours))
    })
}

/// Prefer `~/…` for paths under the home directory so headings stay short.
fn display_repo_path(repo: &Path) -> String {
    let shown = std::env::home_dir()
        .and_then(|home| repo.strip_prefix(home).ok().map(Path::to_path_buf))
        .map(|rest| format!("~/{}", rest.display()))
        .unwrap_or_else(|| repo.display().to_string());
    clean_text(&shown)
}

fn parse_git_date(raw: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(raw.trim())
        .ok()
        .map(|dt| dt.with_timezone(&Local))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yours() -> Vec<String> {
        vec!["edward@aldreth.com".to_string()]
    }

    #[test]
    fn author_committer_or_coauthor_email_matches() {
        assert!(involves_you(
            &yours(),
            &["edward@aldreth.com", "x@example.com"]
        ));
        assert!(involves_you(
            &yours(),
            &["x@example.com", "Edward@Aldreth.com"]
        ));
        assert!(involves_you(
            &yours(),
            &[
                "x@example.com",
                "x@example.com",
                trailer_email("Edward A-H <edward@aldreth.com>")
            ]
        ));
    }

    #[test]
    fn partial_and_unrelated_emails_are_excluded() {
        let emails = [
            "fred@aldreth.com",
            "edward@aldreth.com.evil",
            trailer_email("Edward"),
        ];
        assert!(!involves_you(&yours(), &emails));
    }

    #[test]
    fn reads_trailer_emails() {
        assert_eq!(
            trailer_email("Edward <edward@aldreth.com>"),
            "edward@aldreth.com"
        );
        assert_eq!(trailer_email(" edward@aldreth.com "), "edward@aldreth.com");
        assert_eq!(
            trailer_email("Edward <edward@aldreth.com> (work)"),
            "edward@aldreth.com"
        );
    }

    #[test]
    fn parses_a_log_line() {
        let line = [
            "abc1234",
            "2026-11-03T10:00:00+00:00",
            "2026-11-03T11:00:00+00:00",
            "someone@example.com",
            "someone@example.com",
            "Fix\u{1b}[31m thing",
            "Edward <edward@aldreth.com>\u{1e}Other <o@example.com>",
        ]
        .join("\u{1f}");
        let (entry, committed) = parse_log_line(&line, &yours(), "~/x").expect("mine");
        assert_eq!(entry.hash, "abc1234");
        assert!(!entry.subject.contains('\u{1b}'));
        assert!(committed > entry.when);
        assert!(parse_log_line(&line, &["nobody@example.com".into()], "~/x").is_none());
        assert!(parse_log_line("garbage", &yours(), "~/x").is_none());
    }

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("rct-git-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let status = git(repo)
            .args(["-c", "commit.gpgsign=false", "-c", "core.hooksPath="])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_repo(path: &Path, local_email: &str) {
        std::fs::create_dir_all(path).unwrap();
        run_git(path, &["init", "-q"]);
        run_git(path, &["config", "user.email", local_email]);
        run_git(path, &["config", "user.name", "Test"]);
    }

    fn commit(repo: &Path, message: &str, author: &str, committer: &str, days_ago: i64) {
        let when = (Local::now() - Duration::days(days_ago)).to_rfc3339();
        let status = git(repo)
            .args([
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "--allow-empty",
            ])
            .args(["--no-verify", "-m", message])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Author")
            .env("GIT_AUTHOR_EMAIL", author)
            .env("GIT_COMMITTER_NAME", "Committer")
            .env("GIT_COMMITTER_EMAIL", committer)
            .env("GIT_AUTHOR_DATE", &when)
            .env("GIT_COMMITTER_DATE", &when)
            .status()
            .expect("git runs");
        assert!(status.success(), "commit {message:?} failed");
    }

    /// `root/mine` with a mix of commits, plus `root/nested/deep` two levels down.
    fn fixture_repos(name: &str) -> PathBuf {
        let root = temp_root(name);
        let mine = root.join("mine");
        init_repo(&mine, "edward@aldreth.com");
        // Oldest first: `--since` stops walking at the first commit past the cutoff.
        commit(
            &mine,
            "too old",
            "edward@aldreth.com",
            "edward@aldreth.com",
            10,
        );
        commit(
            &mine,
            "mine today",
            "edward@aldreth.com",
            "edward@aldreth.com",
            0,
        );
        commit(
            &mine,
            "pairing\n\nCo-authored-by: Ed <Edward@Aldreth.com>",
            "pair@example.com",
            "pair@example.com",
            3,
        );
        commit(
            &mine,
            "not mine",
            "other@example.com",
            "other@example.com",
            0,
        );
        init_repo(&root.join("nested/deep"), "work@example.com");
        commit(
            &root.join("nested/deep"),
            "work today",
            "work@example.com",
            "work@example.com",
            0,
        );
        std::fs::create_dir_all(root.join("not-a-repo")).unwrap();
        root
    }

    fn subjects(entries: &[CommitEntry]) -> Vec<&str> {
        entries.iter().map(|c| c.subject.as_str()).collect()
    }

    #[test]
    fn collects_your_commits_from_real_repos() {
        let root = fixture_repos("explicit");
        let missing = root.join("missing");
        let (stats, warnings) = collect_commits(&[root.clone(), missing.clone()], 1, &yours());

        assert_eq!(subjects(&stats.today), ["mine today"]);
        assert_eq!(subjects(&stats.week), ["mine today", "pairing"]);
        assert_eq!(
            warnings,
            [format!("repo path missing: {}", missing.display())]
        );
        assert!(stats.week.iter().all(|c| c.repo.ends_with("mine")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn falls_back_to_each_repos_user_email_and_searches_deeper() {
        let root = fixture_repos("fallback");
        assert_eq!(
            discover_repos(std::slice::from_ref(&root), 3),
            [root.join("mine"), root.join("nested/deep")]
        );
        let (stats, warnings) = collect_commits(std::slice::from_ref(&root), 3, &[]);
        assert!(warnings.is_empty(), "{warnings:?}");
        let mut today = subjects(&stats.today);
        today.sort_unstable();
        assert_eq!(today, ["mine today", "work today"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn broken_repo_is_a_warning_not_a_crash() {
        let root = temp_root("broken");
        std::fs::create_dir_all(root.join("fake/.git")).unwrap();
        let (stats, warnings) = collect_commits(std::slice::from_ref(&root), 1, &yours());
        assert!(stats.week.is_empty());
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        let _ = std::fs::remove_dir_all(root);
    }
}
