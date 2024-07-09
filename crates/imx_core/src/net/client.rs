use std::time::Duration;
use anyhow::{bail, Result};
use bytes::Bytes;
use dashmap::DashMap;
use log::{error, info};
use tokio::time::sleep;

use crate::net::{Addrs, DataCenter};
use crate::net::connection::{ConnType, Connection};
use crate::net::error::Error;
use crate::net::event::{Event, event_channel, EventReceiver, EventSender};
use crate::net::socket::Socket;
use crate::util::HashMap;

/// 网络消息客户端
pub(crate) struct Client {
    data_centers: HashMap<i32, DataCenter>,
    cur_dc_id: i32,
    cur_user_id: i64,
    sending_push_ping: bool,
    sending_ping: bool,
}

impl Client {

    pub fn new(addrs: Addrs) -> Self {
        let len = addrs.len();
        let mut data_centers = HashMap::default();
        for i in 0..len {
            let addr = addrs[i].clone();
            let dc = DataCenter::new(i as i32, addr);
            data_centers.insert(i as i32, dc);
        }

        Self {
            data_centers,
            cur_dc_id: 0,
            cur_user_id: 0,
            sending_push_ping: false,
            sending_ping: false,
        }
    }

    /// 每秒执行一次
    pub async fn select(&mut self) -> Result<()> {
        let cur_dc_id = self.cur_dc_id;
        debug_assert!(self.data_centers.contains_key(&cur_dc_id));
        let mut dc = self.data_centers.get_mut(&cur_dc_id).unwrap();

        let conn = dc.generic_conn().await?;

        // 处理 events
        let rx = conn.socket.receiver();
        let ev_count = rx.len();
        info!("events count: {}", ev_count);
        for _ in 0..ev_count {
            match rx.recv().await {
                Event::OnReceivedData(_) => {}
                Event::OnSocketError(e) => {
                    error!("{}", e);
                }
                Event::OnIntercepted => {
                    dc.close().await;
                    bail!(Error::Intercepted);
                }
            }
        }

        // ping
        conn.ping().await?;

        Ok(())
    }

    pub async fn destroy(&mut self) {
        for dc in &mut self.data_centers {
            dc.close().await;
        }
    }

    pub async fn send_msg(&self, msg: Bytes) -> Result<Bytes> {
        info!("send: {:?}", msg);
        todo!()
    }

    async fn send_ping(&mut self, dc: &DataCenter, use_push: bool) {

    }

    async fn process_request_queue(&self, _conn_type: ConnType, _dc_id: usize) {
        info!("process_request_queue");
    }

    pub async fn on_connection_connected(&mut self, dc_id: usize, conn_type: ConnType) -> Result<()> {
        // let dc = self.data_centers[dc_id].clone();
        //
        // if (conn_type == ConnType::Generic || conn_type == ConnType::GenericMedia)
        //     && dc.is_handshaking_any() {
        //     return dc.on_handshake_connection_connected(conn_type).await;
        // }
        //
        // if dc.has_auth_key(conn_type, true) {
        //     if conn_type == ConnType::Push {
        //         // sendingPushPing = false;
        //         // lastPushPingTime = getCurrentTimeMonotonicMillis();
        //         self.send_ping(&dc, true).await;
        //     } else {
        //         if conn_type == ConnType::Generic && dc_id == self.cur_dc_id {
        //             // sendingPing = false;
        //         }
        //         // if (networkPaused && lastPauseTime != 0) {
        //         //    lastPauseTime = getCurrentTimeMonotonicMillis();
        //         // }
        //         self.process_request_queue(conn_type, dc_id).await;
        //     }
        // }
        
        Ok(())
    }

}

