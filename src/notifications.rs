//! 本机通知采集（WinRT `UserNotificationListener`）。
//!
//! 需在应用清单声明 `User Notification Listener` 能力；实测非打包进程下
//! `RequestAccessAsync` 亦可返回 `Allowed`（用户在系统设置中允许后），
//! 故不依赖稀疏 MSIX 即可工作。
//!
//! 采集为轮询式：WinRT 的 `NotificationChanged` 事件在非 UI 线程上不可靠，
//! 轮询 `GetNotificationsAsync` 并按 id 差分更稳定（与 Windows 端做法一致）。

/// 一条本机通知的采集结果（字段与 Windows `LocalNotificationListenerService` 对齐）。
#[derive(Clone, Debug)]
pub struct Notification {
    /// 系统通知 id（u32 的十进制字符串）。
    pub id: String,
    /// 应用显示名。
    pub app_name: String,
    /// 应用标识：优先 AUMID，其次包族名。
    pub app_package: String,
    pub title: String,
    pub text: String,
}

#[cfg(target_os = "windows")]
mod imp {
    use super::Notification;
    use windows::UI::Notifications::Management::{
        UserNotificationListener, UserNotificationListenerAccessStatus,
    };
    use windows::UI::Notifications::NotificationKinds;

    /// 获取监听器；未授权时返回错误说明。
    fn listener() -> Result<UserNotificationListener, String> {
        let listener =
            UserNotificationListener::Current().map_err(|e| format!("获取监听器失败: {e}"))?;
        let status = listener
            .RequestAccessAsync()
            .map_err(|e| format!("请求访问失败: {e}"))?
            .get()
            .map_err(|e| format!("等待授权失败: {e}"))?;
        if status != UserNotificationListenerAccessStatus::Allowed {
            return Err(format!("未授权: {status:?}"));
        }
        Ok(listener)
    }

    /// 读取当前全部 Toast 通知。
    pub fn list() -> Result<Vec<Notification>, String> {
        let listener = listener()?;
        let items = listener
            .GetNotificationsAsync(NotificationKinds::Toast)
            .map_err(|e| format!("枚举通知失败: {e}"))?
            .get()
            .map_err(|e| format!("等待通知列表失败: {e}"))?;

        let mut out = Vec::new();
        let size = items.Size().map_err(|e| format!("读取数量失败: {e}"))?;
        for i in 0..size {
            let item = match items.GetAt(i) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let id = item.Id().map(|v| v.to_string()).unwrap_or_default();
            let app_info = match item.AppInfo() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let aumid = app_info.AppUserModelId().map(|s| s.to_string()).unwrap_or_default();
            let pfn = app_info.PackageFamilyName().map(|s| s.to_string()).unwrap_or_default();
            log::debug!("通知 AppInfo: aumid=[{aumid}] pfn=[{pfn}]");
            let app_package = if !aumid.is_empty() {
                aumid
            } else if !pfn.is_empty() {
                pfn
            } else {
                String::new()
            };
            let app_name = app_info
                .DisplayInfo()
                .ok()
                .and_then(|d| d.DisplayName().ok())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| app_package.clone());

            // 标题取绑定的第一个文本，其余文本按行拼接为内容
            let mut title = String::new();
            let mut text = String::new();
            if let Ok(notification) = item.Notification() {
                if let Ok(visual) = notification.Visual() {
                    if let Ok(bindings) = visual.Bindings() {
                        if let Ok(count) = bindings.Size() {
                            if count > 0 {
                                if let Ok(binding) = bindings.GetAt(0) {
                                    if let Ok(elements) = binding.GetTextElements() {
                                        let n = elements.Size().unwrap_or(0);
                                        for j in 0..n {
                                            let Ok(el) = elements.GetAt(j) else { continue };
                                            let t = el.Text().map(|s| s.to_string()).unwrap_or_default();
                                            if t.trim().is_empty() {
                                                continue;
                                            }
                                            if j == 0 {
                                                title = t;
                                            } else if text.is_empty() {
                                                text = t;
                                            } else {
                                                text.push('\n');
                                                text.push_str(&t);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            out.push(Notification {
                id,
                app_name,
                app_package,
                title,
                text,
            });
        }
        Ok(out)
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::Notification;

    pub fn list() -> Result<Vec<Notification>, String> {
        Err("非 Windows 平台不支持本机通知采集".to_string())
    }
}

///
/// 读取当前全部本机通知。
///
/// 每条形如 `id\u{1}appName\u{1}appPackage\u{1}title\u{1}text`，多条以 `\u{2}` 分隔；
/// 字段内含分隔符时以空格替代，保证可解析。
pub fn list_encoded() -> Result<String, String> {
    const FIELD_SEP: char = '\u{1}';
    const RECORD_SEP: char = '\u{2}';
    let items = imp::list()?;
    let sanitize = |s: &str| s.replace(FIELD_SEP, " ").replace(RECORD_SEP, " ");
    let encoded = items
        .iter()
        .map(|n| {
            format!(
                "{}{}{}{}{}{}{}{}{}",
                sanitize(&n.id),
                FIELD_SEP,
                sanitize(&n.app_name),
                FIELD_SEP,
                sanitize(&n.app_package),
                FIELD_SEP,
                sanitize(&n.title),
                FIELD_SEP,
                sanitize(&n.text)
            )
        })
        .collect::<Vec<_>>()
        .join(&RECORD_SEP.to_string());
    Ok(encoded)
}
