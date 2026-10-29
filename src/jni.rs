use jni::objects::JClass;
use jni::sys::{jint, jstring};
use jni::JNIEnv;

use crate::smtc;

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_getSmtcInfo(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::logging::init();
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
    crate::logging::init();
    smtc::get_playback_status()
}

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_getSourceAppId(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::logging::init();
    let app = smtc::get_source_app_id();
    match env.new_string(&app) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 当前会话封面的 base64 编码（原始图片字节，通常 JPEG/PNG）；无封面时为空串。
#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_getCoverBase64(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::logging::init();
    use base64::Engine as _;
    let bytes = smtc::get_cover_bytes();
    let encoded = if bytes.is_empty() {
        String::new()
    } else {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    };
    match env.new_string(&encoded) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 采集运行标志与 JavaVM 缓存：采集线程需 Attach 到 JVM 后回调 Kotlin。
static JVM: std::sync::OnceLock<jni::JavaVM> = std::sync::OnceLock::new();

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_startAudioCapture(
    env: JNIEnv,
    _class: JClass,
    listener: jni::objects::JObject,
) -> jint {
    crate::logging::init();
    // 缓存 JVM，供采集线程 attach
    if JVM.get().is_none() {
        if let Ok(vm) = env.get_java_vm() {
            let _ = JVM.set(vm);
        }
    }
    // 生成全局引用，避免局部引用在回调线程失效
    let global = match env.new_global_ref(&listener) {
        Ok(g) => g,
        Err(_) => return -1,
    };

    let ret = crate::audio::start(move |pcm: &[i16]| {
        let Some(vm) = JVM.get() else { return };
        let Ok(mut guard) = vm.attach_current_thread() else { return };
        // 以 byte[] 传递给 Kotlin，长度按 16 位样本换算
        let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        let Ok(arr) = guard.byte_array_from_slice(&bytes) else { return };
        // 绑定为具名变量：JValue 借用其地址，临时值会在语句结束时被释放
        let arr_obj: jni::objects::JObject = arr.into();
        let payload = jni::objects::JValue::Object(&arr_obj);
        let _ = guard.call_method(&global, "onAudioPcm", "([B)V", &[payload]);
    });
    ret
}

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_stopAudioCapture(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    crate::logging::init();
    crate::audio::stop();
    0
}

#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_isAudioCapturing(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    crate::logging::init();
    if crate::audio::is_running() { 1 } else { 0 }
}

/// 采集流格式：返回 [采样率, 声道数]；未运行时为 [0, 0]。
#[no_mangle]
pub extern "system" fn Java_wtf_magma_audio_SMTCLoader_audioCaptureFormat(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jintArray {
    crate::logging::init();
    let (rate, channels) = crate::audio::format();
    let Ok(arr) = env.new_int_array(2) else { return std::ptr::null_mut() };
    let _ = env.set_int_array_region(&arr, 0, &[rate, channels]);
    arr.into_raw()
}