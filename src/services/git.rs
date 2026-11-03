use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Duration, Local};
use walkdir::WalkDir;

use crate::error::{CareError, Result};
use crate::model::{CommitEntry, CommitStats, clean_text};

const FIELD: char = '\u{1f}';
const COAUTHOR_SEP: char = '\u{1e}';

/// Discover git repos under configured roots and collect commits where **you**
/// are sole or joint author: git author, committer, or a `Co-authored-by` trailer
/// matches your identity (`RCT_GIT_AUTHOR`, else each repo's `user.email` / `user.name`).
pub fn collect_commits(
    roots: &[PathBuf],
    depth: usize,
    author_override: Option<&str>,
) -> (CommitStats, Vec<String>) {
    let mut stats = CommitStats::default();
    let mut warnings: Vec<String> = roots
        .iter()
        .filter(|r| !r.exists())
        .map(|r| format!("repo path missing: {}", r.display()))
        .collect();

    let day_ago = Local::now() - Duration::hours(24);
    for repo in discover_repos(roots, depth) {
        let identities = identities_for_repo(&repo, author_override);
        if identities.is_empty() {
            // Safer to show nothing than everyone else's history.
            warnings.push(format!(
                "{}: no git user.email/user.name — set RCT_GIT_AUTHOR to see your commits",
                repo.display()
            ));
            continue;
        }

        match log_week(&repo, &identities) {
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

fn identities_for_repo(repo: &Path, override_author: Option<&str>) -> Vec<String> {
    if let Some(raw) = override_author {
        return raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
    }
    ["user.email", "user.name"]
        .iter()
        .filter_map(|key| git_config(repo, key))
        .collect()
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
fn log_week(repo: &Path, identities: &[String]) -> Result<Vec<(CommitEntry, DateTime<Local>)>> {
    // hash, author date, committer date, author name/email, committer name/email,
    // subject, Co-authored-by values.
    let pretty = "%h%x1f%aI%x1f%cI%x1f%an%x1f%ae%x1f%cn%x1f%ce%x1f%s%x1f\
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
        .map_err(|e| CareError::Git {
            path: repo.display().to_string(),
            message: format!("failed to run git: {e}"),
        })?;

    if !output.status.success() {
        return Err(CareError::Git {
            path: repo.display().to_string(),
            message: clean_text(&String::from_utf8_lossy(&output.stderr)),
        });
    }

    let repo_label = display_repo_path(repo);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(|line| parse_log_line(line, identities, &repo_label))
        .collect())
}

fn parse_log_line(
    line: &str,
    identities: &[String],
    repo_label: &str,
) -> Option<(CommitEntry, DateTime<Local>)> {
    let f: Vec<&str> = line.splitn(9, FIELD).collect();
    let [
        hash,
        authored,
        committed,
        an,
        ae,
        cn,
        ce,
        subject,
        coauthors,
    ] = f[..]
    else {
        return None;
    };
    let mut people = vec![(an, ae), (cn, ce)];
    people.extend(
        coauthors
            .split(COAUTHOR_SEP)
            .filter(|c| !c.trim().is_empty())
            .map(parse_person),
    );
    if !involves_you(identities, &people) {
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

/// `Name <email>` → `(name, email)`; a bare value is treated as both.
fn parse_person(raw: &str) -> (&str, &str) {
    match raw.split_once('<') {
        Some((name, rest)) => (name.trim(), rest.trim_end().trim_end_matches('>').trim()),
        None => (raw.trim(), raw.trim()),
    }
}

/// True if an identity exactly matches (case-insensitively) the name or email of
/// the author, committer, or a co-author. Values containing `@` match emails;
/// anything else matches names.
fn involves_you(identities: &[String], people: &[(&str, &str)]) -> bool {
    identities.iter().any(|id| {
        let (id_name, id_email) = parse_person(id);
        if id_email.contains('@') {
            people
                .iter()
                .any(|(_, email)| email.eq_ignore_ascii_case(id_email))
        } else {
            let wanted = id_name.to_lowercase();
            !wanted.is_empty() && people.iter().any(|(name, _)| name.to_lowercase() == wanted)
        }
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

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sole_author_matches() {
        let people = [
            ("Edward", "edward@aldreth.com"),
            ("Edward", "edward@aldreth.com"),
        ];
        assert!(involves_you(&ids(&["edward@aldreth.com"]), &people));
    }

    #[test]
    fn joint_via_coauthored_by() {
        let people = [
            ("Cursor Agent", "cursoragent@cursor.com"),
            ("Cursor Agent", "cursoragent@cursor.com"),
            parse_person("Edward Andrews-Hodgson <Edward@Aldreth.com>"),
        ];
        assert!(involves_you(&ids(&["edward@aldreth.com"]), &people));
    }

    #[test]
    fn joint_via_committer() {
        let people = [
            ("Cursor Agent", "cursoragent@cursor.com"),
            ("Edward", "edward@aldreth.com"),
        ];
        assert!(involves_you(&ids(&["edward@aldreth.com"]), &people));
    }

    #[test]
    fn unrelated_and_partial_matches_are_excluded() {
        let people = [
            ("Fred", "fred@aldreth.com"),
            ("Edwardo", "else@example.com"),
            parse_person("Bot <bot@example.com>"),
        ];
        assert!(!involves_you(&ids(&["ed@aldreth.com", "Edward"]), &people));
    }

    #[test]
    fn names_match_whole_and_case_insensitively() {
        let people = [("Zoë Smith", "z@example.com")];
        assert!(involves_you(&ids(&["zoë smith"]), &people));
        assert!(involves_you(&ids(&["Zoë <z@example.com>"]), &people));
    }

    #[test]
    fn parses_a_log_line() {
        let line = [
            "abc1234",
            "2026-11-03T10:00:00+00:00",
            "2026-11-03T11:00:00+00:00",
            "Someone",
            "someone@example.com",
            "Someone",
            "someone@example.com",
            "Fix\u{1b}[31m thing",
            "Edward <edward@aldreth.com>\u{1e}Other <o@example.com>",
        ]
        .join("\u{1f}");
        let (entry, committed) =
            parse_log_line(&line, &ids(&["edward@aldreth.com"]), "~/x").expect("mine");
        assert_eq!(entry.hash, "abc1234");
        assert!(!entry.subject.contains('\u{1b}'));
        assert!(committed > entry.when);
        assert!(parse_log_line(&line, &ids(&["nobody@example.com"]), "~/x").is_none());
        assert!(parse_log_line("garbage", &ids(&["x"]), "~/x").is_none());
    }
}
