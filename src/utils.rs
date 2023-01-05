use std::os::unix::fs::symlink;
use std::process::Command;
use std::process::ExitStatus;
use std::{fs::remove_dir_all, path::Path};

use thiserror::Error;
use toml::Value;

#[derive(Error, Debug)]
pub enum UtilsError {
	#[error("Fail to execute {0} cause: {1}")]
	ExecError(String, String),

	#[error("The toml value not found path {0}")]
	TomlValueNotFound(String),

	#[error("The directory {0} does not seems to be safe to delete (no 'binst' in the path")]
	DirNotSafeToDelete(String),

	#[error(transparent)]
	IOError(#[from] std::io::Error),
}

impl UtilsError {
	fn from_exec_stderr(cmd: &str, args: &[&str], cause: &dyn std::error::Error) -> Self {
		let command = format!("{} {}", cmd, args.join(" "));
		UtilsError::ExecError(command, cause.to_string())
	}
	fn from_exec_status(cmd: &str, args: &[&str], status: ExitStatus) -> Self {
		let command = format!("{} {}", cmd, args.join(" "));
		UtilsError::ExecError(command, status.to_string())
	}
}

pub fn get_toml_value<'v>(root: &'v Value, arr: &[&str]) -> Result<&'v Value, UtilsError> {
	let mut value: &Value = root;
	for name in arr {
		value = match value.get(name) {
			Some(v) => v,
			None => return Err(UtilsError::TomlValueNotFound(arr.join("."))),
		}
	}
	Ok(value)
}

pub fn get_toml_value_as_string(root: &Value, arr: &[&str]) -> Result<String, UtilsError> {
	let value = get_toml_value(root, arr)?;
	match value.as_str() {
		Some(str) => Ok(str.to_owned()),
		None => Err(UtilsError::TomlValueNotFound(arr.join("."))),
	}
}

pub fn sym_link(original: &Path, link: &Path) -> Result<(), std::io::Error> {
	// TODO: add windows support
	symlink(original, link)
}

// some small but still additional precaution when deleting directory
pub fn safer_remove_dir(dir: &Path) -> Result<(), UtilsError> {
	let path_str = dir.to_string_lossy(); // good enough for contains below
	if !path_str.contains("binst") {
		return Err(UtilsError::DirNotSafeToDelete(path_str.to_string()));
	}

	remove_dir_all(dir)?;

	Ok(())
}

//// Remove redundant / as well as start and end /
pub fn clean_path(uri: impl AsRef<str>) -> String {
	fn cleaner(s: &str) -> String {
		s.split('/').filter(|p| !p.is_empty()).collect::<Vec<&str>>().join("/")
	}

	uri.as_ref().splitn(2, "://").map(cleaner).collect::<Vec<String>>().join("://")
}

pub fn exec_cmd_args(cmd: &str, args: &[&str]) -> Result<(), UtilsError> {
	let mut proc = Command::new(cmd);
	proc.args(args);

	println!("> executing: {} {}", cmd, args.join(" "));

	match proc.spawn()?.wait() {
		Ok(status) => {
			if !status.success() {
				Err(UtilsError::from_exec_status(cmd, args, status))
			} else {
				Ok(())
			}
		}
		Err(ex) => Err(UtilsError::from_exec_stderr(cmd, args, &ex)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_clean_path() {
		// clean path like
		assert_eq!("path", clean_path("path"));
		assert_eq!("path", clean_path("path/"));
		assert_eq!("path", clean_path("/path/"));

		// clean url like
		assert_eq!("https://example.net/foo/bar", clean_path("https://example.net/foo/bar"));
		assert_eq!(
			"https://example.net/foo/bar",
			clean_path("https://example.net/foo/bar/")
		);
		assert_eq!("example.net/foo/bar", clean_path("example.net////foo/bar"));
	}
}
