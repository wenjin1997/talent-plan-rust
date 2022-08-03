use std::net::SocketAddr;
use std::process::exit;
use structopt::StructOpt;
use kvs::{Result, KvsClient};

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const ADDRESS_FORMAT: &str = "IP:PORT";

/// A kvs client
#[derive(Debug, StructOpt)]
#[structopt(name = "kvs-client", about = "A kvs client")]
struct Cli {
    #[structopt(subcommand)]
    command: Commands,
}

#[derive(Debug, StructOpt)]
enum Commands {
    /// Set the value of a string key to a string.
    #[structopt(name = "set")]
    Set {
        /// A string key
        #[structopt(name = "key")]
        key: String,

        /// The string value of the key
        #[structopt(name = "value")]
        value: String,

        /// Sets the server address
        #[structopt(long, value_name = ADDRESS_FORMAT, default_value = DEFAULT_LISTENING_ADDRESS, parse(try_from_str))]
        addr: SocketAddr,
    },

    /// Get the string value of a given string key.
    #[structopt(name = "get")]
    Get {
        /// A string key
        #[structopt(name = "key")]
        key: String,

        /// Sets the server address
        #[structopt(long, value_name = ADDRESS_FORMAT, default_value = DEFAULT_LISTENING_ADDRESS, parse(try_from_str))]
        addr: SocketAddr,
    },

    /// Remove a given string key.
    #[structopt(name = "rm")]
    Rm {
        /// A string key
        #[structopt(name = "key")]
        key: String,

        /// Sets the server address
        #[structopt(long, value_name = ADDRESS_FORMAT, default_value = DEFAULT_LISTENING_ADDRESS, parse(try_from_str))]
        addr: SocketAddr,
    }
}

fn main() {
    let cli = Cli::from_args();
    if let Err(e) = run(cli) {
        eprintln!("{}", e);
        exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Get { key,addr } => {
            let mut client = KvsClient::connect(addr)?;
            if let Some(value) = client.get(key)? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
        }
        Commands::Set {key, value, addr} => {
            let mut client = KvsClient::connect(addr)?;
            client.set(key, value)?;
        }
        Commands::Rm { key , addr} => {
            let mut client = KvsClient::connect(addr)?;
            client.remove(key)?;
        }
    }
    Ok(())
}