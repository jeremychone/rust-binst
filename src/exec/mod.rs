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
	repo::{extract_opts_from_argc, BinRepo, BinRepoError},
	utils::{get_toml_value_as_string, UtilsError},
};

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
	Ok(bin_repo.install().await?)
}

#[tokio::main]
pub async fn exec_publish(argc: &ArgMatches) -> Result<(), ExecError> {
	let repo_path = argc.value_of("repo").ok_or(ExecError::NoRepo)?;
	let toml = read_to_string(CARGO_TOML)?;
	let toml: Value = toml::from_str(&toml)?;
	let bin_name = get_toml_value_as_string(&toml, &["package", "name"])?;

	let bin_repo = BinRepo::new(&bin_name, repo_path, extract_opts_from_argc(argc))?;

	Ok(bin_repo.publish().await?)
}

#[tokio::main]
pub async fn exec_update(argc: &ArgMatches) -> Result<(), ExecError> {
	let bin_name = argc.value_of("bin_name").ok_or(ExecError::NoBinName)?;
	let repo_path = match argc.value_of("repo") {
		Some(repo_path) => repo_path.to_owned(),
		None => {
			// get version from .binst/bin/...
			let version_dir = get_version_dir_from_symlink(bin_name)?;

			let install_toml_path = version_dir.join("install.toml");
			let toml = read_to_string(&install_toml_path)?;
			let toml: Value = toml::from_str(&toml)?;
			let repo = match get_toml_value_as_string(&toml, &["install", "repo"]) {
				Ok(repo) => repo,
				Err(_) => return Err(ExecError::NoRepoFoundInArgumentOrInInstallToml(install_toml_path.to_string_lossy().to_string())),
			};

			repo
		}
	};

	let installed_version = extract_version_from_binst_bin(bin_name)?;
	let installed_version = Version::parse(&installed_version)?;

	let repo = BinRepo::new(&bin_name, &repo_path, extract_opts_from_argc(argc))?;
	let origin_toml = repo.get_origin_info_toml_content().await?;

	let origin_toml: Value = toml::from_str(&origin_toml)?;
	let origin_version = get_toml_value_as_string(&origin_toml, &["stable", "version"])?;
	let origin_version = Version::parse(&origin_version)?;

	println!("Updating {} from repo {}", &bin_name, &repo_path);

	if origin_version > installed_version {
		println!("  Installing emote version {} ( > local version {})", origin_version, installed_version);
		repo.install().await?;
	} else {
		println!(
			"   No need to update {}, remote version {} <= local version {}",
			bin_name, origin_version, installed_version
		);
	}

	Ok(())
}

fn extract_version_from_binst_bin(bin_name: &str) -> Result<String, ExecError> {
	let version_dir = get_version_dir_from_symlink(bin_name)?;

	let version =
		version_dir
			.file_name()
			.and_then(|f| Some(f.to_string_lossy().to_string()))
			.and_then(|f| if f.starts_with("v") { Some((&f[1..]).to_owned()) } else { None });
	let version = version.ok_or(ExecError::NoVersionFromBinPath(version_dir.to_string_lossy().to_string()))?;

	Ok(version)
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
