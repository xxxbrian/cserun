use ssh2::Session;
use std::io::{self, Read};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

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
}

pub fn exec(conf: Config) -> Result<(), Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect(conf.server_addr)?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

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

    let mut channel = sess.channel_session()?;
    channel.exec(conf.command.as_str())?;

    // set to unblocking mode
    sess.set_blocking(false);

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
    println!("\nExit status: {}", channel.exit_status()?);

    Ok(())
}
