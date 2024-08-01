use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Args {
    /// Which server to connect to (default is the first one of the list)
    #[clap(short, long, value_parser)]
    pub server_id: Option<String>,

    /// Override the path of the configuration file
    #[clap(short, long, value_parser)]
    pub config_file_path: Option<String>,

    /// If set, more information will be printed to the console
    #[clap(short, long, value_parser, global = true)]
    pub verbose: bool,

    /// If set, no version check is being performed during a connection attempt.
    #[clap(short, long, value_parser, global = true)]
    pub no_version_check: bool,
}
