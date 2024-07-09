use core::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use core::ops::Index;
use core::str::FromStr;
use std::borrow::Borrow;
use std::net::{AddrParseError, IpAddr};

/// An internet address, like [SocketAddr]
#[derive(Debug, Clone)]
pub enum Addr {
    /// Internet socket address
    SocketAddr(SocketAddr),
    /// Custom address
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Addrs(Vec<Addr>);

impl PartialEq for Addr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Addr::SocketAddr(addr1), Addr::SocketAddr(addr2)) => addr1 == addr2,
            (Addr::Custom(text1), Addr::Custom(text2)) => text1 == text2,
            _ => false,
        }
    }
}

impl Addrs {
    #[inline]
    pub fn is_empty(&self) -> bool { self.0.is_empty() }

    #[inline]
    pub fn len(&self) -> usize { self.0.len() }
}

impl Index<usize> for Addrs {
    type Output = Addr;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.0.index(index)
    }
}

impl TryFrom<Addr> for SocketAddr {
    type Error = AddrParseError;

    fn try_from(value: Addr) -> Result<Self, Self::Error> {
        match value {
            Addr::SocketAddr(s) => Ok(s),
            Addr::Custom(s) => SocketAddr::from_str(s.as_str())
        }
    }
}

// =========== From ================
impl From<SocketAddr> for Addr {
    fn from(addr: SocketAddr) -> Self {
        Addr::SocketAddr(addr)
    }
}

impl From<&str> for Addr {
    fn from(value: &str) -> Self {
        match SocketAddr::from_str(value) {
            Ok(addr) => Addr::SocketAddr(addr),
            _ => Addr::Custom(value.to_owned())
        }
    }
}

impl<T: AsRef<str>> From<(T, u16)> for Addr {
    fn from(value: (T, u16)) -> Self {
        let ip = value.0.as_ref();
        let port = value.1;
        if let Ok(addr) = Ipv4Addr::from_str(ip) {
            return SocketAddr::V4(SocketAddrV4::new(addr, port)).into();
        }
        if let Ok(addr) = Ipv6Addr::from_str(ip) {
            return SocketAddr::V6(SocketAddrV6::new(addr, port, 0, 0)).into();
        }
        Addr::Custom(format!("{}:{}", ip, port))
    }
}

impl<T: Into<Addr>> From<T> for Addrs {
    fn from(value: T) -> Self {
        Self(vec![value.into()])
    }
}

impl<T: Into<Addr>> From<Vec<T>> for Addrs {
    fn from(value: Vec<T>) -> Self {
        let vec = value.into_iter().map(|x| x.into()).collect();
        Self(vec)
    }
}

impl<T: Into<Addr>, const N: usize> From<[T; N]> for Addrs {
    fn from(value: [T; N]) -> Self {
        let vec = value.into_iter().map(|x| x.into()).collect();
        Self(vec)
    }
}
