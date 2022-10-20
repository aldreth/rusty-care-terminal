use std::error::Error;

mod git;
mod ui;

fn main() -> Result<(), Box<dyn Error>> {
    ui::run_ui()
}
