#![allow(unused)]

macro_rules! cfg_net_tcp {
    ($($item:item)*) => {
        $(
            #[cfg(any(feature = "tcp", not(any(feature = "quic", feature = "ws"))))]
            $item
        )*
    }
}

macro_rules! cfg_net_quic {
    ($($item:item)*) => {
        $(
            #[cfg(all(feature = "quic", not(feature = "tcp")))]
            $item
        )*
    }
}

macro_rules! cfg_net_ws {
    ($($item:item)*) => {
        $(
            #[cfg(all(feature = "ws", not(feature = "tcp"), not(feature = "quic")))]
            $item
        )*
    }
}