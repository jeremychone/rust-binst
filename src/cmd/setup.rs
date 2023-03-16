use crate::cmd::clap_cmd::version;
use crate::cmd::{Error, Result};
use crate::repo::create_bin_symlink;
use crate::{paths::*, repo::create_install_toml};
use semver::Version;
use std::fs::{copy, create_dir_all, write};

const SELF_REPO: &str = "https://binst.io/self";
const SELF_STREAM: &str = "main";

pub fn exec_setup() -> Result<()> {
	let home_dir = dirs::home_dir().expect("No home dir");
	let bin_name = "binst";

	if !home_dir.is_dir() {
		Err(Error::NoHomeDir)?;
	}

	// create the binst as needed
	let binst_dir = binst_dir();
	println!("Self installing binst under {}", binst_dir.to_string_lossy());
	if !binst_dir.is_dir() {
		create_dir_all(binst_dir.as_path())?;
	}

	// create the ~/.binst/env as needed
	let env_path = binst_env();
	if !env_path.is_file() {
		let env = include_bytes!("../assets/env");
		let env: String = String::from_utf8_lossy(env).into();
		write(&env_path, env)?;
	}

	// copy the binary version to its packages
	let version = version();
	let version = match Version::parse(&version) {
		Ok(version) => version,
		Err(_) => return Err(Error::CargoInvalidVersion(version.to_owned())),
	};

	let package_dir = binst_package_bin_dir(bin_name, &version)?;
	let unpacked_dir = package_dir.join("unpacked");
	if !unpacked_dir.is_dir() {
		create_dir_all(&unpacked_dir)?;
	}
	let exec_path = std::env::current_exe()?;
	let binst_path = unpacked_dir.join(bin_name);
	copy(&exec_path, &binst_path)?;

	create_install_toml(&package_dir, SELF_REPO, SELF_STREAM, &version)?;

	// create the binary
	create_bin_symlink(bin_name, &binst_path)?;

	println!(
		"  Done - You can now delete this {} file, it has been copied to {}",
		exec_path.to_string_lossy(),
		binst_path.to_string_lossy()
	);

	println!(
		r#"
  IMPORTANT: Add '~/.binst/bin/' to your PATH environment. 
    You can add the 'source "$HOME/.binst/env"' in your sh file
      1) On mac: echo '\nsource "$HOME/.binst/env"' >> ~/.zshenv
      2) On linux: echo 'source "$HOME/.binst/env"' >> ~/.bashrc
"#
	);

	Ok(())
}
