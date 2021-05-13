pub mod setup;

use std::fs::read_to_string;

use clap::ArgMatches;
use thiserror::Error;
use toml::Value;

use crate::{
	repo::{extract_opts, BinRepo, BinRepoError},
	utils::{get_toml_value_as_string, UtilsError},
};

pub const CARGO_TOML: &str = "Cargo.toml";

#[derive(Error, Debug)]
pub enum ExecError {
	#[error("Must have a -r repo_url for now (later can be found from later)")]
	NoRepo,

	#[error("Install command must have a binary name in argugment")]
	NoBinName,

	#[error(transparent)]
	BinRepoError(#[from] BinRepoError),

	#[error(transparent)]
	IOError(#[from] std::io::Error),

	#[error(transparent)]
	TomlError(#[from] toml::de::Error),

	#[error(transparent)]
	UtilsError(#[from] UtilsError),
}

pub fn exec_install(argc: &ArgMatches) -> Result<(), ExecError> {
	let bin_name = argc.value_of("bin_name").ok_or(ExecError::NoBinName)?;
	let repo_path = argc.value_of("repo").ok_or(ExecError::NoRepo)?;
	let bin_repo = BinRepo::new(bin_name, repo_path, extract_opts(argc))?;
	Ok(bin_repo.install()?)
}

pub fn exec_publish(argc: &ArgMatches) -> Result<(), ExecError> {
	let repo_path = argc.value_of("repo").ok_or(ExecError::NoRepo)?;
	let toml = read_to_string(CARGO_TOML)?;
	let toml: Value = toml::from_str(&toml)?;
	let bin_name = get_toml_value_as_string(&toml, &["package", "name"])?;

	let bin_repo = BinRepo::new(&bin_name, repo_path, extract_opts(argc))?;

	Ok(bin_repo.publish()?)
}
