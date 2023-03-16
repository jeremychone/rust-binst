// -- Re-exports
pub use self::error::{Error, Result};
pub use self::exec::cmd_exec;

// -- Imports
use semver::Version;

// -- Sub-Modules
pub mod clap_cmd;
pub mod error;
pub mod exec;
pub mod setup;

struct InstalledBinInfo {
	stream: String,
	version: Version,
	repo_raw: String,
}

pub const CARGO_TOML: &str = "Cargo.toml";
