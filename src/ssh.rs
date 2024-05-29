use console::{style, Emoji};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use ssh2::Session;
use std::fs;
use std::io::Write;
use std::io::{self, Read};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

static TRUCK: Emoji<'_, '_> = Emoji("🚚  ", "");
static CLIP: Emoji<'_, '_> = Emoji("🔗  ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨  ", "");
static NETWORK: Emoji<'_, '_> = Emoji("🌐  ", "");
static LOCK: Emoji<'_, '_> = Emoji("🔒  ", "");
static PROHIBITED: Emoji<'_, '_> = Emoji("🚫  ", "");
static FOLDER: Emoji<'_, '_> = Emoji("📁 ", "");
static FILE: Emoji<'_, '_> = Emoji("📄 ", "");
static SPACESHIP: Emoji<'_, '_> = Emoji("🚀  ", "");

pub struct AuthKey {
    pub pubkey: Option<PathBuf>,
    pub privekey: PathBuf,
    pub passphrase: Option<String>,
}

pub enum Auth {
    Password(String),
    AuthKey(AuthKey),
    Agent,
}

pub struct Config {
    pub server_addr: String,
    pub username: String,
    pub auth: Auth,
    pub command: String,
    pub envs: Vec<String>,
    pub no_sync: bool,
}

pub fn exec(conf: Config) -> Result<i32, Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect(&conf.server_addr)?;
    println!(
        "{} {} Connecting to {}",
        style("[1/5]").bold().dim(),
        NETWORK,
        style(conf.server_addr).italic().cyan()
    );

    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;
    println!(
        "{} {} Handshake successful",
        style("[2/5]").bold().dim(),
        CLIP
    );

    match conf.auth {
        Auth::Password(p) => {
            sess.userauth_password(conf.username.as_str(), p.as_str())?;
        }
        Auth::AuthKey(auth_key) => {
            sess.userauth_pubkey_file(
                conf.username.as_str(),
                auth_key.pubkey.as_ref().map(|p| p.as_path()),
                auth_key.privekey.as_path(),
                auth_key.passphrase.as_ref().map(|p| p.as_str()),
            )?;
        }
        Auth::Agent => {
            let mut agent = sess.agent()?;
            agent.connect()?;
            agent.list_identities()?;
            let identities = agent.identities()?;
            if identities.len() == 0 {
                return Err("No identities found in the ssh-agent".into());
            }
            sess.userauth_agent(conf.username.as_str())?;
        }
    }
    println!(
        "{} {} Authenticated as {}",
        style("[3/5]").bold().dim(),
        LOCK,
        style(conf.username).italic().green()
    );

    let sftp = sess.sftp()?;

    let local_dir = "./";
    // get current timestep as file name. e.g. ~/.cserun/temp/2024-02-14-01-10-40-224/
    let temp_dir_name = chrono::Local::now()
        .format("%Y-%m-%d-%H-%M-%S-%3f")
        .to_string();
    let remote_dir = format!(".cserun/temp/{}", temp_dir_name); // ssh2's sftp use ~/ as root, no need to add ~/
    let remote_dir_path = Path::new(&remote_dir);

    // create the remote dir
    sftp_mkdir_recursive(&sftp, remote_dir_path)?;

    // log the command to command.txt
    let mut remote_command_file = sftp.create(remote_dir_path.join("command.txt").as_path())?;
    remote_command_file.write_all(conf.command.as_bytes())?;

    // setup the container dir
    let container_path = remote_dir_path.join("container");
    if !conf.no_sync {
        upload_dir(&sftp, Path::new(local_dir), container_path.as_path())?;
        println!(
            "{} {} Synced local files to remote",
            style("[4/5]").bold().dim(),
            TRUCK
        );
    } else {
        // only create the container dir
        sftp.mkdir(container_path.as_path(), 0o755)?;
        println!(
            "{} {} Skipped syncing local files",
            style("[4/5]").bold().dim(),
            PROHIBITED
        );
    }

    let mut channel = sess.channel_session()?;
    let mut pre_exec_command = String::new();
    // set environment variables
    for env in conf.envs {
        let env: Vec<&str> = env.split(':').collect();
        // libssh2's setenv may not work with cse server https://github.com/libssh2/libssh2/issues/546
        channel.setenv(env[0], env[1]).unwrap_or_else(|_| {
            pre_exec_command.push_str(&format!("export {}={} && ", env[0], env[1]));
        });
    }
    println!(
        "{} {} Environment variables set",
        style("[5/5]").bold().dim(),
        SPARKLE
    );
    // before exec, cd to the remote dir
    pre_exec_command.push_str(&format!("cd {}/container && ", remote_dir));
    let command = format!("{}{}", pre_exec_command, conf.command);
    channel.exec(&command)?;
    println!(
        "{} {} Command sented: {}",
        style("[5/5]").bold().dim(),
        SPACESHIP,
        style(conf.command).yellow(),
    );

    // set to unblocking mode
    sess.set_blocking(false);

    println!(
        "{} {} {}",
        style("===============").bold().magenta(),
        style("Output").italic().bold().magenta(),
        style("===============").bold().magenta()
    );
    let mut buffer = [0; 4096];
    loop {
        if channel.eof() {
            // if channel closed, break the loop
            break;
        }

        let mut is_data_available = false;

        // try to read the standard output
        match channel.read(&mut buffer) {
            Ok(size) if size > 0 => {
                print!("{}", String::from_utf8_lossy(&buffer[..size]));
                is_data_available = true;
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e.into()),
        }

        // try to read the standard error
        match channel.stderr().read(&mut buffer) {
            Ok(size) if size > 0 => {
                eprint!("{}", String::from_utf8_lossy(&buffer[..size]));
                is_data_available = true;
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e.into()),
        }

        if !is_data_available {
            // wait for 100ms to reduce CPU usage
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    channel.wait_close()?;
    println!(
        "{}",
        style("======================================")
            .bold()
            .magenta()
    );

    let exit_status = channel.exit_status()?;
    match exit_status {
        0 => println!("Exit status: {}", style("Success").green()),
        _status => println!("Exit status: {}", style(format!("Error {}", _status)).red()),
    }

    Ok(exit_status)
}

fn sftp_mkdir_recursive(sftp: &ssh2::Sftp, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_path = PathBuf::new();
    for component in path.components() {
        current_path.push(component);
        if let Ok(metadata) = sftp.stat(current_path.as_path()) {
            if metadata.is_dir() {
                continue;
            }
            return Err(format!("{:?} is not a directory", current_path).into());
        }
        sftp.mkdir(current_path.as_path(), 0o755)?;
    }
    Ok(())
}

// upload every file and directory in the local directory to remote directory
fn upload_dir(
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_base_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let walker = WalkBuilder::new(local_path)
        .ignore(true) // https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html#method.ignore
        .git_ignore(true) // https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html#method.git_ignore
        .build();

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        .template("{prefix:.bold.dim} {spinner} {wide_msg}")?;
    let pb = ProgressBar::new_spinner();
    pb.set_style(spinner_style);
    pb.set_prefix("Syncing");
    pb.enable_steady_tick(Duration::from_millis(100));

    for result in walker {
        if let Ok(entry) = result {
            let path = entry.path();
            // Calculate the relative path
            if let Ok(strip_path) = path.strip_prefix(local_path) {
                let remote_path = remote_base_path.join(strip_path);
                if path.is_dir() {
                    // Make sure the remote directory exists
                    match sftp.mkdir(&remote_path, 0o755) {
                        Ok(_) => pb.set_message(
                            format!("{} Created remote dir: {:?}", FOLDER, remote_path).to_string(),
                        ),
                        Err(err) => {
                            println!("Directory creation error (might already exist): {:?}", err)
                        }
                    }
                } else {
                    upload_file(sftp, path, &remote_path)?;
                    pb.set_message(
                        format!("{} Uploaded file: {:?}", FILE, remote_path).to_string(),
                    );
                }
            }
        }
    }

    Ok(())
}

fn upload_file(
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::open(local_path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    let mut remote_file = sftp.create(remote_path)?;
    remote_file.write_all(&contents)?;

    Ok(())
}
