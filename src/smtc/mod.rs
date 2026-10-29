#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(target_os = "windows"))]
mod stub;

pub fn get_smtc_info() -> String {
    #[cfg(target_os = "windows")]
    {
        windows::get_smtc_info()
    }

    #[cfg(not(target_os = "windows"))]
    {
        stub::get_smtc_info()
    }
}

pub fn get_playback_status() -> i32 {
    #[cfg(target_os = "windows")]
    {
        windows::get_playback_status()
    }

    #[cfg(not(target_os = "windows"))]
    {
        stub::get_playback_status()
    }
}

pub fn get_source_app_id() -> String {
    #[cfg(target_os = "windows")]
    {
        windows::get_source_app_id()
    }

    #[cfg(not(target_os = "windows"))]
    {
        stub::get_source_app_id()
    }
}

pub fn get_cover_bytes() -> Vec<u8> {
    #[cfg(target_os = "windows")]
    {
        windows::get_cover_bytes()
    }

    #[cfg(not(target_os = "windows"))]
    {
        stub::get_cover_bytes()
    }
}