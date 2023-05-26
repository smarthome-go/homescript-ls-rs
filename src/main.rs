use anyhow::{bail, Context};
use clap::Parser;
use cli::Args;
use log::{info, Level};
use loggerv::Logger;
use smarthome_sdk_rs::{Auth, Client};

mod cli;
mod config;
mod ls;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logger
    Logger::new()
        .max_level(if args.verbose {
            Level::Trace
        } else {
            Level::Info
        })
        .colors(true)
        .level(true)
        .module_path_filters(vec![env!("CARGO_PKG_NAME").replace('-', "_")])
        .module_path(false)
        .init()
        .unwrap();

    // Select an appropriate configuration file path
    let config_path = match args.config_file_path {
        Some(from_args) => from_args,
        None => match config::file_path() {
            Some(path) => path,
            None => bail!("Your home directory could not be determined.\nHINT: To use this program, please use the manual config file path command-line-flag")
        }
    };

    // Read or create the configuration file
    let conf = match config::read_config(&config_path)
        .with_context(|| format!("Could not read or create config file (at {config_path})"))?
    {
        Some(conf) => conf,
        None => {
            info!("Created a new configuration file (at `{config_path}`).\nHINT: To get started, edit this file to set up your server(s) and run this program again.");
            return Ok(());
        }
    };

    // Select a server profile based on command line arguments or the default
    let profile = match args.server_id {
        Some(from_args) => match conf.servers.iter().find(|server| server.id == from_args) {
            Some(found) => found,
            None => {
                bail!("Invalid server id from args: the id `{from_args}` was not found in the server list");
            }
        },
        None => &conf.servers[0],
    };

    // Create a Smarthome client
    let smarthome_client =
        match Client::new(&profile.url, Auth::QueryToken(profile.token.clone())).await {
            Ok(client) => client,
            Err(err) => {
                bail!("Could not connect to Smarthome: {err}");
            }
        };

    ls::start_service(smarthome_client).await;
    Ok(())
}
