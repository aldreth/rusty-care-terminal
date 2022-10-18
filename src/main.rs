/*
 * libgit2 "log" example - shows how to walk history and get commit info
 *
 * Written by the libgit2 contributors
 *
 * To the extent possible under law, the author(s) have dedicated all copyright
 * and related and neighboring rights to this software to the public domain
 * worldwide. This software is distributed without any warranty.
 *
 * You should have received a copy of the CC0 Public Domain Dedication along
 * with this software. If not, see
 * <http://creativecommons.org/publicdomain/zero/1.0/>.
 */

// eg `cargo run

// #![deny(warnings)]

use git2::{Commit, DiffOptions, Repository, Signature, Time};
use git2::{Error, Pathspec};
use std::{
    str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(name = "spec", last = true)]
    arg_spec: Vec<String>,
}

fn run(args: &Args) -> Result<(), Error> {
    // TODO: get from env
    let path = "/Users/edward.andrewshodgson/Developer/work/dashboards-and-visualisations";
    let author = "Edward Andrews-Hodgson";

    let repo = Repository::open(path)?;
    let mut revwalk = repo.revwalk()?;

    // Prepare the revwalk based on CLI parameters
    revwalk.set_sorting(git2::Sort::NONE)?;
    revwalk.push_head()?;

    // Prepare our diff options and pathspec matcher
    let mut diffopts = DiffOptions::new();

    let ps = Pathspec::new(args.arg_spec.iter())?;

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

        if !args.arg_spec.is_empty() {
            match commit.parents().len() {
                0 => {
                    let tree = filter_try!(commit.tree());
                    let flags = git2::PathspecFlags::NO_MATCH_ERROR;
                    if ps.match_tree(&tree, flags).is_err() {
                        return None;
                    }
                }
                _ => {
                    let m = commit.parents().all(|parent| {
                        match_with_parent(&repo, &commit, &parent, &mut diffopts).unwrap_or(false)
                    });
                    if !m {
                        return None;
                    }
                }
            }
        }
        if !sig_matches(&commit.author(), author) {
            return None;
        }
        let one_day = Duration::from_secs(60 * 60 * 24);
        let one_week = one_day * 7;

        if !time_within(commit.time(), one_week) {
            return None;
        }

        Some(Ok(commit))
    });

    // print!
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
    time > Time::new(then, 0)
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

fn match_with_parent(
    repo: &Repository,
    commit: &Commit,
    parent: &Commit,
    opts: &mut DiffOptions,
) -> Result<bool, Error> {
    let a = parent.tree()?;
    let b = commit.tree()?;
    let diff = repo.diff_tree_to_tree(Some(&a), Some(&b), Some(opts))?;
    Ok(diff.deltas().len() > 0)
}

impl Args {}

fn main() {
    let args = Args::from_args();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
