#![allow(unused)] // silence unused warnings while exploring (to comment out)

mod app_error;
mod argc;
mod exec;
mod paths;
mod repo;
mod utils;

use app_error::AppError;
use exec::{exec_install, exec_publish, exec_update};

use crate::exec::setup::exec_setup;
use argc::cmd_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	match run() {
		Ok(_) => println!("  ✔ All good and well"),
		Err(e) => {
			println!("ERROR CAUGHT - {:?}", e)
		}
	};
	Ok(())
}
fn run() -> Result<(), AppError> {
	let cmd = cmd_app().get_matches();

	match cmd.subcommand() {
		("self", Some(_)) => exec_setup()?,
		("publish", Some(sub_cmd)) => exec_publish(sub_cmd)?,
		("install", Some(sub_cmd)) => exec_install(sub_cmd)?,
		("update", Some(sub_cmd)) => exec_update(sub_cmd)?,
		_ => {
			// needs cmd_app version as the orginal got consumed by get_matches
			cmd_app().print_long_help()?;
			println!("\n");
		}
	}

	Ok(())
}
