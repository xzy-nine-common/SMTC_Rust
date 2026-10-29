//! 系统声音采集（WASAPI 回环）与 PCM 交付。
//!
//! Windows 上对「渲染（输出）」设备打开输入流时，cpal 会置
//! `AUDCLNT_STREAMFLAGS_LOOPBACK`，因此无需手写 COM 即可采集系统正在播放的声音。
//! 采集到的样本统一转为 16 位小端交错 PCM，经回调交给宿主（Kotlin）推流。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
mod imp {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{SampleFormat, StreamConfig};

    /// 打开的采集流与其配置。
    ///
    /// `stream` 只需被持有：cpal 的 `Stream` 在 Drop 时停止采集并释放设备，
    /// 因此字段本身不读取，但必须随结构体存活。
    pub struct Capture {
        #[allow(dead_code)]
        pub stream: cpal::Stream,
        pub sample_rate: i32,
        pub channels: i32,
    }

    /// 句柄不可跨线程直接持有 `Stream` 之外的类型限制由上层 Mutex 承担。
    unsafe impl Send for Capture {}

    /// 在默认输出设备上打开回环采集流。
    ///
    /// `on_data` 收到的缓冲区是 16 位小端交错 PCM。
    pub fn open<F>(on_data: F) -> Option<Capture>
    where
        F: FnMut(&[i16]) + Send + 'static,
    {
        let host = cpal::default_host();
        // 回环采集的目标是「输出」设备：在渲染设备上开输入流
        let device = host.default_output_device()?;
        let supported = device.default_output_config().ok()?;
        // cpal 0.18 起 SampleRate 为 u32 类型别名（早期版本为 SampleRate 元组结构体）
        let sample_rate = supported.sample_rate() as i32;
        let channels = supported.channels() as i32;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();

        let mut callback = on_data;
        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_input_stream(
                    config.clone(),
                    move |data: &[f32], _| {
                        let mut pcm = Vec::with_capacity(data.len());
                        for s in data {
                            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            pcm.push(v);
                        }
                        callback(&pcm);
                    },
                    |err| on_stream_error(&err),
                    None,
                )
                .ok()?,
            SampleFormat::I16 => device
                .build_input_stream(
                    config.clone(),
                    move |data: &[i16], _| callback(data),
                    |err| on_stream_error(&err),
                    None,
                )
                .ok()?,
            SampleFormat::U16 => device
                .build_input_stream(
                    config.clone(),
                    move |data: &[u16], _| {
                        let mut pcm = Vec::with_capacity(data.len());
                        for s in data {
                            pcm.push((*s as i32 - 32768) as i16);
                        }
                        callback(&pcm);
                    },
                    |err| on_stream_error(&err),
                    None,
                )
                .ok()?,
            other => {
                log::warn!("不支持的采样格式: {other:?}");
                return None;
            }
        };

        if stream.play().is_err() {
            log::warn!("启动采集流失败");
            return None;
        }
        Some(Capture {
            stream,
            sample_rate,
            channels,
        })
    }

    /// 采集流错误分级：瞬态抖动降级，真实故障才告警。
    ///
    /// `Xrun`（缓冲欠载/过载）在回环采集下常见且不影响音频连续性，
    /// 按 trace 记录以免刷屏；`DeviceChanged` 表示输出设备切换后流已自动改道，仍处活动状态。
    pub fn on_stream_error(err: &cpal::Error) {
        match err.kind() {
            cpal::ErrorKind::Xrun => log::trace!("音频采集缓冲抖动: {err}"),
            cpal::ErrorKind::DeviceChanged => log::info!("音频输出设备已切换，采集流已改道"),
            _ => log::warn!("音频采集流错误: {err}"),
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    /// 非 Windows 平台无回环采集，保持空实现。
    pub struct Capture {
        pub sample_rate: i32,
        pub channels: i32,
    }

    pub fn open<F>(_on_data: F) -> Option<Capture>
    where
        F: FnMut(&[i16]) + Send + 'static,
    {
        None
    }

    /// 非 Windows 无采集流，保留同名入口以免上层分支差异。
    pub fn on_stream_error(_err: &str) {
        log::trace!("非 Windows 平台无采集流");
    }
}

/// 采集会话：流需常驻，句柄由全局槽持有。
static CAPTURE: OnceLock<Mutex<Option<imp::Capture>>> = OnceLock::new();

/// 采集是否运行中。
static RUNNING: AtomicBool = AtomicBool::new(false);

fn slot() -> &'static Mutex<Option<imp::Capture>> {
    CAPTURE.get_or_init(|| Mutex::new(None))
}

///
/// 启动系统声音采集。
///
/// `on_data` 在采集线程被调用，参数为 16 位小端交错 PCM。
/// 返回 0 表示成功，负值表示失败；已在运行时返回 0（幂等）。
pub fn start<F>(on_data: F) -> i32
where
    F: FnMut(&[i16]) + Send + 'static,
{
    if RUNNING.load(Ordering::SeqCst) {
        return 0;
    }
    let capture = match imp::open(on_data) {
        Some(c) => c,
        None => {
            log::warn!("打开系统声音采集失败");
            return -1;
        }
    };
    if let Ok(mut guard) = slot().lock() {
        *guard = Some(capture);
    }
    RUNNING.store(true, Ordering::SeqCst);
    0
}

/// 停止采集并释放设备。
pub fn stop() {
    if !RUNNING.swap(false, Ordering::SeqCst) {
        return;
    }
    if let Ok(mut guard) = slot().lock() {
        *guard = None;
    }
}

/// 采集是否运行中。
pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

/// 采集流的采样率与声道数；未运行时返回 (0, 0)。
pub fn format() -> (i32, i32) {
    match slot().lock() {
        Ok(guard) => match guard.as_ref() {
            Some(c) => (c.sample_rate, c.channels),
            None => (0, 0),
        },
        Err(_) => (0, 0),
    }
}
