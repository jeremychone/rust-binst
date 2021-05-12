// #![allow(unused)] // silence unused warnings while exploring (to comment out)

mod app_error;
mod argv;
mod paths;
mod setup;

use app_error::AppError;

use argv::cmd_app;
use setup::exec_setup;

fn main() -> Result<(), AppError> {
	let cmd = cmd_app().get_matches();

	if let Some(setup_cmd) = cmd.subcommand_matches("setup") {
		println!("Will perform setup");
		return exec_setup(setup_cmd);
	} else {
		println!("will do nothing");
	}

	Ok(())
}
