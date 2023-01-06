// #![allow(unused)] // silence unused warnings while exploring (to comment out)

mod error;
mod exec;
mod paths;
mod prelude;
mod repo;
mod utils;

use crate::exec::setup::exec_setup;
use exec::argc::cmd_app;
use exec::{exec_install, exec_publish, exec_update};

// Re-export the crate Error.
pub use crate::error::Error;
// Alias Result to be the crate Result.
pub type Result<T> = core::result::Result<T, Error>;

fn main() -> Result<()> {
	match run() {
		Ok(_) => println!("✔ All good and well"),
		Err(e) => {
			println!("Error:\n  {}", e)
		}
	};
	Ok(())
}

fn run() -> Result<()> {
	let cmd = cmd_app().get_matches();

	match cmd.subcommand() {
		Some(("self", _)) => exec_setup()?,
		Some(("publish", sub_cmd)) => exec_publish(sub_cmd)?,
		Some(("install", sub_cmd)) => exec_install(sub_cmd)?,
		Some(("update", sub_cmd)) => exec_update(sub_cmd)?,
		_ => {
			// needs cmd_app version as the orginal got consumed by get_matches
			cmd_app().print_long_help()?;
			println!("\n");
		}
	}

	Ok(())
}
