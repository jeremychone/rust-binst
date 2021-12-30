use thiserror::Error;

use crate::exec::ExecError;
use crate::repo::error::BinRepoError;

#[derive(Error, Debug)]
pub enum AppError {
	#[error("No Home Dir")]
	NoHomeDir,

	// Represents all other cases of `std::io::Error`.
	#[error(transparent)]
	IOError(#[from] std::io::Error),

	#[error(transparent)]
	InstallError(#[from] ExecError),

	#[error(transparent)]
	BinRepoError(#[from] BinRepoError),

	#[error(transparent)]
	ClapError(#[from] clap::Error),

	#[error("Cargo.toml has an invalid semver version {0}")]
	CargoInvalidVersion(String),
}
