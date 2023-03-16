// #![allow(unused)] // silence unused warnings while exploring (to comment out)

// -- Re-Exports
pub use crate::error::{Error, Result};

// -- Imports
use crate::cmd::cmd_exec;

// -- Sub-Modules
mod cmd;
mod error;
mod paths;
mod prelude;
mod repo;
mod utils;

fn main() -> Result<()> {
	match cmd_exec() {
		Ok(_) => println!("✔ All good and well"),
		Err(e) => {
			println!("Error:\n  {}", e)
		}
	};
	Ok(())
}
