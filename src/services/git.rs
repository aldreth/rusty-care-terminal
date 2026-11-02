use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Local};
use walkdir::WalkDir;

use crate::error::{CareError, Result};
use crate::model::{CommitEntry, CommitStats};

/// Discover git repos under configured roots (depth-limited) and collect commits.
pub fn collect_commits(roots: &[PathBuf], depth: usize) -> Result<CommitStats> {
    let repos = discover_repos(roots, depth);
    let mut stats = CommitStats::default();

    for repo in repos {
        match log_commits(&repo, "1 day ago") {
            Ok(mut entries) => stats.today.append(&mut entries),
            Err(e) => {
                // Soft-fail per repo so one bad path does not kill the dashboard.
                eprintln!("warning: {e}");
            }
        }
        match log_commits(&repo, "7 days ago") {
            Ok(mut entries) => stats.week.append(&mut entries),
            Err(e) => eprintln!("warning: {e}"),
        }
    }

    stats.today.sort_by(|a, b| b.when.cmp(&a.when));
    stats.week.sort_by(|a, b| b.when.cmp(&a.when));
    Ok(stats)
}

pub fn discover_repos(roots: &[PathBuf], depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for root in roots {
        if !root.exists() {
            eprintln!("warning: repo path missing: {}", root.display());
            continue;
        }

        if is_git_repo(root) {
            found.push(root.clone());
            // Still walk for nested repos when depth > 1.
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

fn log_commits(repo: &Path, since: &str) -> Result<Vec<CommitEntry>> {
    let output = Command::new("git")
        .args([
            "-C",
            repo.to_str().unwrap_or("."),
            "log",
            "--all",
            &format!("--since={since}"),
            "--date=iso-strict",
            "--pretty=format:%h%x09%ad%x09%an%x09%s",
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
    let repo_name = repo
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| repo.display().to_string());

    let mut entries = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        if parts.len() < 4 {
            continue;
        }
        let when = parse_git_date(parts[1]).unwrap_or_else(Local::now);
        entries.push(CommitEntry {
            repo: repo_name.clone(),
            hash: parts[0].to_string(),
            author: parts[2].to_string(),
            subject: parts[3].to_string(),
            when,
        });
    }
    Ok(entries)
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
