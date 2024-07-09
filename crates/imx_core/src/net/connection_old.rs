use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};

use anyhow::{bail, Result};
use bytes::Bytes;
use crossbeam::atomic::AtomicCell;
use rand::random;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::net::Addr;
use crate::net::error::Error;
use crate::net::event::{Event, EventSender};

thread_local! { static LAST_CONN_TOKEN: Cell<u32> = Cell::new(1); }


#[allow(unused)]
struct ConnInner {
    addr: Addr,
    socket: Mutex<Option<ConnSocketInner>>,
    conn_type: ConnType,
    conn_state: AtomicCell<ConnState>,
    conn_token: AtomicU32,
    session_id: AtomicI64,
    next_seq_no: AtomicU32,
    first_packet_sent: AtomicBool,
    cur_proto_type: AtomicCell<ProtoType>,
    encrypt_key: AtomicCell<[u8; 32]>,
    encrypt_iv: AtomicCell<[u8; 16]>,
    decrypt_key: AtomicCell<[u8; 32]>,
    decrypt_iv: AtomicCell<[u8; 16]>,
}

impl Connection {
    pub fn new(dc_id: usize, addr: Addr, conn_type: ConnType, tx: EventSender) -> Self {
        let inner = ConnInner {
            addr,
            socket: Mutex::new(None),
            conn_type,
            conn_state: AtomicCell::new(ConnState::Idle),
            conn_token: AtomicU32::new(0),
            session_id: AtomicI64::new(random()),
            next_seq_no: AtomicU32::new(0),
            first_packet_sent: AtomicBool::new(false),
            cur_proto_type: AtomicCell::new(ProtoType::EE),
            encrypt_key: AtomicCell::new([0; 32]),
            encrypt_iv: AtomicCell::new([0; 16]),
            decrypt_key: AtomicCell::new([0; 32]),
            decrypt_iv: AtomicCell::new([0; 16]),
        };
        Self {
            dc_id, tx,
            inner: Arc::new(inner),
        }
    }

    pub fn get_dc_id(&self) -> usize { self.dc_id }

    pub fn get_type(&self) -> ConnType { self.inner.conn_type }

    pub fn get_state(&self) -> ConnState { self.inner.conn_state.load() }

    pub fn get_token(&self) -> u32 { self.inner.conn_token.load(Ordering::Acquire) }

    pub fn get_session_id(&self) -> i64 {
        self.inner.session_id.load(Ordering::Acquire)
    }

    pub fn generate_new_session_id(&self) {
        self.inner.session_id.store(random(), Ordering::Release);
    }

    pub fn generate_msg_seq_no(&self, increment: bool) -> u32 {
        let value = self.inner.next_seq_no.load(Ordering::Acquire);
        if increment {
            self.inner.next_seq_no.fetch_add(1, Ordering::AcqRel);
        }
        value * 2 + if increment { 1 } else { 0 }
    }

    pub async fn connect(&self) -> Result<()> {
        // todo
        let conn_state = self.inner.conn_state.load();
        if conn_state == ConnState::Connected || conn_state == ConnState::Connecting { return Ok(()); }

        self.inner.conn_state.store(ConnState::Connecting);

        self.inner.first_packet_sent.store(false, Ordering::Release);
        // todo

        // let (socket, mut rx) = ConnSocket::connect(self.inner.addr.clone()).await?;
        // { *self.inner.socket.lock().await = Some(socket); }

        let conn = self.clone();
        // tokio::spawn(async move {
        //     while let Some(res) = rx.recv().await {
        //         match res {
        //             Ok(bytes) => {
        //                 if let Err(e) = conn.on_received_data(bytes).await {
        //                     error!("{:?}", e);
        //                 };
        //             }
        //             Err(e) => {
        //                 error!("{:?}", e);
        //                 // 发送端不再发送新的, 缓冲区的数据继续 recv, 直到 None
        //                 rx.close();
        //             }
        //         }
        //     }
        //     drop(rx);
        // });

        self.on_connected().await?;

        Ok(())
    }

    pub async fn reconnect(&self) -> Result<()> {
        if self.inner.conn_type == ConnType::Proxy {
            self.suspend(false).await;
        } else {
            //forceNextPort = true;
            self.suspend(true).await;
            self.connect().await?;
        }
        Ok(())
    }

    async fn on_connected(&self) -> Result<()> {
        self.inner.conn_state.store(ConnState::Connected);
        LAST_CONN_TOKEN.with(|it| {
            let token = it.get();
            self.inner.conn_token.store(token, Ordering::Release);
            it.set(token + 1);
        });

        self.tx.send(Event::OnConnectionConnected {
            dc_id: self.dc_id,
            conn_type: self.inner.conn_type,
        })?;

        Ok(())
    }

    pub async fn suspend(&self, idle: bool) {
        let conn_state = self.inner.conn_state.load();
        if conn_state == ConnState::Idle || conn_state == ConnState::Suspended { return; }
        self.inner.conn_state.store(if idle { ConnState::Idle } else { ConnState::Suspended });
        self.intercept().await;
        // onConnectionClosed
        self.inner.first_packet_sent.store(false, Ordering::Release);
        // lastPacketLength = 0;
        self.inner.conn_token.store(0, Ordering::Release);
        // wasConnected = false;
    }

    /// 发送数据
    pub async fn send_data(&self, data: Bytes, report_ack: bool, encrypted: bool) -> Result<()> {
        if data.is_empty() { bail!("data buf empty") }

        let conn_state = self.inner.conn_state.load();
        if conn_state == ConnState::Idle || conn_state == ConnState::Reconnecting || conn_state == ConnState::Suspended {
            self.connect().await?;
        }



        Ok(())
    }

    /// 接收到服务端数据
    // TODO ConnectionsManager::onConnectionDataReceived
    // TODO Handshake::processHandshakeResponse
    async fn on_received_data(&self, data: Bytes) -> Result<()> {
        // let mut data = data.to_vec();
        //
        // // todo delete
        // let s = std::str::from_utf8(&data).unwrap();
        // info!("Client received {} bytes: \n{}", data.len(), s);
        //
        // let decrypt_key = self.inner.decrypt_key.load();
        // let decrypt_iv = self.inner.decrypt_iv.load();
        // // aes256_ctr128_encrypt(&mut data, decrypt_key, decrypt_iv);
        //
        // let conn_type = self.inner.conn_type;
        // if conn_type == ConnType::Generic || conn_type == ConnType::Temp || conn_type == ConnType::GenericMedia {
        //
        // }
        //
        // let mut buf = ByteBuffer::from(data);
        // let proto_type = self.inner.cur_proto_type.load();
        // while !buf.is_empty() {
        //     if proto_type == ProtoType::EF {
        //         let f_byte = buf.read_u8();
        //         if (f_byte & (1 << 7)) != 0 {
        //
        //         }
        //     }
        // }




        // self.tx.send(Event::OnReceivedData {
        //     conn: self.clone(),
        //     data: Bytes::from(data),
        // })?;

        Ok(())
    }

    async fn send_packet(&self, data: Bytes) -> Result<()> {
        let socket = self.inner.socket.lock().await;
        match &*socket {
            Some(s) => s.send_packet(data).await,
            None => bail!("connection(dc{:?}, type ${:?}) disconnected, don't send data", self.dc_id, self.inner.conn_type)
        }
    }

    pub async fn intercept(&self) {
        if let Some(mut socket) = self.inner.socket.lock().await.take() {
            socket.intercept().await;
        }
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Connection[{:?}] dc_id={}, conn_type={:?}", self.inner.addr, self.dc_id, self.inner.conn_type)
    }
}








