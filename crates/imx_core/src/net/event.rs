use std::fmt::{Debug, Formatter};

use async_channel::{Receiver, Sender, TrySendError};

use crate::net::socket::Error;

pub fn event_channel() -> (EventSender, EventReceiver) {
    let (tx, rx) = async_channel::bounded(128);
    (EventSender(tx), EventReceiver(rx))
}

#[derive(Clone)]
pub struct EventSender(Sender<Event>);

#[derive(Clone)]
pub struct EventReceiver(Receiver<Event>);

impl EventSender {
    pub fn send(&self, ev: Event) -> Result<(), TrySendError<Event>> {
        self.0.try_send(ev)
    }

    pub fn close(&self) -> bool {
        self.0.close()
    }
}

impl EventReceiver {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub async fn recv(&self) -> Event {
        self.0.recv().await.unwrap_or_else(|_| Event::OnIntercepted)
    }

    pub fn close(&self) -> bool {
        self.0.close()
    }
}

pub enum Event {
    // OnConnectionConnected { dc_id: usize, conn_type: ConnType },
    OnReceivedData(Vec<u8>),
    OnSocketError(Error),
    OnIntercepted,
}

impl Debug for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            // Event::OnConnectionConnected { dc_id, conn_type } => write!(f, "OnConnectionConnected(dc_id={}, conn_type={:?})", dc_id, conn_type),
            Event::OnReceivedData(ref data) => write!(f, "OnReceivedData(data={:?})", data),
            Event::OnIntercepted => write!(f, "OnIntercepted"),
            Event::OnSocketError(ref e) => write!(f, "OnSocketError {}", e),
        }
    }
}