use std::{fs::create_dir_all, path::PathBuf};

use dirs::home_dir;
use platform_info::{PlatformInfo, Uname};

const BINST_DIR: &str = ".binst";
// const BINST_CFG: &str = "config";
const BINST_ENV: &str = "env";
// const BINST_CRD: &str = "credentials";
// const BINST_BIN_DIR: &str = "bin";
// const BINST_PKG_DIR: &str = "packages";

pub fn binst_dir() -> PathBuf {
	let home_dir = home_dir().expect("No home dir");
	home_dir.join(BINST_DIR)
}

pub fn binst_tmp_dir(folder: Option<&str>) -> Result<PathBuf, std::io::Error> {
	let tmp_path = match folder {
		Some(folder) => binst_dir().join("tmp").join(folder),
		None => binst_dir().join("tmp"),
	};

	if !tmp_path.is_dir() {
		create_dir_all(&tmp_path)?;
	}
	Ok(tmp_path)
}

pub fn binst_package_bin_dir(bin_name: &str, version: &str) -> Result<PathBuf, std::io::Error> {
	let path = binst_dir().join("packages").join(bin_name).join(format!("v{}", version));
	if !path.is_dir() {
		create_dir_all(&path)?;
	}
	Ok(path)
}

pub fn binst_env() -> PathBuf {
	binst_dir().join(BINST_ENV)
}

// pub fn binst_config() -> PathBuf {
// 	binst_dir().join(BINST_CFG)
// }

pub fn binst_bin_dir() -> PathBuf {
	binst_dir().join("bin")
}

pub fn os_target() -> String {
	let platform = PlatformInfo::new().unwrap();
	let machine = platform.machine().to_string();
	let sysname = platform.sysname().to_string().to_lowercase();
	let mut target = String::new();
	target.push_str(&machine);
	if sysname.contains("darwin") {
		target.push_str("-apple-darwin");
	} else if sysname.contains("linux") {
		// FIXME: right now assumg gnu, but might not be safe
		target.push_str("-unknown-linux-gnu");
	} else {
		// TODO: add support for Windows
		target.push_str("-not-supported");
	}

	target
}
