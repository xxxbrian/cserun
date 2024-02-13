# CSERun

CSERun is a utility tool designed to assist UNSW CSE students in running course commands such as `autotest` and `give` in their local environment. Powered by Rust, it simplifies the process of executing course-specific commands directly from a local machine.

## Installation

To install CSERun, follow these steps:

1. Clone the repository:
```sh
git clone https://github.com/xxxbrian/cserun
```

2. Navigate to the cloned directory and install using cargo:
```sh
cd cserun && cargo install --path .
```

## Configuration and Usage

After installation, navigate to the directory where you wish to run commands:

```sh
cd /path/to/your/work # The directory to run commands in
# Example: cd ~/COMP6991/lab01/exercise01
```

To run a command, use CSERun as follows:

```sh
cserun "your_command_here"
# Example: cserun "6991 autotest"
```

### Configuration Details

After the initial run, CSERun will prompt you to create and modify a TOML configuration file. This file contains essential settings for connecting to the CSE server, including server details and authentication method. Here’s a breakdown of the configuration file and how to customize it:

#### Server Configuration

```toml
[server]
addr = "cse.unsw.edu.au" # Default server address, no need to change.
port = 22                # Default port, no need to change.
user = "z5555555"        # Replace "zID" with your actual zID.
```

#### Authentication Configuration

You must choose **one** of the three available authentication methods. Each method has its own set of requirements:

##### 1. Password Authentication

If you prefer using a password for authentication, uncomment and fill in the `password` field. If the password is not provided, CSERun will prompt you for it when needed.

```toml
[auth]
type = "password"
password = "your_password" # Optional. Recommended to fill for convenience.
```

##### 2. Key Authentication

For those who use SSH keys, specify the path to your private key. If your key is passphrase-protected, you can also specify the passphrase.

```toml
# [auth]
type = "key"
private_key_path = "/path/to/private/key" # Required for key authentication.
# public_key_path = "/path/to/public/key" # Optional.
# passphrase = "your_passphrase" # Optional.
```

##### 3. Agent Authentication

Agent authentication is useful if you have an SSH agent running that manages your keys.

```toml
# [auth]
type = "agent"
```

**Note:** Remember, these authentication methods are mutually exclusive; only one method should be configured in the file.

#### Completing the Configuration

After choosing and setting up your preferred authentication method, save the changes to the configuration file. Re-run CSERun in your project directory to start using it with the configured settings.

## Additional Information

To enhance file synchronization speed with the server, CSERun supports `.gitignore` and `.ignore` files. It will exclude files and directories specified in these files from syncing, which is particularly useful for ignoring project-generated directories like `node_modules` and `target`.
