use crate::utils::UtilsError;

#[derive(thiserror::Error, Debug)]
pub enum BinRepoError {
	#[error(
		"Cannot access bucket bucket {0} with credential from {1} 
   Check you have the right profile in .aws/config and credentials"
	)]
	RepoS3BucketNotAccessible(String, String),

	#[error(
		"Aws crendials not found in environment or profile. 
  Make sure to set the (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, and AWS_DEFAULT_REGION) 
  or add a --profile your_profile"
	)]
	S3CredMissingInEnvOrProfile,

	#[error("Profile {0} not found or missing credentials")]
	S3CredMissingProfile(String),

	#[error("http protocols not supported for publish")]
	HttpProtocolNotSupportedForPublish,

	#[error("Invalid S3 repo url {0}")]
	RepoInvalidS3(String),

	#[error("Invalid {0}")]
	InvalidInfoToml(String),

	#[error("Invalid version from origin latest.toml")]
	InvalidVersionFromOrigin,

	#[error("Origin latest.toml not found. Might be wrong stream or package name. Not found {0}")]
	OriginLatestNotFound(String),

	#[error("The package .tar.gz file was not found at {0}")]
	OriginTarGzNotFound(String),

	#[error("The unpacked binary file not found at {0}")]
	UnpackedBinFileNotFound(String),

	#[error("No bin file found unser target/release. Make sure to do a cargo build --release")]
	NoReleaseBinFile,

	#[error(transparent)]
	IOError(#[from] std::io::Error),

	#[error(transparent)]
	TomlError(#[from] toml::de::Error),

	#[error(transparent)]
	UtilsError(#[from] UtilsError),

	#[error(transparent)]
	ReqwestError(#[from] reqwest::Error),

	#[error(transparent)]
	AnyhowError(#[from] anyhow::Error),
}
