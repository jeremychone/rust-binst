use std::os::unix::fs::symlink;
use std::{fs::remove_dir_all, path::Path};

use thiserror::Error;
use toml::Value;

#[derive(Error, Debug)]
pub enum UtilsError {
	#[error("The toml value not found path {0}")]
	TomlValueNotFound(String),

	#[error("The directory {0} does not seems to be safe to delete (no 'binst' in the path")]
	DirNotSafeToDelete(String),

	#[error(transparent)]
	IOError(#[from] std::io::Error),
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
