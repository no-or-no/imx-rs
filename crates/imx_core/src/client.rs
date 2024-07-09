use core::time::Duration;
use std::fmt::{Debug, Formatter};
use std::thread;

use anyhow::{bail, Result};
use async_channel::{Receiver, Sender, unbounded};
use bytes::Bytes;
use log::{error, info};
use serde::Serialize;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::oneshot;
use tokio::time;
use tokio::time::Interval;

use with_crc::WithCrc;

use crate::{net, proto};
use crate::net::Addrs;
use crate::proto::{MtDe, MtRpc};

/// 即时通讯客户端
pub struct Client {
    rt: Runtime,
    tx: Sender<Action>,
}

impl Client {
    /// 创建客户端
    /// - [addrs] 服务器地址, 可以传一个或多个 ipv4 或 ipv6 地址
    ///
    /// # Examples
    /// ```rust
    /// use imx_core::Client;
    ///
    /// let client = Client::new("127.0.0.1:80")?;
    /// let client = Client::new(("127.0.0.1", 80))?;
    /// let client = Client::new(["127.0.0.1:80", "127.0.0.1:443"])?;
    ///
    /// client.start();
    /// client.stop();
    /// ```
    pub fn new<T>(addrs: T) -> Result<Self> where T: Into<Addrs> {
        let addrs = addrs.into();
        if addrs.is_empty() { bail!("addrs is empty"); }

        let rt = Builder::new_multi_thread()
            .thread_name("client-worker")
            .enable_all()
            .build()?;

        let (tx, rx) = unbounded();

        rt.spawn(async move {
            run_client(rx, addrs).await;
        });

        Ok(Self { rt, tx })
    }

    /// 启动
    pub fn start(&self) {
        if let Err(e) = self.tx.clone().try_send(Action::Start) {
            error!("{}", e);
        }
    }

    /// 停止
    pub fn stop(&self) {
        if let Err(e) = self.tx.clone().try_send(Action::Stop) {
            error!("{}", e);
        }
    }

    /// 发送消息
    pub async fn send<Rpc: MtRpc>(&self, msg: &Rpc) -> Result<Rpc::Return> {
        let bytes = proto::to_bytes(msg)?;
        let (one_tx, one_rx) = oneshot::channel();
        self.tx.clone().try_send(Action::SendMsg(bytes, one_tx))?;
        let res = one_rx.await??;
        Rpc::Return::from_bytes(&res)
    }

    /// 释放客户端, 调用之后, 无法通过 `start` 再次启动
    pub fn release(&self) {
        if let Err(e) = self.tx.clone().try_send(Action::Release) {
            error!("{}", e);
        }
    }
}

async fn run_client(mut rx: Receiver<Action>, addrs: Addrs, ) {
    let mut client: Option<net::Client> = None;
    let mut interval: Option<Interval> = None;

    loop {
        match &mut interval {
            None => {
                if let Ok(Action::Start) = rx.recv().await {
                    if client.is_some() {
                        info!("The client is running and does not need to be started again.");
                    } else {
                        info!("Start client...");
                        interval = Some(time::interval(Duration::from_secs(1)));
                        client = Some(net::Client::new(addrs.clone()));
                    }
                }
            }
            Some(timer) => {
                tokio::select! {
                    _ = timer.tick() => {
                        if let Some(client) = &mut client {
                            if let Err(e) = client.select().await {
                                error!("{}", e);
                            }
                        }
                    }
                    Ok(ev) = rx.recv() => match ev {
                        Action::SendMsg(msg, result_tx) => {
                            if let Some(client) = &mut client {
                                let result = client.send_msg(msg).await;
                                result_tx.send(result).unwrap();
                            }
                        }
                        Action::Stop => {
                            if let Some(mut client) = client.take() {
                                info!("Stop client...");
                                client.destroy().await;
                                interval.take();
                                info!("Client stopped.");
                            }
                        }
                        Action::Release => {
                            if let Some(mut client) = client.take() {
                                info!("Stop client...");
                                client.destroy().await;
                                interval.take();
                                info!("Client stopped.");
                            }
                            info!("Release client runtime!!!");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// 外部调用方与异步运行时之间交互的事件
enum Action {
    Start,
    Stop,
    SendMsg(Bytes, oneshot::Sender<Result<Bytes>>),
    Release,
}

impl Debug for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Action::SendMsg(ref bytes, _) => { write!(f, "SendMsg({:?})", bytes) }
            Action::Start => write!(f, "Start client..."),
            Action::Stop => write!(f, "Stop client!"),
            Action::Release => write!(f, "Release!!!"),
        }
    }
}
