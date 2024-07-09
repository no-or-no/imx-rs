#[cfg(debug_assertions)]
pub use debug::*;
#[cfg(not(debug_assertions))]
pub use release::*;

pub const PFS_ENABLED: bool = true;
/// 接收网络数据的缓冲区大小
pub const READ_BUFFER_SIZE: usize = 1024 * 1024 * 2;
pub const TEMP_AUTH_KEY_EXPIRE_TIME: i32 = 24 * 60 * 60;

#[cfg(debug_assertions)]
mod debug {
    pub const PING_DURATION: i64 = 5000;
}

#[cfg(not(debug_assertions))]
mod release {
    pub const PING_DURATION: i64 = 19000;
}
