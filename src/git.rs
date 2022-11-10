use crate::config;
use git2::{Error, Oid, Repository, Signature, Time};
use std::{
    str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type CommitSummary = String;
type CommitTime = String;
#[derive(Debug)]
pub struct GitInfo(pub Oid, pub CommitSummary, pub CommitTime);

#[derive(Debug)]
pub struct RepoInfo(pub String, pub Vec<GitInfo>);

pub fn get_repo_info(days: u32) -> Vec<RepoInfo> {
    let author = config::get_author();
    let directories = config::get_directories();

    let mut vec: Vec<RepoInfo> = Vec::new();

    directories.into_iter().for_each(|dir| {
        let this_path = dir.path();
        let str_path = this_path.to_str().expect("This should be a path");
        let commits = get_commits(str_path, &author, days);

        match commits {
            Ok(commits) => {
                if !commits.is_empty() {
                    vec.push(RepoInfo(str_path.to_string(), commits));
                }
            }
            Err(err) => println!("{:?}", err),
        }
    });
    vec
}

// TODO: this won't need to be public once get_repo_info is used
pub fn get_commits(path: &str, author: &str, days: u32) -> Result<Vec<GitInfo>, Error> {
    let one_day = Duration::from_secs(60 * 60 * 24);
    let mut vec: Vec<GitInfo> = Vec::new();

    let repo = Repository::open(path)?;
    let mut revwalk = repo.revwalk()?;

    // Prepare the revwalk based on CLI parameters
    revwalk.set_sorting(git2::Sort::NONE)?;
    revwalk.push_head()?;

    // Filter our revwalk based on the CLI parameters
    macro_rules! filter_try {
        ($e:expr) => {
            match $e {
                Ok(t) => t,
                Err(e) => return Some(Err(e)),
            }
        };
    }

    let revwalk = revwalk.filter_map(|id| {
        let id = filter_try!(id);
        let commit = filter_try!(repo.find_commit(id));

        if !sig_matches(&commit.author(), author) {
            return None;
        }

        if !time_within(commit.time(), one_day * days) {
            return None;
        }

        Some(Ok(commit))
    });

    for commit in revwalk {
        let commit = commit?;

        let info: GitInfo = GitInfo(
            commit.id(),
            commit.summary().unwrap_or_default().to_string(),
            format_time(commit.author().when()),
        );
        vec.push(info)
    }

    Ok(vec)
}

fn sig_matches(sig: &Signature, name: &str) -> bool {
    sig.name().map(|n| n.contains(name)).unwrap_or(false)
        || sig.email().map(|n| n.contains(name)).unwrap_or(false)
}

fn time_within(time: Time, duration: Duration) -> bool {
    let then = SystemTime::now()
        .checked_sub(duration)
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    time.seconds() > then
}

fn format_time(time: Time) -> String {
    // TODO: use updated or system version of time here
    let ts = time::Timespec::new(time.seconds() + (time.offset_minutes() as i64) * 60, 0);
    let time = time::at(ts);
    time.strftime("%a %b %e %T %Y").unwrap().to_string()
}
