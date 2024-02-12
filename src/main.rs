mod parse;
mod ssh;

use clap::{arg, command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = command!()
        .arg(arg!([command] "Command to execute").required(true))
        .get_matches();

    let command_to_exec = matches.get_one::<String>("command").unwrap();

    println!("Executing command: {}", command_to_exec);
    let mut conf = parse::get_ssh_config();
    conf.command.push_str(command_to_exec.as_str());
    ssh::exec(conf)?;
    Ok(())
}
