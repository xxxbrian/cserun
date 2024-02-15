mod parse;
mod ssh;

use clap::Parser;

/// A simple tool to run commands on CSE server
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    /// Do not sync files before running the command
    #[clap(long, requires = "command")]
    no_sync: bool,

    /// Set environment variables, in the format of KEY:VALUE
    #[clap(long, value_name = "KEY:VALUE", value_parser = parse_env, requires = "command")]
    env: Vec<String>,

    /// The command to run on the cse server
    command: Option<String>,

    /// Show the path of config file
    #[clap(long, conflicts_with_all = &["no_sync", "env", "command"])]
    config: bool,
}

fn parse_env(s: &str) -> Result<String, String> {
    if s.contains(':') && s.split(':').count() == 2 && !s.starts_with(':') && !s.ends_with(':') {
        Ok(s.to_string())
    } else {
        Err("Environment variable must be in KEY:VALUE format".to_string())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.config {
        let config_path = parse::get_config_path()?;
        println!("Config file path: {}", config_path.display());
        return Ok(());
    }

    let command_to_exec = args.command.unwrap();

    let mut conf = parse::get_ssh_config();
    conf.command.push_str(command_to_exec.as_str());
    conf.envs = args.env;
    conf.no_sync = args.no_sync;
    match ssh::exec(conf) {
        Ok(_) => Ok(()),
        Err(e) => {
            // ask user to check the config file
            let config_path = parse::get_config_path()?;
            let new_e = format!(
                "Error: {}, please check the config file at {}",
                e,
                config_path.display()
            );
            Err(new_e.into())
        }
    }
}
