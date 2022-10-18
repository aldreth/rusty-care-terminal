#![deny(warnings)]

use git2::Error;
use git2::{Commit, Repository, Signature, Time};
use std::{
    str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn run() -> Result<(), Error> {
    let one_day = Duration::from_secs(60 * 60 * 24);
    let one_week = one_day * 7;

    // TODO: get from env
    let path = "/Users/edward.andrewshodgson/Developer/work/dashboards-and-visualisations";
    let author = "Edward Andrews-Hodgson";

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

        if !time_within(commit.time(), one_week) {
            return None;
        }

        Some(Ok(commit))
    });

    for commit in revwalk {
        let commit = commit?;
        print_commit(&commit);
    }

    Ok(())
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

fn print_commit(commit: &Commit) {
    println!("commit {}", commit.id());

    if commit.parents().len() > 1 {
        print!("Merge:");
        for id in commit.parent_ids() {
            print!(" {:.8}", id);
        }
        println!();
    }

    let author = commit.author();
    println!("Author: {}", author);
    print_time(&author.when(), "Date:   ");
    println!();

    for line in String::from_utf8_lossy(commit.message_bytes()).lines() {
        println!("    {}", line);
    }
    println!();
}

fn print_time(time: &Time, prefix: &str) {
    let (offset, sign) = match time.offset_minutes() {
        n if n < 0 => (-n, '-'),
        n => (n, '+'),
    };
    let (hours, minutes) = (offset / 60, offset % 60);

    // TODO: use updated or system version of time here
    let ts = time::Timespec::new(time.seconds() + (time.offset_minutes() as i64) * 60, 0);
    let time = time::at(ts);

    println!(
        "{}{} {}{:02}{:02}",
        prefix,
        time.strftime("%a %b %e %T %Y").unwrap(),
        sign,
        hours,
        minutes
    );
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
