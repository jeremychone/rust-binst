pub type Result<T> = core::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("No Home Dir")]
	NoHomeDir,

	#[error("Install command must have a binary name in argument")]
	NoBinName,

	#[error("No repo in argument or in install.toml {0}")]
	NoRepoFoundInArgumentOrInInstallToml(String),

	#[error("Cannot find package dir for bin {0}")]
	CannotFindBinPackageDir(String),

	#[error("Version could not be found from bin path {0}")]
	NoVersionFromBinPath(String),

	#[error("Cargo.toml has an invalid semver version {0}")]
	CargoInvalidVersion(String),

	#[error(transparent)]
	BinRepo(#[from] crate::repo::Error),

	#[error(transparent)]
	IO(#[from] std::io::Error),

	#[error(transparent)]
	Toml(#[from] toml::de::Error),

	#[error(transparent)]
	SemVer(#[from] semver::Error),

	#[error(transparent)]
	Utils(#[from] crate::utils::Error),
}
