use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};

use anyhow::bail;
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<SmarthomeServer>,
}

#[derive(Serialize, Deserialize)]
pub struct SmarthomeServer {
    pub id: String,
    pub url: String,
    pub token: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            servers: vec![SmarthomeServer::default()],
        }
    }
}

impl Default for SmarthomeServer {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            url: "http://smarthome.box".to_string(),
            token: "-".repeat(32),
        }
    }
}

pub fn file_path() -> Option<String> {
    match env::var("HOME") {
        Ok(home) => {
            if let Ok(xdg_home) = env::var("XDG_CONFIG_HOME") {
                Some(format!("{}/homescript-ls-rs/config.toml", xdg_home))
            } else {
                Some(format!("{}/.config/homescript-ls-rs/config.toml", home))
            }
        }
        Err(_) => None,
    }
}

pub fn read_config(file_path: &str) -> anyhow::Result<Option<Config>> {
    // Either read or create a configuration file based on it's current existence
    let path = Path::new(file_path);
    match &path.exists() {
        true => {
            // The file exists, it can be read
            debug!("Found existing config file at {file_path}");
            let content = fs::read_to_string(path)?;
            let config = toml::from_str(&content)?;
            // Validate the contents of the config file
            Ok(Some(validate_config(config)?))
        }
        false => {
            // The file does not exist, therefore create a new one
            fs::create_dir_all(path.parent().unwrap())?;
            let mut file = File::create(path)?;
            // TODO: dont do this
            // file.write_all(include_bytes!("default_config.toml"))?;
            // In case a few new struct fields must be serialized
            file.write_all(
                toml::to_string_pretty(&Config::default())
                    .unwrap()
                    .as_bytes(),
            );
            Ok(None)
        }
    }
}

fn validate_config(config: Config) -> anyhow::Result<Config> {
    let mut ids: Vec<&str> = Vec::with_capacity(config.servers.len());
    if config.servers.is_empty() {
        bail!("No servers specified: at least one server must be specified")
    }
    for server in &config.servers {
        // Validate that every ID is unique
        if ids.contains(&server.id.as_str()) {
            bail!(
                "Duplicate server ID: the ID `{}` must be unique",
                server.id.as_str()
            )
        }
        ids.push(&server.id);

        // Check that there is an authentication token
        if server.token.is_empty() {
            bail!(
                "No authentication token provided for server `{}`",
                server.id
            )
        }

        // Validate that the token is well-formed
        if server.token.len() != 32 {
            bail!(
                "Malformed access token for server {}: token is not 32 characters long",
                server.id
            )
        }
        if server.token.contains(' ') || !server.token.is_ascii() {
            bail!(
                "Malformed access token for server {}: may not contain whitespace or non-ASCII characters",
                server.id
            )
        }
    }
    Ok(config)
}
