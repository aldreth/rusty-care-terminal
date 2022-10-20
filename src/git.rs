use git2::{Error, Oid};
use git2::{Repository, Signature, Time};
use std::{
    str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub struct GitInfo(pub Oid, pub String, pub String);

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
