// TODO: Don't forget to remove these
#![allow(dead_code)]
#![allow(unused_imports)]

use std::error::Error;

mod config;
mod git;
mod ui;

// fn main() -> Result<(), Box<dyn Error>> {
//     ui::run_ui()
// }

fn main() {
    println!();
    println!();
    let info = git::get_repo_info(7);
    info.into_iter().for_each(|i| {
        let path = i.0;
        let git_info = i.1;

        println!("path {}", path);
        git_info.into_iter().for_each(|g| println!("{}", g.1));
        println!();
    })
}
