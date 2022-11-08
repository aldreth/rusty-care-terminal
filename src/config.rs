use git_discover::is_git;
use std::env;
use std::fs;

pub fn get_directories() -> Vec<fs::DirEntry> {
    env::var("TTC_REPOS")
        .expect("TTC_REPOS environment variable is not set")
        .split(',')
        .flat_map(|dir| {
            fs::read_dir(dir)
                .expect("Directory from TTC_REPOS not found")
                .flatten()
                .filter(|x| !x.path().is_file())
                .filter(|x| is_git(x.path().join(".git")).is_ok())
        })
        .collect()
}
