use std::{fs::create_dir_all, fs::write};

use clap::ArgMatches;

use crate::app_error::AppError;
use crate::paths::*;

#[allow(unused)] //
pub fn exec_setup(argv: &ArgMatches) -> Result<(), AppError> {
	println!("setup mod exec");

	let home_dir = dirs::home_dir().expect("No home dir");

	if !home_dir.is_dir() {
		return Err(AppError::NoHomeDir);
	}

	// create the binst as needed
	let binst_dir = binst_dir();
	if !binst_dir.is_dir() {
		create_dir_all(binst_dir.as_path())?;
		println!("binst home dir created {}", binst_dir.to_str().unwrap())
	}

	// create the ~/.binst/env as needed
	let env_path = binst_env();
	if !env_path.is_file() {
		let env = include_bytes!("../assets/env");
		let env: String = String::from_utf8_lossy(env).into();
		write(&env_path, env)?;
		println!("binst {} file created", env_path.to_str().unwrap());
	}

	println!(
		"
IMPORTANT: Add '~/.binst/bin/' to your PATH environment. You can follow one of the following options:
  1) Set it manually in your shell boostrap file
  2) Add 'source ~/.binst/env' in your shell boostrap env file (.e.g. ~/.zshenv on mac)
"
	);

	Ok(())
}
