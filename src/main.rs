// TODO: Don't forget to remove these
#![allow(dead_code)]
#![allow(unused_imports)]

mod config;
mod git;
mod ui;

// fn main() -> Result<(), Box<dyn Error>> {
//     ui::run_ui()
// }

fn main() {
    println!("{:?}", config::get_author())
}
