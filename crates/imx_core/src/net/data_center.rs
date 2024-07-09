use anyhow::Result;

use crate::net::{AuthKey, Session};
use crate::net::addr::Addr;
use crate::net::connection::{connect, Connection, ConnType};
use crate::proto::msg::Encrypted;
use crate::proto::transport::Abridged;

pub struct DataCenter {
    pub id: i32,
    pub addr: Addr,
    pub generic_conn: Option<Connection<Abridged, Encrypted>>,
    pub session: Session,
    auth_key: Option<AuthKey>,
}

impl DataCenter {
    pub fn new(id: i32, addr: Addr) -> Self {
        Self {
            id, addr,
            generic_conn: None,
            session: Session::new(),
            auth_key: None,
        }
    }

    pub async fn generic_conn(&mut self) -> Result<&mut Connection<Abridged, Encrypted>> {
        if self.generic_conn.is_none() {
            let conn = connect(
                self.addr.clone(),
                self.id,
                ConnType::Generic,
                self.session.clone(),
                self.auth_key,
                None, /* todo */
            ).await?;

            self.auth_key = Some(conn.msg_wrap.auth_key);
            self.generic_conn = Some(conn);
        }
        Ok(self.generic_conn.as_mut().unwrap())
    }

    pub async fn close(&mut self) {
        if let Some(conn) = self.generic_conn.take() {
            conn.close().await;
        }
        // todo
    }
}

/*pub struct DataCenter {
    pub id: usize,
    pub addr: Addr,
    tx: EventSender,
    handshakes: Mutex<Vec<Arc<Handshake>>>,
    generic_conn: OnceLock<Connection>,
    generic_media_conn: OnceLock<Connection>,
    temp_conn: OnceLock<Connection>,
    push_conn: OnceLock<Connection>,
    download_conn: Vec<Connection>,
    upload_conn: Vec<Connection>,
    proxy_conn: Vec<Connection>,
    is_cdn_dc: bool,
    auth_key_perm: Option<Bytes>,
    auth_key_perm_id: i64,
    auth_key_media_temp: Option<Bytes>,
    auth_key_media_temp_id: i64,
    auth_key_temp: Option<Bytes>,
    auth_key_temp_id: i64,
}

impl DataCenter {
    pub(crate) fn new(id: usize, addr: Addr, tx: EventSender) -> Self {
        Self {
            id, addr, tx,
            handshakes: Mutex::new(vec![]),
            generic_conn: OnceLock::new(),
            generic_media_conn: OnceLock::new(),
            temp_conn: OnceLock::new(),
            push_conn: OnceLock::new(),
            download_conn: vec![],
            upload_conn: vec![],
            proxy_conn: vec![],
            auth_key_perm: None,
            auth_key_perm_id: 0,
            auth_key_media_temp: None,
            auth_key_media_temp_id: 0,
            auth_key_temp: None,
            is_cdn_dc: false,
            auth_key_temp_id: 0,
        }
    }

    fn generic_conn(&self) -> Connection {
        self.generic_conn.get_or_init(|| Connection::new(
            self.id, self.addr.clone(), ConnType::Generic,
            self.tx.clone()
        )).clone()
    }

    fn generic_media_conn(&self) -> Connection {
        self.generic_media_conn.get_or_init(|| Connection::new(
            self.id, self.addr.clone(), ConnType::GenericMedia,
            self.tx.clone()
        )).clone()
    }

    fn push_conn(&self) -> Connection {
        self.push_conn.get_or_init(|| Connection::new(
            self.id, self.addr.clone(), ConnType::Push,
            self.tx.clone()
        )).clone()
    }

    fn temp_conn(&self) -> Connection {
        self.temp_conn.get_or_init(|| Connection::new(
            self.id, self.addr.clone(), ConnType::Temp,
            self.tx.clone()
        )).clone()
    }

    pub async fn get_generic_connection(&self, need_connect: bool, allow_pending_key: bool) -> Option<Connection> {
        let (auth_key, _) = self.get_auth_key(ConnType::Generic, false, allow_pending_key);
        if auth_key.is_none() { return None; }

        let conn = self.generic_conn();
        if need_connect {
            let r = conn.connect().await;
            if r.is_err() {
                error!("{}", r.unwrap_err());
                return None;
            }
        }

        Some(conn)
    }

    pub async fn get_generic_media_connection(&self, need_connect: bool, allow_pending_key: bool) -> Option<Connection> {
        let (auth_key, _) = self.get_auth_key(ConnType::GenericMedia, false, allow_pending_key);
        if auth_key.is_none() { return None; }

        let conn = self.generic_media_conn();
        if need_connect {
            let r = conn.connect().await;
            if r.is_err() {
                error!("{}", r.unwrap_err());
                return None;
            }
        }

        Some(conn)
    }

    pub async fn get_push_connection(&self, need_connect: bool) -> Option<Connection> {
        let (auth_key, _) = self.get_auth_key(ConnType::Push, false, false);
        if auth_key.is_none() { return None; }

        let conn = self.push_conn();
        if need_connect {
            let r = conn.connect().await;
            if r.is_err() {
                error!("{}", r.unwrap_err());
                return None;
            }
        }

        Some(conn)
    }

    pub async fn get_temp_connection(&self, need_connect: bool) -> Option<Connection> {
        let (auth_key, _) = self.get_auth_key(ConnType::Temp, false, true);
        if auth_key.is_none() { return None; }

        let conn = self.temp_conn();
        if need_connect {
            let r = conn.connect().await;
            if r.is_err() {
                error!("{}", r.unwrap_err());
                return None;
            }
        }

        Some(conn)
    }

    pub fn has_auth_key(&self, conn_type: ConnType, allow_pending_key: bool) -> bool {
        self.get_auth_key(conn_type, false, allow_pending_key).0.is_some()
    }

    /// 返回 (auth_key, auth_key_id)
    pub fn get_auth_key(&self, conn_type: ConnType, perm: bool, allow_pending_key: bool) -> (Option<Bytes>, i64) {
        let use_perm_key = self.is_cdn_dc || perm || !PFS_ENABLED;
        if use_perm_key {
            return (self.auth_key_perm.clone(), self.auth_key_perm_id);
        } else {
            let is_media = conn_type.is_media_type() && self.has_media_addr();
            let mut auth_key_pending: Option<Bytes> = None;
            let mut auth_key_pending_id = 0i64;

            for handshake in &*self.handshakes.lock() {
                if (is_media && handshake.typo == HandshakeType::MediaTemp)
                    || (!is_media && handshake.typo == HandshakeType::Temp) {
                    auth_key_pending = handshake.auth_key_pending.clone();
                    auth_key_pending_id = handshake.auth_key_pending_id.load(Ordering::Acquire);
                    break;
                }
            }

            if allow_pending_key && auth_key_pending.is_some() {
                return (auth_key_pending, auth_key_pending_id);
            }
            if is_media {
                return (self.auth_key_media_temp.clone(), self.auth_key_media_temp_id);
            }

            return (self.auth_key_temp.clone(), self.auth_key_temp_id);
        }
    }

    /// 返回 (requests_data, quick_ack_id)
    pub async fn create_requests_data(&self, mut requests: Vec<NetworkMessage>, conn: &Connection, pfs_init: bool) -> (Option<Bytes>, i32) {
        let (auth_key, auth_key_id) = self.get_auth_key(conn.get_type(), pfs_init, true);
        if auth_key.is_none() { return (None, 0); }

        let msg_id: i64;
        let mut msg_body: Bytes;
        let msg_seqno: i32;

        if requests.len() == 1 {
            let net_msg = requests.remove(0);
            msg_body = match &net_msg.message.outgoing_body {
                Some(b) => b.clone(),
                None => net_msg.message.body.clone()
            };

            let msg_time = msg_id_to_time(net_msg.message.msg_id);
            let cur_time = self.msg_sender.now();

            if !pfs_init && (net_msg.force_container || msg_time < cur_time - 30000 || msg_time > cur_time + 25000) {
                let mut container = MsgContainer::default();
                container.messages.push(net_msg.message);

                msg_id = self.msg_sender.gen_msg_id();
                msg_body = proto::to_bytes(&container).unwrap().into();
                msg_seqno = conn.generate_msg_seq_no(false) as i32;
            } else {
                msg_id = net_msg.message.msg_id;
                msg_seqno = net_msg.message.seqno;
            }
        } else {
            let mut container = MsgContainer::default();
            for net_msg in requests {
                container.messages.push(net_msg.message);
            }
            msg_id = self.msg_sender.gen_msg_id();
            msg_body = proto::to_bytes(&container).unwrap().into();
            msg_seqno = conn.generate_msg_seq_no(false) as i32;
        }

        let mt_proto_version = if pfs_init { 1 } else { 2 };
        let msg_size = msg_body.len();
        let mut additional_size = (32 + msg_size) % 16;
        if additional_size != 0 {
            additional_size = 16 - additional_size;
        }
        if mt_proto_version == 2 {
            let index: u8 = random();
            additional_size += ((2 + (index % 14)) * 16) as usize;
        }

        let mut buf = ByteBuffer::with_capacity(24 + 32 + msg_size + additional_size);
        buf.write_i64(auth_key_id);
        buf.set_position(24);

        if pfs_init {
            buf.write_i64(random());
            buf.write_i64(random());
        } else {
            buf.write_i64(self.get_server_salt(conn.get_type().is_media_type()));
            buf.write_i64(conn.get_session_id());
        }
        buf.write_i64(msg_id);
        buf.write_i32(msg_seqno);
        buf.write_i32(msg_size as i32);
        buf.write_all(&msg_body);

        let quick_ack_id = 0;
        // todo

        return (Some(buf.to_bytes()), quick_ack_id);
    }

    fn get_server_salt(&self, _is_media: bool) -> i64 {

        0
    }

    pub(crate) fn is_handshaking_any(&self) -> bool {
        !self.handshakes.lock().is_empty()
    }

    pub(crate) fn is_handshaking(&self, handshake_type: HandshakeType) -> bool {
        let handshakes = self.handshakes.lock();
        if !handshakes.is_empty() {
            for handshake in &*handshakes {
                if handshake.typo == handshake_type { return true; }
            }
        }
        false
    }

    /// 开始握手
    pub(crate) async fn begin_handshake(&self, handshake_type: HandshakeType, need_connect: bool) -> Result<()> {
        if handshake_type == HandshakeType::Current {
            let handshakes = self.handshakes.lock().clone();
            for handshake in handshakes {
                self.handshake(&handshake, need_connect).await?;
            }
        } else if self.auth_key_perm.is_none() {
            if !self.is_handshaking(HandshakeType::Perm) {
                let handshake = self.new_handshake(HandshakeType::Perm);
                self.handshake(&handshake, need_connect).await?;
            }
        } else if PFS_ENABLED {
            if handshake_type == HandshakeType::All || handshake_type == HandshakeType::Temp {
                if !self.is_handshaking(HandshakeType::Temp) {
                    let handshake = self.new_handshake(HandshakeType::Temp);
                    self.handshake(&handshake, need_connect).await?;
                }
            }
            if (handshake_type == HandshakeType::All || handshake_type == HandshakeType::MediaTemp) && self.has_media_addr() {
                if !self.is_handshaking(HandshakeType::MediaTemp) {
                    let handshake = self.new_handshake(HandshakeType::MediaTemp);
                    self.handshake(&handshake, need_connect).await?;
                }
            }
        }
        Ok(())
    }

    fn new_handshake(&self, typo: HandshakeType) -> Arc<Handshake> {
        let handshake = Arc::new(Handshake::new(typo));
        self.handshakes.lock().push(handshake.clone());
        return handshake;
    }

    async fn handshake(&self, handshake: &Handshake, need_connect: bool) -> Result<()> {
        info!("Start handshaking");

        let conn = self.get_connection(handshake);

        handshake.set_state(1);
        if need_connect {
            conn.suspend(false).await;
            conn.connect().await?;
        }

        // send first
        let mut req = ReqPQMulti::default();
        rand::thread_rng().fill_bytes(&mut req.nonce);

        handshake.auth_nonce.store(Some(req.nonce.clone()));

        self.msg_sender.send_unencrypted_msg(req, &conn).await
    }

    pub(crate) fn has_media_addr(&self) -> bool {
        // TODO
        false
    }

    /// 关闭所有连接
    pub async fn close_connections(&self) {
        self.generic_conn().intercept().await;
        self.generic_media_conn().intercept().await;
        self.temp_conn().intercept().await;
        self.push_conn().intercept().await;
        for conn in &self.download_conn {
            conn.intercept().await;
        }
        for conn in &self.upload_conn {
            conn.intercept().await;
        }
        for conn in &self.proxy_conn {
            conn.intercept().await;
        }
    }

    fn get_connection(&self, handshake: &Handshake) -> Connection {
        if handshake.typo == HandshakeType::MediaTemp {
            self.generic_media_conn()
        } else {
            self.generic_conn()
        }
    }

    pub(crate) async fn on_handshake_connection_connected(&self, conn_type: ConnType) -> Result<()> {
        let handshakes = self.handshakes.lock().clone();
        if handshakes.is_empty() { return Ok(()); }

        let media = conn_type == ConnType::GenericMedia;
        for h in &*handshakes {
            if (media && h.typo == HandshakeType::MediaTemp)
                || (!media && h.typo != HandshakeType::MediaTemp) {
                if h.state() == 0 || !h.need_resend_data() {
                    return Ok(());
                }
                self.handshake(&h, false).await?;
            }
        }

        Ok(())
    }

}*/
