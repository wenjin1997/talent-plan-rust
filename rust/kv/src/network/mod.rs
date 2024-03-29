mod frame;
mod tls;
mod stream;

use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
pub use frame::{read_frame, FrameCoder};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tracing::info;
pub use tls::{TlsServerAcceptor, TlsClientConnector};

use crate::{CommandRequest, CommandResponse, KvError, Service};
use crate::network::stream::ProstStream;

/// 处理服务器端的某个 accept 下来的 socket 的读写
// 旧的接口
// pub struct ProstServerStream<S> {
//     inner: S,
//     service: Service,
// }
pub struct ProstServerStream<S> {
    inner: ProstStream<S, CommandRequest, CommandResponse>,
    service: Service,
}

/// 处理客户端 socket 的读写
// 旧的接口
// pub struct ProstClientStream<S> {
//     inner: S,
// }
pub struct ProstClientStream<S> {
    inner: ProstStream<S, CommandResponse, CommandRequest>,
}

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S, service: Service) -> Self {
        Self {
            inner: ProstStream::new(stream),
            service,
        }
    }

    pub async fn process(mut self) -> Result<(), KvError> {
        let stream = &mut self.inner;
        while let Some(Ok(cmd)) = stream.next().await {
            info!("Got a new command: {:?}", cmd);
            let res = self.service.execute(cmd);
            stream.send(res).await.unwrap();
        }
        // info!("Client {:?} disconnected", self.addr);
        Ok(())
    }

    // // 旧的接口方法， 删除
    // pub async fn process(mut self) -> Result<(), KvError> {
    //     while let Ok(cmd) = self.recv().await {
    //         info!("Got a new command: {:?}", cmd);
    //         let res = self.service.execute(cmd);
    //         self.send(res).await?;
    //     }
    //     // info!("Client {:?} disconnected", self.addr);
    //     Ok(())
    // }

    // // 旧的接口方法，删除
    // async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
    //     let mut buf = BytesMut::new();
    //     msg.encode_frame(&mut buf)?;
    //     let encode = buf.freeze();
    //     self.inner.write_all(&encode[..]).await?;
    //     Ok(())
    // }

    // // 旧的接口方法，删除
    // async fn recv(&mut self) -> Result<CommandRequest, KvError> {
    //     let mut buf = BytesMut::new();
    //     let stream = &mut self.inner;
    //     read_frame(stream, &mut buf).await?;
    //     CommandRequest::decode_frame(&mut buf)
    // }
}

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self { inner: ProstStream::new(stream) }
    }

    pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
        let stream = &mut self.inner;
        stream.send(cmd).await?;

        match stream.next().await {
            Some(v) => v,
            None => Err(KvError::Internal("Didn't get any response".into()))
        }
    }

    // // 旧的接口方法，删除
    //
    // pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
    //     self.send(cmd).await?;
    //     Ok(self.recv().await?)
    // }
    //
    // async fn send(&mut self, msg: CommandRequest) -> Result<(), KvError> {
    //     let mut buf = BytesMut::new();
    //     msg.encode_frame(&mut buf)?;
    //     let encoded = buf.freeze();
    //     self.inner.write_all(&encoded[..]).await?;
    //     Ok(())
    // }
    //
    // async fn recv(&mut self) -> Result<CommandResponse, KvError> {
    //     let mut buf = BytesMut::new();
    //     let stream = &mut self.inner;
    //     read_frame(stream, &mut buf).await?;
    //     CommandResponse::decode_frame(&mut buf)
    // }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use bytes::Bytes;
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};

    use crate::{assert_res_ok, MemTable, ServiceInner, Value};

    use super::*;

    #[tokio::test]
    async fn client_server_basic_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        // 发送 HSET，等待回应

        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let res = client.execute(cmd).await.unwrap();

        // 第一次 HSET 服务器应该返回 None
        assert_res_ok(res, &[Value::default()], &[]);

        // 再发一个 HSET
        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute(cmd).await?;

        // 服务器应该返回上一次的结果
        assert_res_ok(res, &["v1".into()], &[]);

        Ok(())
    }

    #[tokio::test]
    async fn client_server_compression_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        let v: Value = Bytes::from(vec![0u8; 16384]).into();
        let cmd = CommandRequest::new_hset("t2", "k2", v.clone());
        let res = client.execute(cmd).await?;

        assert_res_ok(res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t2", "k2");
        let res = client.execute(cmd).await?;

        assert_res_ok(res, &[v], &[]);

        Ok(())
    }

    async fn start_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let service: Service = ServiceInner::new(MemTable::new()).into();
                let server = ProstServerStream::new(stream, service);
                tokio::spawn(server.process());
            }
        });

        Ok(addr)
    }
}

#[cfg(test)]
pub mod utils {
    use std::io::Error;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use bytes::{BufMut, BytesMut};
    use tokio::io::{AsyncRead, AsyncWrite};

    pub struct DummyStream {
        pub buf: BytesMut,
    }

    impl AsyncRead for DummyStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            // 看看 ReadBuf 需要多大的数据
            let len = buf.capacity();

            // split 出这么大的数据
            let data = self.get_mut().buf.split_to(len);

            // 拷贝给 ReadBuf
            buf.put_slice(&data);

            // 直接完工
            std::task::Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for DummyStream {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
            self.get_mut().buf.put_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }
}