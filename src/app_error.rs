use thiserror::Error;

#[allow(unused)] //
#[derive(Error, Debug)]
pub enum AppError {
	#[error("No Home Dir")]
	NoHomeDir,

	/// Path not safe to delete
	#[error("Path not safe to delete: {0}")]
	PathNotSafeToDelete(String),

	#[error("Cannot delete non node mobule dir")]
	CantDeleteNonNodeMobuleDir,

	/// Represents a failure to read from input.
	#[error("Read error")]
	ReadError { source: std::io::Error },

	#[error("Path Error")]
	PathNotExist(String),

	// Represents all other cases of `std::io::Error`.
	#[error(transparent)]
	IOError(#[from] std::io::Error),
}
