use std::env::current_dir;
use std::fs;
use structopt::StructOpt;
use std::net::SocketAddr;
use std::process::exit;
use clap::arg_enum;
use log::{warn, info, error, LevelFilter};
use kvs::{KvsEngine, KvsServer, KvsStore, SledKvsEngine};
use kvs::Result;

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const ADDRESS_FORMAT: &str = "IP:PORT";
const DEFAULT_ENGINE: Engine = Engine::kvs;

/// A kvs server
#[derive(Debug, StructOpt)]
#[structopt(name = "kvs-server", about = "A kvs server")]
struct Cli {
    /// Sets the listening address
    #[structopt(
        long,
        value_name = ADDRESS_FORMAT,
        default_value = DEFAULT_LISTENING_ADDRESS,
        parse(try_from_str)
    )]
    addr: SocketAddr,

    /// Sets the storage engine
    #[structopt(
        long,
        value_name = "ENGINE-NAME",
        possible_values = &Engine::variants()
    )]
    engine: Option<Engine>,

}

arg_enum! {
    #[allow(non_camel_case_types)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Engine {
        kvs,
        sled,
    }
}


fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let mut cli = Cli::from_args();
    // and_then 函数
    // Calls op if the result is Ok, otherwise returns the Err value of self.
    // This function can be used for control flow based on Result values.
    //  pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, op: F) -> Result<U, E> {
    //         match self {
    //             Ok(t) => op(t),
    //             Err(e) => Err(e),
    //         }
    //     }
    let res = current_engine().and_then(move |curr_engine| {
        if cli.engine.is_none() {
            cli.engine = curr_engine;
        }
        if curr_engine.is_none() && cli.engine != curr_engine {
            error!("Wrong engine!");
            exit(1);
        }
        run(cli)
    });
    if let Err(e) = res {
        error!("{}", e);
        exit(1);
    }

}

fn run(cli: Cli) -> Result<()> {
    let engine = cli.engine.unwrap_or(DEFAULT_ENGINE);
    info!("Storage engine: {}", engine);
    info!("Listening on {}", cli.addr);

    // 将 engine 写到 engine 文件中
    fs::write(current_dir()?.join("engine"), format!("{}", engine))?;

    match engine {
        Engine::kvs => run_with_engine(KvsStore::open(current_dir()?)?, cli.addr),
        Engine::sled => run_with_engine(SledKvsEngine::new(sled::open(current_dir()?)?), cli.addr),
    }
}

fn run_with_engine<E: KvsEngine>(engine: E, addr: SocketAddr) -> Result<()> {
    let server = KvsServer::new(engine);
    server.run(addr)
}

fn current_engine() -> Result<Option<Engine>> {
    let engine = current_dir()?.join("engine");
    if !engine.exists() {
        return Ok(None);
    }

    // read_to_string 函数：Read the entire contents of a file into a string.
    match fs::read_to_string(engine)?.parse() {
        Ok(engine) => Ok(Some(engine)),
        Err(e) => {
            warn!("The content of engine file is invalid: {}", e);
            Ok(None)
        }
    }
}