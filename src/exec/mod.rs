pub mod setup;

use std::{
	fs::{self, read_to_string},
	path::PathBuf,
};

use clap::ArgMatches;
use semver::{SemVerError, Version};
use thiserror::Error;
use toml::Value;

use crate::{
	paths::binst_bin_dir,
	repo::{extract_opts_from_argc, BinRepo, BinRepoError, MAIN_STREAM},
	utils::{clean_path, get_toml_value_as_string, UtilsError},
};

struct InstalledBinInfo {
	stream: String,
	version: Version,
	repo_raw: String,
}

pub const CARGO_TOML: &str = "Cargo.toml";

#[derive(Error, Debug)]
pub enum ExecError {
	#[error("Must have a -r repo_url for now (later can be found from later)")]
	NoRepo,

	#[error("Install command must have a binary name in argument")]
	NoBinName,

	#[error("No repo in argument or in install.toml {0}")]
	NoRepoFoundInArgumentOrInInstallToml(String),

	#[error("Cannot find package dir for bin {0}")]
	CannotFindBinPackageDir(String),

	#[error("Version could not be found from bin path {0}")]
	NoVersionFromBinPath(String),

	#[error(transparent)]
	BinRepoError(#[from] BinRepoError),

	#[error(transparent)]
	IOError(#[from] std::io::Error),

	#[error(transparent)]
	TomlError(#[from] toml::de::Error),

	#[error(transparent)]
	SemVerError(#[from] SemVerError),

	#[error(transparent)]
	UtilsError(#[from] UtilsError),
}

#[tokio::main]
pub async fn exec_install(argc: &ArgMatches) -> Result<(), ExecError> {
	let bin_name = argc.value_of("bin_name").ok_or(ExecError::NoBinName)?;
	let repo_path = argc.value_of("repo").ok_or(ExecError::NoRepo)?;
	let bin_repo = BinRepo::new(bin_name, repo_path, extract_opts_from_argc(argc))?;

	let stream = argc.value_of("stream").unwrap_or(MAIN_STREAM).to_owned();
	Ok(bin_repo.install(stream).await?)
}

#[tokio::main]
pub async fn exec_publish(argc: &ArgMatches) -> Result<(), ExecError> {
	let repo_path = argc.value_of("repo").ok_or(ExecError::NoRepo)?;
	let toml = read_to_string(CARGO_TOML)?;
	let toml: Value = toml::from_str(&toml)?;
	let bin_name = get_toml_value_as_string(&toml, &["package", "name"])?;

	let bin_repo = BinRepo::new(&bin_name, repo_path, extract_opts_from_argc(argc))?;
	let at_path = argc.value_of("path").map(clean_path);

	Ok(bin_repo.publish(at_path).await?)
}

#[tokio::main]
pub async fn exec_update(argc: &ArgMatches) -> Result<(), ExecError> {
	let bin_name = argc.value_of("bin_name").ok_or(ExecError::NoBinName)?;

	let InstalledBinInfo {
		stream,
		repo_raw,
		version: installed_version,
	} = extract_installed_bin_info(bin_name)?;

	let repo = BinRepo::new(&bin_name, &repo_raw, extract_opts_from_argc(argc))?;
	let origin_toml = repo.get_origin_latest_toml_content(&stream).await?;

	let origin_toml: Value = toml::from_str(&origin_toml)?;
	let origin_version = get_toml_value_as_string(&origin_toml, &["stable", "version"])?;
	let origin_version = Version::parse(&origin_version)?;

	println!("Updating {} from repo {}", &bin_name, &repo_raw);

	if origin_version > installed_version {
		println!("  Installing emote version {} ( > local version {})", origin_version, installed_version);
		repo.install(stream).await?;
	} else {
		println!(
			"   No need to update {}, remote version {} <= local version {}",
			bin_name, origin_version, installed_version
		);
	}

	Ok(())
}

fn extract_installed_bin_info(bin_name: &str) -> Result<InstalledBinInfo, ExecError> {
	let version_dir = get_version_dir_from_symlink(bin_name)?;

	// extract the version from the dir path
	let version = version_dir
		.file_name()
		.and_then(|f| Some(f.to_string_lossy().to_string()))
		.and_then(|f| match Version::parse(&f) {
			Ok(version) => Some(version),
			Err(_) => None,
		});
	let version = version.ok_or(ExecError::NoVersionFromBinPath(version_dir.to_string_lossy().to_string()))?;

	let install_toml_path = version_dir.join("install.toml");
	let install_toml = read_to_string(&install_toml_path)?;
	let install_toml: Value = toml::from_str(&install_toml)?;

	// get the stream
	let stream = match get_toml_value_as_string(&install_toml, &["install", "stream"]) {
		Ok(stream) => stream,
		Err(_) => MAIN_STREAM.to_string(),
	};

	// get the repo
	let repo_raw = match get_toml_value_as_string(&install_toml, &["install", "repo"]) {
		Ok(repo) => repo,
		Err(_) => return Err(ExecError::NoRepoFoundInArgumentOrInInstallToml(install_toml_path.to_string_lossy().to_string())),
	};

	Ok(InstalledBinInfo { version, stream, repo_raw })
}

fn get_version_dir_from_symlink(bin_name: &str) -> Result<PathBuf, ExecError> {
	let bin_dir = binst_bin_dir();
	let bin_symlink = bin_dir.join(bin_name);
	let path = fs::canonicalize(&bin_symlink)?;
	let package = path.parent().and_then(|f| f.parent());

	match package {
		Some(path) => Ok(path.to_path_buf()),
		None => Err(ExecError::CannotFindBinPackageDir(bin_symlink.to_string_lossy().to_string())),
	}
}
