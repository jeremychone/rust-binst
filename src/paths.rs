use std::path::PathBuf;

use dirs::home_dir;

const BINST_DIR: &str = ".binst";
const BINST_CFG: &str = "config";
const BINST_ENV: &str = "env";
// const BINST_CRD: &str = "credentials";
// const BINST_BIN_DIR: &str = "bin";
// const BINST_PKG_DIR: &str = "packages";

pub fn binst_dir() -> PathBuf {
	let home_dir = home_dir().expect("No home dir");
	home_dir.join(BINST_DIR)
}

pub fn binst_env() -> PathBuf {
	binst_dir().join(BINST_ENV)
}

#[allow(unused)] //
pub fn binst_config() -> PathBuf {
	binst_dir().join(BINST_CFG)
}

#[allow(unused)] //
pub fn binst_bin_dir() -> PathBuf {
	binst_dir().join(BINST_DIR)
}
