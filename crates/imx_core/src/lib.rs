extern crate core;

pub use client::Client;
pub use net::{Addr, Addrs};

#[macro_use]
mod macros;

mod client;
pub mod defines;
mod net;
pub mod proto;

pub mod util;
