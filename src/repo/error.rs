use aws_config::retry::ProvideErrorKind;
use aws_sdk_s3::types::SdkError;
use aws_smithy_http::result::CreateUnhandledError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	// region:    --- S3
	#[error(
		"Cannot access bucket bucket {0} with credential from {1} 
   Check you have the right profile in .aws/config and credentials"
	)]
	RepoS3BucketNotAccessible(String, String),

	#[error(
		"AWS crendials not found in environment or profile. 
  Make sure to set the AWS Credential environment variables
    - BINST_REPO_AWS_KEY_ID, BINST_REPO_AWS_KEY_SECRET, and BINST_REPO_AWS_REGION.
    - Or AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, and AWS_DEFAULT_REGION.
    - Or add a `--profile your_profile` to use your aws config profile."
	)]
	S3CredMissingInEnvOrProfile,

	#[error("AWS Environment variables must have REGION or ENDPOINT (both can't be None)")]
	S3CredMustHaveRegionOrEndpoint,

	#[error("Profile {0} not found or missing credentials")]
	S3CredMissingProfile(String),

	#[error("Invalid EndPoint '{0}'")]
	InvalidEndPoint(String),

	#[error("No environment variable '{0}'")]
	NoCredentialEnv(String),

	#[error("http protocols not supported for publish")]
	HttpProtocolNotSupportedForPublish,

	#[error("Credential profile config key {0} not found")]
	NoCredentialConfig(String),

	#[error("No credentials found for profile {0}.")]
	NoCredentialsForProfile(String),

	#[error("Invalid S3 repo url {0}")]
	RepoInvalidS3(String),

	/// Simplified AWS Error message with code.
	#[error("AWS Error. Code: {0}")]
	AwsServiceError(String), // Code
	// endregion: --- S3

	// region:    --- Others
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
	UtilsError(#[from] crate::utils::Error),

	#[error(transparent)]
	ReqwestError(#[from] reqwest::Error),

	#[error(transparent)]
	ByteStream(#[from] aws_smithy_http::byte_stream::error::Error),
	// endregion: --- Others
}

/// Generic for AWS Error simple reporting
impl<E> From<SdkError<E>> for Error
where
	E: std::error::Error + Send + Sync + CreateUnhandledError + ProvideErrorKind + 'static,
{
	fn from(val: SdkError<E>) -> Self {
		let se = val.into_service_error();
		let code = se.code().unwrap_or_default().to_string();
		Error::AwsServiceError(code)
		// Note: Unforuntately, it seems there is no trait for the .message(), so,
		//       cannot get it when using the generic way. Can be implemented for each E type.
		// Note: String format of the se gives "Unhandled error" which is confusing.
	}
}
