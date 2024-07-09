use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use async_channel::Receiver;
use crossbeam::atomic::AtomicCell;
use log::info;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;

use crate::defines::READ_BUFFER_SIZE;
use crate::net::Addr;
use crate::net::event::{Event, event_channel, EventReceiver, EventSender};
use crate::net::socket::{Error, Socket};

pub(crate) struct TcpSocket {
    intercept: Arc<AtomicCell<bool>>,
    tx: EventSender,
    rx: EventReceiver,
    wt: OwnedWriteHalf,
}

impl Socket for TcpSocket {
    async fn connect(addr: Addr) -> Result<Self> {
        info!("(tcp) Connecting {:?} ...", addr);
        let server_addr = SocketAddr::try_from(addr)?;
        // 连接服务器
        let stream = TcpStream::connect(server_addr).await?;
        // 读写分离
        let (mut rd, wt) = stream.into_split();
        // 异步 task 之间通信
        let (tx, rx) = event_channel();
        let tx2 = tx.clone();

        let intercept = Arc::new(AtomicCell::new(false));
        let intercept2 = intercept.clone();
        // 在独立的异步任务中, 监听服务端发送的数据 (服务端可能会主动推送, 或客户端发送请求后返回的响应)
        tokio::spawn(async move {
            let mut buf = vec![0; READ_BUFFER_SIZE];
            loop {
                if intercept2.load() {
                    // 退出循环之前, 先关闭消息通道, 已发送的消息不会被清空, 直到被处理
                    tx2.close();
                    break;
                }

                match rd.read(&mut buf).await {
                    Ok(0) => {
                        tx2.send(Event::OnSocketError(Error::EOF)).ok();
                        tx2.close();
                        break;
                    }
                    Ok(n) => {
                        buf.truncate(n);
                        tx2.send(Event::OnReceivedData(buf.to_vec())).ok();
                    }
                    Err(e) => {
                        tx2.send(Event::OnSocketError(Error::Io(e))).ok();
                    }
                }
            }
            info!("(tcp) Connection intercepted");
        });

        wt.writable().await?;
        info!("(tcp) Connected to {:?}", wt.peer_addr());

        Ok(Self { intercept, tx, rx, wt })
    }

    async fn send(&self, data: &[u8]) -> Result<()> {
        let mut remaining = &data[..];
        loop {
            self.wt.writable().await?;
            match self.wt.try_write(remaining) {
                Ok(n) => {
                    if n < remaining.len() {
                        remaining = &remaining[n..];
                    } else {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    fn receiver(&self) -> EventReceiver {
        self.rx.clone()
    }

    async fn close(&mut self) {
        if let Err(e) = self.wt.shutdown().await {
            self.tx.send(Event::OnSocketError(Error::Io(e))).ok();
        }
        self.intercept.store(true);
    }
}