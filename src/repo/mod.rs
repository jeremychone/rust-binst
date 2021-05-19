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
use regex::Regex;
use semver::Version;
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

pub const MAIN_STREAM: &str = "main";

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

	fn origin_bin_target_uri(&self, stream_or_path: &str) -> String {
		let target = os_target();
		format!("{}/{}/{}", self.bin_name, target, stream_or_path)
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

pub fn extract_opts_from_argc(argv: &ArgMatches) -> RepoOpts {
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

pub fn extract_stream(version: &Version) -> String {
	if version.pre.len() > 0 {
		let pre = version.pre[0].to_string();
		let rx = Regex::new("[a-zA-Z-]+").unwrap(); // can't fail if it worked once
		let stream = rx.find(&pre).and_then(|m| Some(m.as_str())).unwrap_or("pre");
		let stream = if stream.ends_with("-") { &stream[..stream.len() - 1] } else { stream };

		stream.to_owned()
	} else {
		MAIN_STREAM.to_string()
	}
}

// endregion: BinRepo path function helpers

// region:    Self/Install/Update helpers

//// Returns version path part.
pub fn get_version_part(version: &Version) -> String {
	format!("{}", version.to_string())
}

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

pub fn create_install_toml(package_dir: &PathBuf, repo: &str, stream: &str, version: &Version) -> Result<(), BinRepoError> {
	let install_content = create_install_toml_content(repo, stream, version);
	let install_path = package_dir.join("install.toml");
	File::create(&install_path)?.write_all(install_content.as_bytes())?;
	Ok(())
}

fn create_install_toml_content(repo: &str, stream: &str, version: &Version) -> String {
	format!(
		r#"[install]		
repo = "{}"
stream = "{}"
version = "{}"
"#,
		repo, stream, version
	)
}

// endregion: Self/Install/Update helpers

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_extract_stream() {
		fn run(v: &str) -> String {
			extract_stream(&Version::parse(v).unwrap())
		}

		assert_eq!("main", run("0.1.3"));
		assert_eq!("main", run("0.1.0"));
		assert_eq!("rc", run("0.1.3-rc"));
		assert_eq!("rc", run("0.1.3-rc-1"));
		assert_eq!("rc-big", run("0.1.3-rc-big-1"));
		assert_eq!("beta", run("0.1.3-beta.2"));
		assert_eq!("beta", run("0.1.3-beta2"));
		assert_eq!("big-beta", run("0.1.3-big-beta2"));
		assert_eq!("pre", run("0.1.3-123"));
	}
}
