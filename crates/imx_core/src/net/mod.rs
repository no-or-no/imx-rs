pub use addr::{Addr, Addrs};
pub use auth_key::AuthKey;
pub(crate) use client::Client;
pub use data_center::DataCenter;
pub use session::Session;

mod addr;
mod auth_key;
mod error;
pub(crate) mod event;
mod data_center;
pub(crate) mod handshake;
mod time_sync;
mod session;
mod ping;
mod client;
mod socket;
mod connection;

// #[derive(Debug)]
// pub struct NetworkMessage {
//     message: Message,
//     pub request_id: i32,
//     pub invoke_after: bool,
//     pub need_quick_ack: bool,
//     pub force_container: bool,
// }
//
// impl NetworkMessage {
//     pub fn new(message: Message) -> Self {
//         Self {
//             message,
//             request_id: 0,
//             invoke_after: false,
//             need_quick_ack: false,
//             force_container: false,
//         }
//     }
// }
