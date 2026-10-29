use std::sync::OnceLock;

use windows::{
    Foundation::TimeSpan,
    Media::Control::GlobalSystemMediaTransportControlsSessionManager,
    Storage::Streams::{DataReader, IRandomAccessStreamWithContentType},
};

static MANAGER: OnceLock<GlobalSystemMediaTransportControlsSessionManager> = OnceLock::new();

fn manager() -> Option<&'static GlobalSystemMediaTransportControlsSessionManager> {
    if let Some(m) = MANAGER.get() {
        return Some(m);
    }
    let m = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()?
        .get()
        .ok()?;
    Some(MANAGER.get_or_init(|| m))
}

pub fn get_smtc_info() -> String {
    query().unwrap_or_else(|| "No media".to_string())
}

pub fn get_playback_status() -> i32 {
    manager()
        .and_then(|m| m.GetCurrentSession().ok())
        .and_then(|s| s.GetPlaybackInfo().ok())
        .and_then(|p| p.PlaybackStatus().ok())
        .map(|status| status.0)
        .unwrap_or_default()
}

pub fn get_source_app_id() -> String {
    manager()
        .and_then(|m| m.GetCurrentSession().ok())
        .and_then(|s| s.SourceAppUserModelId().ok())
        .map(|h| h.to_string())
        .unwrap_or_default()
}

fn query() -> Option<String> {
    let session = manager()?.GetCurrentSession().ok()?;

    let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;

    let title = props
        .Title()
        .map(|h| h.to_string())
        .unwrap_or_else(|_| "Unknown".to_string());
    let artist = props.Artist().map(|h| h.to_string()).unwrap_or_default();
    
    let (pos, dur) = session
        .GetTimelineProperties()
        .map(|tl| {
            (
                tl.Position().map(ts_to_ms).unwrap_or(0),
                tl.EndTime().map(ts_to_ms).unwrap_or(0),
            )
        })
        .unwrap_or((0, 0));

    Some(format!("{title}|{artist}|{pos}|{dur}"))
}

fn ts_to_ms(ts: TimeSpan) -> i64 {
    ts.Duration / 10_000
}

/// 当前会话封面的原始字节（通常为 JPEG 或 PNG）。
///
/// 直接返回缩略图流的原始字节，不做解码或转码：调用方按图片字节解析，
/// 与 SMTC 侧提供的编码保持一致。
pub fn get_cover_bytes() -> Vec<u8> {
    read_thumbnail_bytes().unwrap_or_default()
}

fn read_thumbnail_bytes() -> Option<Vec<u8>> {
    let props = manager()?
        .GetCurrentSession()
        .ok()?
        .TryGetMediaPropertiesAsync()
        .ok()?
        .get()
        .ok()?;
    let thumbnail = props.Thumbnail().ok()?;
    let stream: IRandomAccessStreamWithContentType = thumbnail.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()?;
    if size == 0 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size as u32).ok()?.get().ok()?;
    let mut buffer = vec![0u8; size as usize];
    reader.ReadBytes(&mut buffer).ok()?;
    Some(buffer)
}