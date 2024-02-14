mod parse;
mod ssh;

use clap::Parser;

/// A simple tool to run commands on CSE server
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    /// Do not sync files before running the command
    #[clap(long)]
    no_sync: bool,

    /// Set environment variables, in the format of KEY:VALUE
    #[clap(long, value_name = "KEY:VALUE", value_parser = parse_env)]
    env: Vec<String>,

    /// The command to run on the cse server
    command: String,
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
    let command_to_exec = args.command;

    println!("Executing command: {}", command_to_exec);
    let mut conf = parse::get_ssh_config();
    conf.command.push_str(command_to_exec.as_str());
    conf.envs = args.env;
    conf.no_sync = args.no_sync;
    ssh::exec(conf)?;
    Ok(())
}
