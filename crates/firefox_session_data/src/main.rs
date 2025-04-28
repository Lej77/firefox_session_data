#![warn(clippy::all)]
use firefox_session_data as lib;

fn main() -> lib::Result<()> {
    lib::run()
}
