use core::time::Duration;
use std::future::Future;

use anyhow::{bail, Result};
use bytes::Bytes;
use crossbeam::scope;
use log::{error, info};
use num::abs;
use tokio::time;

use crate::defines::PING_DURATION;
use crate::net::{Addr, AuthKey, handshake, Session};
use crate::net::event::{Event, event_channel, EventReceiver, EventSender};
use crate::net::socket::{Error, Socket, SocketImpl};
use crate::net::time_sync::TimeSync;
use crate::proto::{ByteBuffer, MtDe, MtRpc, MtSer, PingDelayDisconnect};
use crate::proto::msg::{Encrypted, MsgWrap, Unencrypted};
use crate::proto::transport::{Abridged, PaddedIntermediate, Transport};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConnType {
    None = 0,
    All = 1 | 2 | 4,
    Generic = 1,
    Download = 2,
    Upload = 4,
    Push = 8,
    Temp = 16,
    Proxy = 32,
    GenericMedia = 64,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ConnState {
    Idle,
    Connecting,
    Reconnecting,
    Connected,
    Suspended,
}

pub(crate) struct Connection<T, W> {
    dc_id: i32,
    conn_type: ConnType,
    pub(crate) socket: SocketImpl,
    transport: T,
    pub(crate) msg_wrap: W,
}

impl<T: Transport, W: MsgWrap> Connection<T, W> {
    pub async fn connect(addr: Addr, dc_id: i32, conn_type: ConnType, transport: T, msg_wrap: W) -> Result<Self> {
        let socket = SocketImpl::connect(addr).await?;

        Ok(Self {
            dc_id,
            conn_type,
            socket,
            transport,
            msg_wrap,
        })
    }

    pub async fn send_rpc<Rpc: MtRpc>(&mut self, rpc: &Rpc) -> Result<Rpc::Return> {
        let data = rpc.to_bytes()?;
        // 将原始 rpc 包装成加密/非加密消息
        let (msg_id, msg) = self.msg_wrap.wrap(&data)?;
        let mut buf = ByteBuffer::new();

        self.transport.pack(&msg, &mut buf);

        self.socket.send(&buf).await?;

        buf.clear();

        loop {
            // todo 设置超时
            match self.socket.receiver().recv().await {
                Event::OnReceivedData(data) => {
                    let recv_count = self.transport.unpack(&data, &mut buf)?;
                    if recv_count == 0 { continue; }

                    let (res_msg_id, res) = self.msg_wrap.unwrap(&buf)?;
                    if msg_id != res_msg_id { continue; }

                    return Rpc::Return::from_bytes(&res);
                }
                Event::OnSocketError(e) => {
                    error!("{}", e);
                }
                Event::OnIntercepted => {
                    bail!(Error::Intercepted);
                }
            }
        }
    }

    pub async fn close(mut self) {
        self.socket.close().await;
    }
}

impl<T: Transport> Connection<T, Unencrypted> {
    async fn handshake(mut self) -> Result<Connection<T, Encrypted>> {
        info!("Handshake step1...");
        let (rpc, step) = handshake::step1(self.dc_id)?;
        let res = self.send_rpc(&rpc).await?;
        info!("Handshake step2...");
        let (rpc, step) = handshake::step2(step, res)?;
        let res = self.send_rpc(&rpc).await?;
        info!("Handshake step3...");
        let (rpc, step) = handshake::step3(step, res)?;
        let res = self.send_rpc(&rpc).await?;
        info!("Handshake complete!");
        let c = handshake::complete(step, res)?;

        let mut session = Session::new();
        session.first_salt = c.first_salt;
        session.set_time_diff(c.time_diff);

        let msg_wrap = Encrypted::new(session.clone(), c.auth_key.into());

        let Self { dc_id, conn_type, socket, transport, .. } = self;
        Ok(Connection { dc_id, conn_type, socket, transport, msg_wrap })
    }
}

impl<T: Transport> Connection<T, Encrypted> {
    pub async fn ping(&mut self) -> Result<()> {
        let session = self.msg_wrap.session.clone();
        let mut state = session.get_ping_state();
        let now = TimeSync::local_millis();
        if abs(now - state.last_check_ping_time) >= PING_DURATION {
            state.last_check_ping_time = now;
            state.last_ping_id += 1;

            let mut req = PingDelayDisconnect::default();
            req.ping_id = state.last_ping_id;
            req.disconnect_delay = 35;

            let res = self.send_rpc(&req).await?;
            // todo
        }
        Ok(())
    }
}

impl ConnType {
    pub fn is_media_type(&self) -> bool {
        *self == ConnType::GenericMedia || *self == ConnType::Download
    }
}

/// [secret] 用于 obfuscation, 当使用 WebSokcet 的时候需要
pub async fn connect(
    addr: Addr,
    dc_id: i32,
    conn_type: ConnType,
    session: Session,
    auth_key: Option<AuthKey>,
    secret: Option<String>,
) -> Result<Connection<Abridged, Encrypted>> {
    match auth_key {
        Some(auth_key) => {
            let transport = Abridged::new().obfuscation(secret);
            let wrap = Encrypted::new(session.clone(), auth_key);
            Connection::connect(addr, dc_id, conn_type, transport, wrap).await
        }
        None => {
            let transport = Abridged::new().obfuscation(secret.clone());
            let wrap = Unencrypted::new(session.clone());
            let conn = Connection::connect(addr, dc_id, conn_type, transport, wrap).await?;
            conn.handshake().await
        }
    }
}
