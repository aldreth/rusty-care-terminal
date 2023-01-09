use git_discover::is_git;
use std::env;
use std::fs;

pub fn get_directories() -> Vec<fs::DirEntry> {
    env::var("RCT_REPOS")
        .expect("RCT_REPOS environment variable is not set")
        .split(',')
        .flat_map(|dir| {
            fs::read_dir(dir)
                .expect("Directory from RCT_REPOS not found")
                .flatten()
                .filter(|x| { 
                    println!("path {:#?}", x.path());
                    !x.path().is_file()})
                .filter(|x| is_git(x.path().join(".git")).is_ok())
        })
        .collect()
}
pub fn get_author() -> String {
    git2::Config::open_default()
        .expect("No git config found")
        .get_string("user.name")
        .expect("No git user name set")
}
