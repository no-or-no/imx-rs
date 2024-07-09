use std::sync::Arc;
use chrono::Local;
use crossbeam::atomic::AtomicCell;

/// [Time Synchronization](https://core.telegram.org/mtproto#time-synchronization)
#[derive(Default, Clone)]
pub struct TimeSync(Arc<AtomicCell<i32>>);

impl TimeSync {
    /// 本机时间毫秒值
    pub fn local_millis() -> i64 {
        Local::now().timestamp_millis()
    }

    /// 本机时间 秒
    pub fn local_seconds() -> i32 {
        (Local::now().timestamp_millis() / 1000) as i32
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// 获取当前时间毫秒值
    pub fn now(&self) -> i64 {
        let diff = self.time_diff() as i64;
        Self::local_millis() + diff * 1000
    }

    /// 获取 time_diff, 单位: 秒
    pub fn time_diff(&self) -> i32 {
        self.0.load()
    }

    /// 设置 time_diff, 单位: 秒
    pub fn update(&self, diff: i32) {
        self.0.store(diff);
    }

}