//! 轮询文档的修改时间，被外部程序修改时重新读取并回调。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::doc::{self, Decoded};

const INTERVAL: Duration = Duration::from_millis(700);

/// `last_mtime` 与主线程共享：主线程保存文件时持锁写入并更新它，避免把自己的保存当成外部修改。
pub fn spawn(
    path: PathBuf,
    last_mtime: Arc<Mutex<Option<SystemTime>>>,
    on_change: impl Fn(Decoded) + Send + 'static,
) {
    thread::spawn(move || loop {
        thread::sleep(INTERVAL);
        let mut last = last_mtime.lock().unwrap();
        let now = doc::mtime(&path);
        // 有些编辑器先删除再重命名，中间短暂不存在，跳过即可
        if now.is_none() || now == *last {
            continue;
        }
        *last = now;
        if let Ok(decoded) = doc::load(&path) {
            on_change(decoded);
        }
    });
}
