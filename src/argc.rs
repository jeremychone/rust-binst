use clap::{crate_version, App, Arg, SubCommand};

pub fn version() -> String {
	crate_version!()[..].to_owned()
}

pub fn cmd_app() -> App<'static, 'static> {
	App::new("binst")
		.version(&crate_version!()[..])
		.about("Decentralized binary install and deployment")
		.subcommand(sub_setup())
		.subcommand(sub_publish())
		.subcommand(sub_install())
}

// region:    Subcommands
fn sub_setup() -> App<'static, 'static> {
	SubCommand::with_name("self").about("Self installing binst into ~/.binst/bin/binst")
}

fn sub_publish() -> App<'static, 'static> {
	SubCommand::with_name("publish")
		.about("Publish the --release binary")
		.arg(arg_repo())
		.arg(arg_profile())
}

fn sub_install() -> App<'static, 'static> {
	SubCommand::with_name("install")
		.about("install an binary package from ")
		.arg(arg_repo())
		.arg(Arg::with_name("bin_name").required(true).help("Name of the bing"))
		.arg(arg_profile())
}
// endregion: Subcommands

// region:    Common Args
fn arg_repo() -> Arg<'static, 'static> {
	Arg::with_name("repo")
		.short("r")
		.takes_value(true)
		.required(true)
		.help("The repo path (starts with . or / for local, otherwise, assume https:// domain/path ")
}

fn arg_profile() -> Arg<'static, 'static> {
	Arg::with_name("profile").long("profile").short("p").takes_value(true).help("Path to profile")
}
// endregion: Common Args
