use crate::exec::ExecError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("No Home Dir")]
	NoHomeDir,

	// Represents all other cases of `std::io::Error`.
	#[error(transparent)]
	IO(#[from] std::io::Error),

	#[error(transparent)]
	Install(#[from] ExecError),

	#[error(transparent)]
	BinRepo(#[from] crate::repo::Error),

	#[error(transparent)]
	Clap(#[from] clap::Error),

	#[error("Cargo.toml has an invalid semver version {0}")]
	CargoInvalidVersion(String),
}
