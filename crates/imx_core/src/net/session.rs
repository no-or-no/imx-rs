use std::sync::Arc;
use crossbeam::atomic::AtomicCell;
use crate::net::AuthKey;
use crate::net::ping::Ping;
use crate::net::time_sync::TimeSync;

pub fn msg_id_to_time(id: i64) -> i64 {
    (id as f64 / 4294967296.0 * 1000.0) as i64
}

pub fn time_to_msg_id(time: i64) -> i64 {
    (time as f64 * 4294967296.0 / 1000.0) as i64
}

///
#[derive(Clone)]
pub struct Session {
    sync: TimeSync,
    last_out_msg_id: Arc<AtomicCell<i64>>,
    ping: Arc<AtomicCell<Ping>>,
    pub first_salt: i64,
}

impl Session {
    pub fn new() -> Self {
        Self {
            sync: TimeSync::new(),
            last_out_msg_id: Arc::new(AtomicCell::new(0)),
            ping: Arc::new(AtomicCell::new(Ping::new())),
            first_salt: 0,
        }
    }

    /// 生成消息 id, [Message Identifier](https://core.telegram.org/mtproto/description#message-identifier-msg-id)
    pub fn new_msg_id(&self) -> i64 {
        let now = self.sync.now();
        let mut id = time_to_msg_id(now);
        self.last_out_msg_id.fetch_update(|last_id| {
            if id <= last_id {
                id = last_id + 1;
            }
            while id % 4 != 0 {
                id += 1;
            }
            Some(id)
        }).unwrap();
        return id;
    }

    /// 使用消息 id 进行时间同步
    pub fn sync_time(&self, msg_id: i64) {
        let time = msg_id_to_time(msg_id);
        let diff = time - TimeSync::local_millis();
        self.set_time_diff(diff as i32);
    }

    /// 设置 time_diff, 单位: 秒
    pub fn set_time_diff(&self, diff: i32) {
        self.sync.update(diff);
    }

    /// 获取 ping 状态
    pub fn get_ping_state(&self) -> Ping {
        self.ping.load()
    }

    /// 设置 ping 状态
    pub fn set_ping_state(&self, state: Ping) {
        self.ping.store(state)
    }
}