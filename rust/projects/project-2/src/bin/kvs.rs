use clap::{Parser, Subcommand};
use kvs::{ KvStore, KvError, Result};
use std::env::current_dir;
use std::process::exit;

/// A key-value store
#[derive(Debug, Parser)]
#[clap(name = "kvs")]
#[clap(about = "A key-value store", long_about = None)]
#[clap(author, version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Set the value of a string key to a string
    #[clap(arg_required_else_help = true)]
    Set {
        #[clap(required = true, value_parser)]
        key: String,

        #[clap(required = true, value_parser)]
        value: String,
    },

    /// Get the string value of a given string key"
    Get {
        /// A string key
        #[clap(required = true, value_parser)]
        key: String,
    },

    /// Remove a given key
    Rm {
        /// A string key
        #[clap(required = true, value_parser)]
        key: String,
    },
}


fn main() -> Result<()>{
    let args = Cli::parse();

    match &args.command {
        Commands::Set {key, value } => {
            let mut store = KvStore::open(current_dir()?)?;
            store.set(key.to_string(), value.to_string())?;
        }

        Commands::Get { key } => {
            let mut store = KvStore::open(current_dir()?)?;
            if let Some(value) = store.get(key.to_string())? {
                println!("{}", value);
            } else {
                println!("Key not found");
            }
        }

        Commands::Rm { key } => {
            let mut store = KvStore::open(current_dir()?)?;
            match store.remove(key.to_string()) {
                Ok(()) => { }
                Err(KvError::KeyNotFound) => {
                    println!("Key not found");
                    exit(1);
                }
                Err(e) => return Err(e),
            }
        }
    }
    Ok(())
}