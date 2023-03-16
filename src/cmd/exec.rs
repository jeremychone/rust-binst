use crate::cmd::clap_cmd::clap_cmd;
use crate::cmd::setup::exec_setup;
use crate::cmd::{Error, InstalledBinInfo, Result, CARGO_TOML};
use crate::paths::binst_bin_dir;
use crate::repo::{BinRepo, MAIN_STREAM};
use crate::utils::{clean_path, get_toml_value_as_string};
use clap::ArgMatches;
use semver::Version;
use std::fs;
use std::path::PathBuf;
use toml::Value;

// region:    --- CMD Executor
pub fn cmd_exec() -> Result<()> {
	let cmd = clap_cmd().get_matches();

	match cmd.subcommand() {
		Some(("self", _)) => exec_setup()?,
		Some(("publish", sub_cmd)) => exec_publish(sub_cmd)?,
		Some(("install", sub_cmd)) => exec_install(sub_cmd)?,
		Some(("update", sub_cmd)) => exec_update(sub_cmd)?,
		Some(("info", sub_cmd)) => exec_info(sub_cmd)?,
		_ => {
			// needs cmd_app version as the orginal got consumed by get_matches
			clap_cmd().print_long_help()?;
			println!("\n");
		}
	}

	Ok(())
}
// endregion: --- CMD Executor

// region:    --- Exec Functions

#[tokio::main]
pub async fn exec_install(argm: &ArgMatches) -> Result<()> {
	let bin_name = argm.get_one::<String>("bin_name").ok_or(Error::NoBinName)?;
	let bin_repo = BinRepo::new(bin_name, argm, false)?;

	let stream = argm
		.get_one::<String>("stream")
		.map(|s| s.to_string())
		.unwrap_or_else(|| MAIN_STREAM.to_string());
	bin_repo.install(stream).await?;
	Ok(())
}

#[tokio::main]
pub async fn exec_publish(argm: &ArgMatches) -> Result<()> {
	let toml = fs::read_to_string(CARGO_TOML)?;
	let toml: Value = toml::from_str(&toml)?;
	let bin_name = get_toml_value_as_string(&toml, &["package", "name"])?;

	let bin_repo = BinRepo::new(&bin_name, argm, true)?;
	let at_path = argm.get_one::<String>("path").map(clean_path);

	Ok(bin_repo.publish(at_path).await?)
}

#[tokio::main]
pub async fn exec_update(argm: &ArgMatches) -> Result<()> {
	let bin_name = argm.get_one::<String>("bin_name").ok_or(Error::NoBinName)?;

	let InstalledBinInfo {
		stream,
		repo_raw,
		version: installed_version,
	} = extract_installed_bin_info(bin_name)?;

	let repo = BinRepo::new(bin_name, argm, false)?;
	let origin_toml = repo.get_origin_latest_toml_content(&stream).await?;

	let origin_toml: Value = toml::from_str(&origin_toml)?;
	let origin_version = get_toml_value_as_string(&origin_toml, &["latest", "version"])?;
	let origin_version = Version::parse(&origin_version)?;

	println!("Updating {} from repo {}", &bin_name, &repo_raw);

	if origin_version > installed_version {
		println!(
			"  Installing remote version {} ( > local version {})",
			origin_version, installed_version
		);
		repo.install(stream).await?;
	} else {
		println!(
			"   No need to update {}, remote version {} <= local version {}",
			bin_name, origin_version, installed_version
		);
	}

	Ok(())
}

#[tokio::main]
pub async fn exec_info(argm: &ArgMatches) -> Result<()> {
	let stream = MAIN_STREAM;

	let bin_name = argm.get_one::<String>("bin_name").ok_or(Error::NoBinName)?;
	let bin_repo = BinRepo::new(bin_name, argm, false)?;

	let version = bin_repo.get_origin_latest_version(stream).await?;
	let url = bin_repo.get_origin_url(stream, &version)?;

	println!(
		r#"Info for binary: {bin_name}
 Latest Version: {version}
     Latest URL: {url}
	"#,
	);

	Ok(())
}

// endregion: --- Exec Functions

// region:    --- Utils

fn extract_installed_bin_info(bin_name: &str) -> Result<InstalledBinInfo> {
	let version_dir = get_version_dir_from_symlink(bin_name)?;

	// extract the version from the dir path
	let version =
		version_dir
			.file_name()
			.map(|f| f.to_string_lossy().to_string())
			.and_then(|f| match Version::parse(&f) {
				Ok(version) => Some(version),
				Err(_) => None,
			});
	let version = version.ok_or(Error::NoVersionFromBinPath(version_dir.to_string_lossy().to_string()))?;

	let install_toml_path = version_dir.join("install.toml");
	let install_toml = fs::read_to_string(&install_toml_path)?;
	let install_toml: Value = toml::from_str(&install_toml)?;

	// get the stream
	let stream = match get_toml_value_as_string(&install_toml, &["install", "stream"]) {
		Ok(stream) => stream,
		Err(_) => MAIN_STREAM.to_string(),
	};

	// get the repo
	let repo_raw = match get_toml_value_as_string(&install_toml, &["install", "repo"]) {
		Ok(repo) => repo,
		Err(_) => {
			return Err(Error::NoRepoFoundInArgumentOrInInstallToml(
				install_toml_path.to_string_lossy().to_string(),
			))
		}
	};

	Ok(InstalledBinInfo {
		version,
		stream,
		repo_raw,
	})
}

fn get_version_dir_from_symlink(bin_name: &str) -> Result<PathBuf> {
	let bin_dir = binst_bin_dir();
	let bin_symlink = bin_dir.join(bin_name);
	let path = fs::canonicalize(&bin_symlink)?;
	let package = path.parent().and_then(|f| f.parent());

	match package {
		Some(path) => Ok(path.to_path_buf()),
		None => Err(Error::CannotFindBinPackageDir(
			bin_symlink.to_string_lossy().to_string(),
		)),
	}
}

// endregion: --- Utils
