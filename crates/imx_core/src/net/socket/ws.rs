use std::sync::Arc;

use anyhow::Result;
use crossbeam::atomic::AtomicCell;

use crate::net::Addr;
use crate::net::event::{EventReceiver, EventSender};
use crate::net::socket::Socket;

pub(crate) struct WebSocket {
    intercept: Arc<AtomicCell<bool>>,
    tx: EventSender,
    rx: EventReceiver,
}

impl Socket for WebSocket {
    async fn connect(addr: Addr) -> Result<Self> {
        todo!()
    }

    async fn send(&self, data: &[u8]) -> Result<()> {
        todo!()
    }

    fn receiver(&self) -> EventReceiver {
        self.rx.clone()
    }

    async fn close(&mut self) {
        self.intercept.store(true);
    }
}