# smtc_jni

**English** · [中文](README.zh.md)

> Rust-powered JNI bridge between your Java app and the Windows **System Media Transport Controls** (SMTC) — read the current media session without leaving the JVM.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)]()
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)]()

---

## Overview

`smtc_jni` is a native (`cdylib`) Rust library that exposes the current media session — title, artist, play position, duration, and playback state — to Java through JNI. It reads data straight from the Windows `GlobalSystemMediaTransportControlsSessionManager`, so it mirrors whatever music app currently owns the media session (Spotify, Groove, a browser, etc.).

## Features

- **Media metadata** — live title & artist of the active session
- **Timeline** — current position and total duration in milliseconds
- **Playback state** — maps `MediaPlaybackStatus` to an easy-to-read `int`
- **Source detection** — the owning app's User Model ID
- **Zero-panic native code** — every call degrades gracefully, never faults into the JVM
- **Cross-platform safety** — non-Windows builds fall back to a stub instead of failing

## Output Format

`nativeGetSmtcInfo()` returns a pipe-delimited string:

```
title|artist|pos_ms|dur_ms
```

| Field   | Type  | Description                          |
|---------|-------|--------------------------------------|
| title   | str   | Media title                          |
| artist  | str   | Artist name                          |
| pos_ms  | int64 | Current playback position (ms)       |
| dur_ms  | int64 | Total duration (ms)                  |

> When no session is active the call returns the sentinel `"No media"`.

## Usage

Java side — declare a matching `native` class. The method names must follow JNI convention for `wtf.magma.audio.SMTCLoader`:

```java
public final class SMTCLoader {

    /** @return "title|artist|pos_ms|dur_ms", or "No media" */
    public static native String getSmtcInfo();

    /**
     * @return Windows MediaPlaybackStatus:
     *         0 Closed · 1 Opened · 2 Changing · 3 Playing · 4 Paused · 5 Stopped
     */
    public static native int getPlaybackStatus();

    /** @return owning app's User Model ID (may be empty) */
    public static native String getSourceAppId();

    static {
        System.loadLibrary("smtc_jni");
    }
}
```

Load the library once (e.g. in a static initializer) and call the natives whenever you need a snapshot.

## Building

Build a release DLL from the crate root:

```bash
cargo build --release
```

Output: `target/release/smtc_jni.dll` — drop this onto the JVM's library path.

### Requirements

| Tool / Crate        | Version / Notes                              |
|---------------------|----------------------------------------------|
| Rust toolchain      | edition 2021                                 |
| Windows SDK         | required for the SMTC API                    |
| `windows` crate     | `0.58` with features `Foundation`, `Media_Control` |
| JNI runtime         | any JDK 8+ at runtime                        |

## Platform Support

| Platform | Native implementation         | Behavior                                |
|----------|-------------------------------|-----------------------------------------|
| Windows  | Full SMTC via `GSMTCSessionManager` | Real data from the active session |
| Others   | Compile-time stub              | Returns `"No media"`, status `0`, empty app ID |

The build is guarded by `#[cfg(target_os = "windows")]`, so non-Windows targets still compile — they just report no media.

## Architecture

```
lib.rs        → module roots (jni, smtc)
├── jni.rs    → #[no_mangle] extern "system" exports mapped to JNI symbols
├── smtc/
│   ├── mod.rs   → platform-dispatch layer (cfg-gated)
│   ├── windows.rs → real Windows implementation
│   └── stub.rs  → non-Windows fallback
```

The JNI symbol names derive from the Java package + class (`Java_wtf_magma_audio_SMTCLoader_...`). If you rename the Java class, keep the JNI signatures in `jni.rs` in sync.

## License

Released under the [MIT License](LICENSE).