// -- Re-export
pub use self::error::{Error, Result};
pub use repo_info::*;

// -- Imports
use crate::paths::{binst_bin_dir, binst_tmp_dir, os_target};
use crate::utils::sym_link;
use clap::ArgMatches;
use regex::Regex;
use semver::Version;
use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// -- Sub-Modules
mod error;
mod install;
mod publish;
mod repo_info;
mod s3w;

// --- Consts
pub const BINST_REPO_URL: &str = "https://repo.binst.io/";
pub const BINST_REPO_BUCKET: &str = "binst-repo";
pub const BINST_REPO_AWS_PROFILE: &str = "binst-repo-user";
// main stream
pub const MAIN_STREAM: &str = "main";

#[derive(Debug)]
pub struct S3Info {
	url: String,
	bucket: String,
	base: String,
	profile: Option<String>,
}

impl S3Info {
	pub fn from_s3_url(s3_url: &str, profile: Option<&str>) -> Result<S3Info> {
		let repo_path = &s3_url[5..];
		let mut parts = repo_path.splitn(2, '/');
		let bucket = match parts.next() {
			Some(bucket) => {
				if bucket.is_empty() {
					return Err(Error::RepoInvalidS3(s3_url.to_owned()));
				}
				bucket
			}
			None => return Err(Error::RepoInvalidS3(s3_url.to_owned())),
		}
		.to_owned();

		let base = match parts.next() {
			Some(base) => {
				if base.starts_with('/') {
					return Err(Error::RepoInvalidS3(s3_url.to_owned()));
				}
				base
			}
			None => "", // empty string for empty base path
		}
		.to_owned();

		let profile = profile.map(|v| v.to_owned());
		let url = s3_url.to_string();

		Ok(S3Info {
			url,
			bucket,
			base,
			profile,
		})
	}
}

#[derive(Debug)]
pub struct BinRepo {
	pub bin_name: String,
	pub install_repo: RepoInfo,
	pub publish_repo: RepoInfo,
	pub target: Option<String>,
}

/// Constructor
impl BinRepo {
	pub fn new(bin_name: &str, argc: &ArgMatches, publish: bool) -> Result<Self> {
		let bin_name = bin_name.to_string();

		let target = if publish {
			argc.get_one::<String>("target").map(|target| target.to_string())
		} else {
			None
		};

		// build the RepoInfo
		let argc_profile = argc.get_one::<String>("profile").map(|s| s.as_str());
		let argc_repo = argc.get_one::<String>("repo");
		let (install_repo, publish_repo) = if let Some(repo) = argc_repo {
			let install_repo = RepoInfo::from_repo_string(repo, argc_profile)?;
			let publish_repo = RepoInfo::from_repo_string(repo, argc_profile)?;
			(install_repo, publish_repo)
		} else {
			(RepoInfo::binst_install_repo(), RepoInfo::binst_publish_repo())
		};

		Ok(BinRepo {
			bin_name,
			install_repo,
			publish_repo,
			target,
		})
	}
}

/// Public functions
impl BinRepo {
	pub fn origin_bin_target_uri(&self, stream_or_path: &str) -> String {
		let target = self.target.as_ref().map(|s| s.to_string()).unwrap_or_else(os_target);
		format!("{}/{}/{}", self.bin_name, target, stream_or_path)
	}

	pub fn get_origin_url(&self, stream_or_path: &str, version: &Version) -> Result<String> {
		let url = match &self.install_repo {
			RepoInfo::Local(local_repo_origin) => self
				.get_origin_local_path(local_repo_origin, stream_or_path, version)?
				.to_string_lossy()
				.to_string(),
			RepoInfo::S3(s3_info) => self.get_origin_s3_url(s3_info, stream_or_path, version)?,
			RepoInfo::Http(base_url) => self.get_origin_http_url(base_url, stream_or_path, version)?,
		};

		Ok(url)
	}
}

// region:    BinRepo path function helpers
fn make_bin_temp_dir(bin_name: &str) -> Result<PathBuf> {
	let start = SystemTime::now().duration_since(UNIX_EPOCH).expect("time anomaly?").as_millis();

	let path = binst_tmp_dir(Some(&format!("{}-{}", bin_name, start)))?;
	Ok(path)
}

fn get_release_bin(name: &str, target: &Option<String>) -> Result<PathBuf> {
	// Note this is to support cross compilation (x86_64-apple-darwin on arm64)
	let bin_file = if let Some(target) = target {
		Path::new("./target").join(target).join("release").join(name)
	} else {
		Path::new("./target/release").join(name)
	};

	match bin_file.is_file() {
		true => Ok(bin_file),
		false => Err(Error::NoReleaseBinFile),
	}
}

pub fn extract_stream(version: &Version) -> String {
	if !version.pre.is_empty() {
		let pre = version.pre.as_str();
		let rx = Regex::new("[a-zA-Z-]+").unwrap(); // can't fail if it worked once
		let stream = rx.find(pre).map(|m| m.as_str()).unwrap_or("pre");
		// let stream = stream.strip_suffix('-');
		let stream = stream.strip_suffix('-').unwrap_or(stream);
		stream.to_string()
	} else {
		MAIN_STREAM.to_string()
	}
}

// endregion: BinRepo path function helpers

// region:    Self/Install/Update helpers

//// Returns version path part.
pub fn get_version_part(version: &Version) -> String {
	version.to_string()
}

pub fn create_bin_symlink(bin_name: &str, unpacked_bin: &Path) -> Result<PathBuf> {
	// make sure the .binst/bin/ directory exists
	let bin_dir = binst_bin_dir();
	if !bin_dir.is_dir() {
		create_dir_all(&bin_dir)?;
	}

	if !unpacked_bin.is_file() {
		return Err(Error::UnpackedBinFileNotFound(
			unpacked_bin.to_string_lossy().to_string(),
		));
	}
	let bin_symlink_path = binst_bin_dir().join(bin_name);
	if bin_symlink_path.is_file() {
		remove_file(&bin_symlink_path)?;
	}
	sym_link(unpacked_bin, &bin_symlink_path)?;
	Ok(bin_symlink_path)
}

pub fn create_install_toml(package_dir: &Path, repo: &str, stream: &str, version: &Version) -> Result<()> {
	let install_content = create_install_toml_content(repo, stream, version);
	let install_path = package_dir.join("install.toml");
	File::create(install_path)?.write_all(install_content.as_bytes())?;
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
