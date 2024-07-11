use crate::ssh::{Auth, AuthKey, Config};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

#[derive(Debug, Deserialize)]
struct TomlConfig {
    server: ServerConfig,
    auth: AuthConfig,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    addr: String,
    port: u16,
    user: String,
}

#[derive(Debug, Deserialize)]
struct AuthConfig {
    #[serde(rename = "type")]
    auth_type: AuthType,
    password: Option<String>,
    private_key_path: Option<String>,
    public_key_path: Option<String>,
    passphrase: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AuthType {
    Password,
    Key,
    Agent,
}

fn default_config_contents() -> &'static str {
    // use include_str! to include the example.toml as binary
    include_str!("../example.toml")
}

pub fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut config_path = dirs::home_dir().ok_or("Cannot find the config directory")?;
    config_path.push(".config");
    config_path.push("cserun");
    fs::create_dir_all(&config_path)?; // make sure the directory exists
    config_path.push("config.toml");
    Ok(config_path)
}

fn read_config() -> Result<TomlConfig, Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    // check if the config file exists
    if !config_path.exists() {
        let mut file = File::create(&config_path)?;
        file.write_all(default_config_contents().as_bytes())?;
        eprintln!(
            "Config file created at {:?}, please fill in the necessary information",
            config_path
        );
        std::process::exit(1);
    }

    let contents = fs::read_to_string(&config_path)?;
    let config: TomlConfig = toml::from_str(&contents)?;

    Ok(config)
}

pub fn get_ssh_config() -> Config {
    let config: TomlConfig = read_config().unwrap_or_else(|e| {
        eprintln!("Error reading config: {}", e);
        std::process::exit(1);
    });
    // match the auth type
    let auth: Auth = match config.auth.auth_type {
        AuthType::Password => {
            let password = match config.auth.password {
                Some(p) => p,
                None => {
                    // ask for password
                    rpassword::prompt_password("ssh password: ").unwrap()
                }
            };
            Auth::Password(password)
        }
        AuthType::Key => {
            let private_key_path = match config.auth.private_key_path {
                Some(p) => PathBuf::from(p.as_str()),
                None => {
                    eprintln!("Private key path not found in config");
                    std::process::exit(1);
                }
            };
            let public_key_path = config
                .auth
                .public_key_path
                .map(|p| PathBuf::from(p.as_str()));
            let passphrase = config.auth.passphrase;
            Auth::Key(AuthKey {
                pubkey: public_key_path,
                privekey: private_key_path,
                passphrase,
            })
        }
        AuthType::Agent => Auth::Agent,
    };
    Config {
        server_addr: format!("{}:{}", config.server.addr, config.server.port),
        username: config.server.user,
        auth,
        command: String::new(),
        envs: Vec::new(),
        no_sync: false,
    }
}
