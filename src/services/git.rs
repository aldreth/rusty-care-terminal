use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Local};
use walkdir::WalkDir;

use crate::error::{CareError, Result};
use crate::model::{CommitEntry, CommitStats};

/// Discover git repos under configured roots and collect commits where **you**
/// are sole or joint author: git author, committer, or a `Co-authored-by` trailer
/// matches your identity (`RCT_GIT_AUTHOR`, else each repo's `user.email` / `user.name`).
pub fn collect_commits(
    roots: &[PathBuf],
    depth: usize,
    author_override: Option<&str>,
) -> (CommitStats, Vec<String>) {
    let repos = discover_repos(roots, depth);
    let mut stats = CommitStats::default();
    let mut warnings = Vec::new();

    for repo in &repos {
        let identities = identities_for_repo(repo, author_override);
        if identities.is_empty() {
            warnings.push(format!(
                "{}: no git user.email/user.name (and no RCT_GIT_AUTHOR); skipping author filter — showing nothing. Set identity to see your commits.",
                repo.display()
            ));
            // Safer empty than dumping everyone else's history when identity is unknown.
            continue;
        }

        match log_commits(repo, "1 day ago", &identities) {
            Ok(mut entries) => stats.today.append(&mut entries),
            Err(e) => warnings.push(e.to_string()),
        }
        match log_commits(repo, "7 days ago", &identities) {
            Ok(mut entries) => stats.week.append(&mut entries),
            Err(e) => warnings.push(e.to_string()),
        }
    }

    if roots.iter().any(|r| !r.exists()) {
        for root in roots {
            if !root.exists() {
                warnings.push(format!("repo path missing: {}", root.display()));
            }
        }
    }

    stats.today.sort_by_key(|c| std::cmp::Reverse(c.when));
    stats.week.sort_by_key(|c| std::cmp::Reverse(c.when));
    (stats, warnings)
}

pub fn discover_repos(roots: &[PathBuf], depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for root in roots {
        if !root.exists() {
            continue;
        }

        if is_git_repo(root) {
            found.push(root.clone());
        }

        let max_depth = depth.max(1);
        for entry in WalkDir::new(root)
            .follow_links(false)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| e.file_name() != ".git")
            .flatten()
        {
            let path = entry.path();
            if entry.file_type().is_dir() && path.join(".git").exists() {
                let repo = path.to_path_buf();
                if !found.iter().any(|p| p == &repo) {
                    found.push(repo);
                }
            }
        }
    }

    found.sort();
    found.dedup();
    found
}

fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

fn identities_for_repo(repo: &Path, override_author: Option<&str>) -> Vec<String> {
    if let Some(raw) = override_author {
        return split_authors(raw);
    }

    let mut ids = Vec::new();
    for key in ["user.email", "user.name"] {
        if let Some(v) = git_config(repo, key)
            && !ids
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&v))
        {
            ids.push(v);
        }
    }
    ids
}

fn split_authors(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn git_config(repo: &Path, key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", repo.to_str().unwrap_or("."), "config", "--get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn log_commits(repo: &Path, since: &str, identities: &[String]) -> Result<Vec<CommitEntry>> {
    // Fields: hash, date, author name, author email, committer name, committer email,
    // subject, Co-authored-by trailer values (pipe-separated).
    let pretty = "%h%x09%ad%x09%an%x09%ae%x09%cn%x09%ce%x09%s%x09%(trailers:key=Co-authored-by,valueonly,separator=|)";
    let output = Command::new("git")
        .args([
            "-C",
            repo.to_str().unwrap_or("."),
            "log",
            "--all",
            &format!("--since={since}"),
            "--date=iso-strict",
            &format!("--pretty=format:{pretty}"),
        ])
        .output()
        .map_err(|e| CareError::Git {
            path: repo.display().to_string(),
            message: format!("failed to spawn git: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CareError::Git {
            path: repo.display().to_string(),
            message: stderr.trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let repo_label = display_repo_path(repo);

    let mut entries = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(8, '\t').collect();
        if parts.len() < 7 {
            continue;
        }
        let author_name = parts[2];
        let author_email = parts[3];
        let committer_name = parts[4];
        let committer_email = parts[5];
        let subject = parts[6];
        let coauthors = parts.get(7).copied().unwrap_or("");

        if !involves_you(
            identities,
            author_name,
            author_email,
            committer_name,
            committer_email,
            coauthors,
        ) {
            continue;
        }

        let when = parse_git_date(parts[1]).unwrap_or_else(Local::now);
        entries.push(CommitEntry {
            repo: repo_label.clone(),
            hash: parts[0].to_string(),
            author: author_name.to_string(),
            subject: subject.to_string(),
            when,
        });
    }
    Ok(entries)
}

/// Prefer `~/…` for paths under `$HOME` so headings stay short.
fn display_repo_path(repo: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        if let Ok(rest) = repo.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    repo.display().to_string()
}

/// True if any identity matches author, committer, or a Co-authored-by trailer.
fn involves_you(
    identities: &[String],
    author_name: &str,
    author_email: &str,
    committer_name: &str,
    committer_email: &str,
    coauthored_by: &str,
) -> bool {
    let haystacks = [
        author_name,
        author_email,
        committer_name,
        committer_email,
        coauthored_by,
    ];
    identities.iter().any(|id| {
        let id = id.trim();
        if id.is_empty() {
            return false;
        }
        haystacks
            .iter()
            .any(|h| !h.is_empty() && contains_ignore_case(h, id))
    })
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn parse_git_date(raw: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Local))
        .or_else(|| {
            chrono::DateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S %z")
                .ok()
                .map(|dt| dt.with_timezone(&Local))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sole_author_matches() {
        let ids = vec!["edward@aldreth.com".into()];
        assert!(involves_you(
            &ids,
            "Edward",
            "edward@aldreth.com",
            "Edward",
            "edward@aldreth.com",
            "",
        ));
    }

    #[test]
    fn joint_via_coauthored_by() {
        let ids = vec!["edward@aldreth.com".into(), "Edward".into()];
        assert!(involves_you(
            &ids,
            "Cursor Agent",
            "cursoragent@cursor.com",
            "Cursor Agent",
            "cursoragent@cursor.com",
            "Edward Andrews-Hodgson <edward@aldreth.com>",
        ));
    }

    #[test]
    fn unrelated_author_excluded() {
        let ids = vec!["edward@aldreth.com".into()];
        assert!(!involves_you(
            &ids,
            "Someone Else",
            "else@example.com",
            "Someone Else",
            "else@example.com",
            "Bot <bot@example.com>",
        ));
    }

    #[test]
    fn joint_via_committer() {
        let ids = vec!["edward@aldreth.com".into()];
        assert!(involves_you(
            &ids,
            "Cursor Agent",
            "cursoragent@cursor.com",
            "Edward",
            "edward@aldreth.com",
            "",
        ));
    }
}
