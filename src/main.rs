// #![allow(unused)] // silence unused warnings while exploring (to comment out)

mod app_error;
mod exec;
mod paths;
mod repo;
mod utils;

use app_error::AppError;
use exec::{exec_install, exec_publish, exec_update};

use crate::exec::setup::exec_setup;
use exec::argc::cmd_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	match run() {
		Ok(_) => println!("✔ All good and well"),
		Err(e) => {
			println!("Error:\n  {}", e)
		}
	};
	Ok(())
}
fn run() -> Result<(), AppError> {
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
