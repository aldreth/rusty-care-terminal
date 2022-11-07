use git_discover::is_git;
use std::env;
use std::fs;

pub fn get_directories() -> std::vec::IntoIter<fs::DirEntry> {
    let mut dirs: Vec<fs::DirEntry> = Vec::new();

    env::var("TTC_REPOS")
        .expect("TTC_REPOS environment variable is not set")
        .split(',')
        .for_each(|dir| {
            if let Ok(entries) = fs::read_dir(dir) {
                entries
                    .flatten()
                    .filter(|x| !x.path().is_file())
                    .filter(|x| is_git(x.path().join(".git")).is_ok())
                    .for_each(|y| {
                        dirs.push(y);
                    })
            }
        });
    dirs.into_iter()
}
