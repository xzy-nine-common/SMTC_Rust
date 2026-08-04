# smtc_jni

[English](README.md) · **中文**

> 用 Rust 编写的 JNI 桥接库，连接你的 Java 应用与 Windows **系统媒体传输控件**（SMTC）——无需离开 JVM 即可读取当前媒体会话。

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)]()
[![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)]()

---

## 概述

`smtc_jni` 是一个原生（`cdylib`）Rust 库，通过 JNI 向 Java 暴露当前媒体会话信息——标题、歌手、播放位置、时长和播放状态。它直接读取 Windows `GlobalSystemMediaTransportControlsSessionManager` 的数据，因此镜像了当前拥有媒体会话的任何音乐应用（Spotify、Groove、浏览器等）。

## 功能特性

- **媒体元数据** — 活跃会话的实时标题与歌手
- **时间线** — 以毫秒为单位的当前位置与总时长
- **播放状态** — 将 `MediaPlaybackStatus` 映射为易于读取的 `int`
- **来源识别** — 所属应用的 User Model ID
- **零崩溃原生代码** — 每次调用均优雅降级，绝不向 JVM 传递异常
- **跨平台安全** — 非 Windows 构建回退到空桩实现而非报错

## 输出格式

`nativeGetSmtcInfo()` 返回以竖线分隔的字符串：

```
title|artist|pos_ms|dur_ms
```

| 字段   | 类型   | 说明                     |
|--------|--------|--------------------------|
| title  | str    | 媒体标题                 |
| artist | str    | 歌手名称                 |
| pos_ms | int64  | 当前播放位置（毫秒）     |
| dur_ms | int64  | 总时长（毫秒）           |

> 当没有活跃会话时，调用返回哨兵字符串 `"No media"`。

## 使用方法

Java 侧需要声明一个与之匹配的 `native` 类。方法名必须遵循 JNI 对 `wtf.magma.audio.SMTCLoader` 的命名约定：

```java
public final class SMTCLoader {

    /** @return "title|artist|pos_ms|dur_ms"，或 "No media" */
    public static native String nativeGetSmtcInfo();

    /**
     * @return Windows MediaPlaybackStatus：
     *         0 Closed · 1 Opened · 2 Changing · 3 Playing · 4 Paused · 5 Stopped
     */
    public static native int nativeGetPlaybackStatus();

    /** @return 所属应用的 User Model ID（可能为空） */
    public static native String nativeGetSourceAppId();

    static {
        System.loadLibrary("smtc_jni");
    }
}
```

在静态初始化块中加载一次库，之后在需要快照时调用这些 native 方法即可。

## 构建

在 crate 根目录构建 release DLL：

```bash
cargo build --release
```

输出：`target/release/smtc_jni.dll`——将它放到 JVM 的库路径上即可。

### 依赖要求

| 工具 / crate      | 版本 / 说明                                 |
|-------------------|---------------------------------------------|
| Rust 工具链       | edition 2021                                |
| Windows SDK       | SMTC API 所需                               |
| `windows` crate   | `0.58`，启用特性 `Foundation`、`Media_Control` |
| JNI 运行时        | 运行时需要 JDK 8+                            |

## 平台支持

| 平台    | 原生实现                   | 行为                              |
|---------|----------------------------|-----------------------------------|
| Windows | 完整 SMTC，基于 `GSMTCSessionManager` | 读取活跃会话的真实数据 |
| 其他    | 编译期空桩                 | 返回 `"No media"`、状态 `0`、空应用 ID |

构建由 `#[cfg(target_os = "windows")]` 守卫，因此非 Windows 目标依然可以编译——只是报告无媒体。

## 架构

```
lib.rs        → 模块根（jni, smtc）
├── jni.rs    → #[no_mangle] extern "system" 导出，映射到 JNI 符号
├── smtc/
│   ├── mod.rs   → 平台分发层（cfg 守卫）
│   ├── windows.rs → Windows 真实实现
│   └── stub.rs  → 非 Windows 回退
```

JNI 符号名由 Java 包名 + 类名推导而来（`Java_wtf_magma_audio_SMTCLoader_...`）。如果重命名 Java 类，请同步更新 `jni.rs` 中的 JNI 签名。

## 许可证

基于 [MIT License](LICENSE) 发布。