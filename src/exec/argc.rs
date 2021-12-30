use clap::{crate_version, App, Arg};

pub fn version() -> String {
	crate_version!()[..].to_owned()
}

pub fn cmd_app() -> App<'static> {
	App::new("binst")
		.version(&crate_version!()[..])
		.about("Decentralized binary install and deployment")
		.subcommand(sub_setup())
		.subcommand(sub_publish())
		.subcommand(sub_install())
		.subcommand(sub_update())
}

// region:    Subcommands
fn sub_setup() -> App<'static> {
	App::new("self").about("Self installing binst into ~/.binst/bin/binst")
}

fn sub_publish() -> App<'static> {
	App::new("publish")
		.about("Publish the --release binary")
		.arg(arg_repo())
		.arg(arg_at_path())
		.arg(arg_profile())
		.arg(arg_target())
}

fn sub_install() -> App<'static> {
	App::new("install")
		.about("install an binary package from a repo")
		.arg(arg_repo())
		.arg(Arg::new("bin_name").required(true).help("Name of the bin package"))
		.arg(arg_stream())
		.arg(arg_profile())
}

fn sub_update() -> App<'static> {
	App::new("update")
		.about("update an already installed library")
		.arg(arg_repo().required(false)) // turn off require for upteate
		.arg(Arg::new("bin_name").required(true).help("Name of the bin package"))
		.arg(arg_profile())
}
// endregion: Subcommands

// region:    Common Args
fn arg_repo() -> Arg<'static> {
	Arg::new("repo")
		.short('r')
		.takes_value(true)
		.help("The repo path e.g., s3://bucket_name/repo-base or https://mydomain.com/repo-base")
}

fn arg_at_path() -> Arg<'static> {
	Arg::new("path")
		.long("path")
		.short('p')
		.takes_value(true)
		.help("Named path to publish/install (for fix 'version' path). Relative to the arch-target root.")
}

fn arg_profile() -> Arg<'static> {
	Arg::new("profile").long("profile").takes_value(true).help("AWS Profile")
}

fn arg_stream() -> Arg<'static> {
	Arg::new("stream")
		.long("stream")
		.short('s')
		.takes_value(true)
		.help("Release stream (default main)")
}

fn arg_target() -> Arg<'static> {
	Arg::new("target")
		.long("target")
		.short('t')
		.takes_value(true)
		.help("Platform target, e.g., x86_64-apple-darwin. Override the default target. Must be supported by cargo --target")
}
// endregion: Common Args
