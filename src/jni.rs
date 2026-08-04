use jni::objects::JClass;
use jni::sys::{jint, jstring};
use jni::JNIEnv;

use crate::smtc;

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_getSmtcInfo(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let info = smtc::get_smtc_info();
    match env.new_string(&info) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_getPlaybackStatus(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    smtc::get_playback_status()
}

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_getSourceAppId(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let app = smtc::get_source_app_id();
    match env.new_string(&app) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}