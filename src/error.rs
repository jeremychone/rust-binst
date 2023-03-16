pub type Result<T> = core::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	// Represents all other cases of `std::io::Error`.
	#[error(transparent)]
	IO(#[from] std::io::Error),

	#[error(transparent)]
	Install(#[from] crate::cmd::error::Error),

	#[error(transparent)]
	BinRepo(#[from] crate::repo::Error),

	#[error(transparent)]
	Clap(#[from] clap::Error),
}
