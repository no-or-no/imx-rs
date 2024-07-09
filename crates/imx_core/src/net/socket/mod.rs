use std::io;

use anyhow::Result;
use thiserror::Error;

use crate::net::Addr;
use crate::net::event::EventReceiver;

cfg_net_tcp! {
    mod tcp;
    pub(crate) type SocketImpl = tcp::TcpSocket;
}
cfg_net_quic! {
    mod quic;
    pub(crate) type SocketImpl = quic::QuicSocket;
}
cfg_net_ws! {
    mod ws;
    pub(crate) type SocketImpl = ws::WebSocket;
}

pub trait Socket: Sized {
    /// 连接服务器
    async fn connect(addr: Addr) -> Result<Self>;
    /// 发送数据包
    async fn send(&self, data: &[u8]) -> Result<()>;
    /// 接收 [Event]
    fn receiver(&self) -> EventReceiver;
    /// 关闭连接
    async fn close(&mut self);
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("EOF")]
    EOF,

    /// 收到该错误, 表示连接已中断, 对应消息通道已关闭
    #[error("intercepted")]
    Intercepted,

    #[error("{0}")]
    Io(#[from] io::Error),

    #[cfg(all(feature = "quic", not(feature = "tcp")))]
    #[error("{0}")]
    Quic(#[from] quinn::recv_stream::ReadError),

}
