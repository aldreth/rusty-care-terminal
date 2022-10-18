mod git;

fn main() {
    match git::get_commits() {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
