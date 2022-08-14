use crate::common::{GetResponse, RemoveResponse, Request, SetResponse};
use crate::{KvsEngine, Result};
use crate::thread_pool::ThreadPool;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use log::{debug, error};
use serde_json::Deserializer;

/// 一个键值对存储的服务器。
///
/// `E: KvsEngine` 代表 kvs 的引擎，可以是 kvs 或者 sled。
pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    engine: E,
    pool: P,
}

impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    /// 给定一个数据库引擎，创建一个 `KvsServer`。
    pub fn new(engine: E, pool: P) -> Self {
        KvsServer { engine, pool }
    }

    /// 监听给定的地址，运行服务器
    pub fn run<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        // bind 方法将创建一个 TcpListener，绑定到给定的地址，返回的监听器已准备好接收连接。
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            let engine = self.engine.clone();
            self.pool.spawn(move || match stream {
                Ok(stream) => {
                    if let Err(e) = serve(engine, stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            })
        }
        Ok(())
    }
}

fn serve<E: KvsEngine>(engine: E, tcp: TcpStream) -> Result<()> {
    // peer_addr() 方法： Returns the socket address of the remote peer of this TCP connection.
    let peer_addr = tcp.peer_addr()?;
    let reader = BufReader::new(&tcp);
    let mut writer = BufWriter::new(&tcp);
    let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();

    macro_rules! send_resp {
        ($resp:expr) => {
            {
                let resp = $resp;
                serde_json::to_writer(&mut writer, &resp)?;
                writer.flush()?;
                debug!("Response sent to {}: {:?}", peer_addr, resp);
            }
        };
    }

    for req in req_reader {
        let req = req?;
        debug!("Receive request from {}: {:?}", peer_addr, req);
        match req {
            Request::Get { key } => send_resp!(match engine.get(key) {
                Ok(value) => GetResponse::Ok(value),
                Err(e) => GetResponse::Err(format!("{}", e)),
            }),
            Request::Set { key, value } => send_resp!(match engine.set(key, value) {
                Ok(_) => SetResponse::Ok(()),
                Err(e) => SetResponse::Err(format!("{}", e)),
            }),
            Request::Remove { key } => send_resp!(match engine.remove(key) {
                Ok(_) => RemoveResponse::Ok(()),
                Err(e) => RemoveResponse::Err(format!("{}", e)),
            }),
        };
    }

    Ok(())
}
