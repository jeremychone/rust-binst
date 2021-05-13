mod aws_provider;
mod install;
mod publish;

use std::{
	fs::{create_dir_all, remove_file, File},
	io::Write,
	path::{Path, PathBuf},
	time::{SystemTime, UNIX_EPOCH},
};

use crate::{
	paths::{binst_bin_dir, binst_tmp_dir, os_target},
	utils::{sym_link, UtilsError},
};
use clap::ArgMatches;
use thiserror::Error;

enum Kind {
	// local path dir
	Local(String),
	// S3, only support via profile for now
	S3(S3Info),
	// http/https, only for install
	Http(String),
}

struct S3Info {
	bucket: String,
	base: String,
	profile: Option<String>,
}

pub struct BinRepo {
	bin_name: String,
	kind: Kind,
	repo_raw: String,
}

#[derive(Default)]
pub struct RepoOpts {
	profile: Option<String>,
}

#[derive(Error, Debug)]
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

	#[error("No or missing credentials in environment variable. Must have (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, and AWS_DEFAULT_REGION)")]
	S3CredMissingEnv,

	#[error("Profile {0} not found or missing credentials")]
	S3CredMissingProfile(String),

	#[error("http protocols not supported for publish")]
	HttpProtocolNotSupportedForPublish,

	#[error("Invalid S3 repo url {0}")]
	RepoInvalidS3(String),

	#[error("Invalid {0}")]
	InvalidInfoToml(String),

	#[error("Origin info.toml not found")]
	OriginInfoNotFound(String),

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

// repo builder function(s) and common methods
impl BinRepo {
	pub fn new(bin_name: &str, repo: &str, opts: RepoOpts) -> Result<Self, BinRepoError> {
		// FIXME: For now hardcode to jc-user pofile. Need to read from --profile
		let kind = parse_repo_uri(repo, opts)?;
		Ok(BinRepo {
			bin_name: bin_name.to_owned(),
			kind,
			repo_raw: repo.to_owned(),
		})
	}

	fn origin_bin_target_uri(&self) -> String {
		let target = os_target();
		format!("{}/{}", self.bin_name, target)
	}
}

fn parse_repo_uri(repo: &str, opts: RepoOpts) -> Result<Kind, BinRepoError> {
	let RepoOpts { profile } = opts;

	let kind = if repo.starts_with("s3://") {
		let repo_path = &repo[5..];
		let mut parts = repo_path.splitn(2, '/');
		let bucket = match parts.next() {
			Some(bucket) => {
				if bucket.len() == 0 {
					return Err(BinRepoError::RepoInvalidS3(repo.to_owned()));
				}
				bucket
			}
			None => return Err(BinRepoError::RepoInvalidS3(repo.to_owned())),
		}
		.to_owned();

		let base = match parts.next() {
			Some(base) => {
				if base.starts_with("/") {
					return Err(BinRepoError::RepoInvalidS3(repo.to_owned()));
				}
				base
			}
			None => "", // empty string for empty base path
		}
		.to_owned();

		Kind::S3(S3Info { bucket, base, profile })
	} else if repo.starts_with("http://") || repo.starts_with("https://") {
		Kind::Http(repo.to_owned())
	} else {
		Kind::Local(repo.to_owned())
	};

	Ok(kind)
}

pub fn extract_opts(argv: &ArgMatches) -> RepoOpts {
	let profile = argv.value_of("profile").and_then(|f| Some(f.to_owned()));
	RepoOpts { profile, ..Default::default() }
}

// region:    BinRepo path function helpers
fn make_bin_temp_dir(bin_name: &str) -> Result<PathBuf, BinRepoError> {
	let start = SystemTime::now().duration_since(UNIX_EPOCH).expect("time anomaly?").as_millis();

	let path = binst_tmp_dir(Some(&format!("{}-{}", bin_name, start)))?;
	Ok(path)
}

fn get_release_bin(name: &str) -> Result<PathBuf, BinRepoError> {
	// TODO: add support for Windows
	let bin_file = Path::new("./target/release").join(name);
	match bin_file.is_file() {
		true => Ok(bin_file),
		false => Err(BinRepoError::NoReleaseBinFile),
	}
}
// endregion: BinRepo path function helpers

// region:    Self/Install/Update helpers
pub fn create_bin_symlink(bin_name: &str, unpacked_bin: &PathBuf) -> Result<PathBuf, BinRepoError> {
	// make sure the .binst/bin/ directory exists
	let bin_dir = binst_bin_dir();
	if !bin_dir.is_dir() {
		create_dir_all(&bin_dir)?;
	}

	if !unpacked_bin.is_file() {
		return Err(BinRepoError::UnpackedBinFileNotFound(unpacked_bin.to_string_lossy().to_string()));
	}
	let bin_symlink_path = binst_bin_dir().join(bin_name);
	if bin_symlink_path.is_file() {
		remove_file(&bin_symlink_path)?;
	}
	sym_link(&unpacked_bin, &bin_symlink_path)?;
	Ok(bin_symlink_path)
}

pub fn create_install_toml(package_dir: &PathBuf, repo: &str, version: &str) -> Result<(), BinRepoError> {
	let install_content = get_install_toml(repo, &version);
	let install_path = package_dir.join("install.toml");
	File::create(&install_path)?.write_all(install_content.as_bytes())?;
	Ok(())
}

fn get_install_toml(repo: &str, version: &str) -> String {
	format!(
		r#"
[install]		
version = "{}"
repo = "{}"
"#,
		version, repo
	)
}

// endregion: Self/Install/Update helpers
