use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use async_channel::Receiver;
use crossbeam::atomic::AtomicCell;
use log::info;
use quinn::{ClientConfig, Endpoint, SendStream};
use quinn::crypto::rustls::QuicClientConfig;
use tokio::io::AsyncWriteExt;

use crate::defines::READ_BUFFER_SIZE;
use crate::net::Addr;
use crate::net::event::{Event, event_channel, EventReceiver, EventSender};
use crate::net::socket::{Channel, channel, Error, Socket};

/// [quinn 文档](https://quinn-rs.github.io/quinn/networking-introduction.html)
/// [quinn examples](https://github.com/quinn-rs/quinn/tree/main/quinn/examples)
pub(crate) struct QuicSocket {
    intercept: Arc<AtomicCell<bool>>,
    tx: EventSender,
    rx: EventReceiver,
    send: SendStream,
}

impl Socket for QuicSocket {
    async fn connect(addr: Addr) -> Result<Self> {
        info!("(quic)Connecting {:?} ...", addr);
        // let mut client_crypto = rustls::ClientConfig::builder()
        //     .with_root_certificates(roots)
        //     .with_no_client_auth();
        let server_addr = SocketAddr::try_from(addr)?;

        let quic_config = QuicClientConfig::try_from(client_crypto)?;
        let config = ClientConfig::new(Arc::new(quic_config));

        // 本地端口
        let mut endpoint = Endpoint::client("127.0.0.1:5000".into())?;
        endpoint.set_default_client_config(config);

        let conn = endpoint.connect(server_addr, "")?.await?;
        let (mut send, mut recv) = conn.open_bi().await?;

        let (tx, rx) = event_channel();
        let tx2 = tx.clone();

        let intercept = Arc::new(AtomicCell::new(false));
        let intercept2 = intercept.clone();

        tokio::spawn(async move {
            let mut buf = vec![0; READ_BUFFER_SIZE];
            loop {
                if intercept2.load() {
                    tx2.close();
                    break;
                }

                match recv.read(&mut buf).await {
                    Ok(None) => {
                        tx2.send(Event::OnSocketError(Error::EOF)).await.ok();
                        tx2.close();
                        break;
                    }
                    Ok(Some(n)) => {
                        buf.truncate(n);
                        tx2.send(Event::OnReceivedData(buf.to_vec())).ok();
                    }
                    Err(e) => {
                        tx2.send(Event::OnSocketError(Error::Quic(e))).await.ok();
                    }
                }
            }
            info!("(quic)Connection intercepted");
        });

        // todo
        //conn.remote_address()

        Ok(Self { intercept, tx, rx, send })
    }

    async fn send(&self, data: &[u8]) -> Result<()> {
        todo!()
    }

    fn receiver(&self) -> EventReceiver {
        self.rx.clone()
    }

    async fn close(&mut self) {
        if let Err(e) = self.send.shutdown().await {
            self.tx.send(Event::OnSocketError(Error::Io(e))).await.ok();
        }
        self.intercept.store(true);
    }
}