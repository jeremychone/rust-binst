use clap::{App, SubCommand};

pub fn cmd_app() -> App<'static, 'static> {
	App::new("binst")
		.version("0.1.0")
		.about("binary install and deployment")
		.subcommand(sub_setup())
}

fn sub_setup() -> App<'static, 'static> {
	SubCommand::with_name("setup").about("Setting update the ~/.binst/ folder")
}
