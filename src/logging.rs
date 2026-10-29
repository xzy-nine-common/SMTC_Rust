//! 库日志桥。
//!
//! 与 `notify-relay-core` 的 `log_bridge` 同构：全局 logger 只能安装一次，
//! 本模块持有唯一安装点，并按 target 过滤第三方库输出，避免依赖噪声刷屏。
//!
//! 宿主进程（JVM 桌面端）可能已由 core 先安装 logger，此时 `set_logger` 失败属正常：
//! 本库日志改由 core 的桥转发到平台日志，同样可用；为避免影响 core 的级别设置，
//! 仅在成功安装时才调整 `max_level`。

use std::sync::Once;

/// 本 crate 的 target 前缀；模块路径形如 `smtc_jni::audio`。
const CRATE_TARGET: &str = "smtc_jni";

static INIT: Once = Once::new();

struct Bridge;

impl log::Log for Bridge {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        // 只转发本库日志：cpal / windows / jni 等第三方输出一律丢弃
        if !record.target().starts_with(CRATE_TARGET) {
            return;
        }
        eprintln!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

static BRIDGE: Bridge = Bridge;

/// 安装全局 logger（进程内只生效一次）。
pub fn init() {
    INIT.call_once(|| {
        if log::set_logger(&BRIDGE).is_ok() {
            // 仅在本库 logger 生效时设置级别，避免改动宿主进程中其它库的级别
            log::set_max_level(log::LevelFilter::Info);
        }
    });
}
