Note: **I’m using a translation tool, so some expressions may be awkward or inaccurate.**

# Overview
This plugin provides a video player based on `media3 ExoPlayer`.

The player accepts HTTP(S) URLs, file paths, Android Content URIs, FsUri values used by [tauri_plugin_android_fs](https://crates.io/crates/tauri-plugin-android-fs), custom data providers, and other sources.
It supports common video and audio formats, as well as HLS and DASH. See [supported-formats](https://developer.android.com/media/media3/exoplayer/supported-formats) for details.

![demo image](https://raw.githubusercontent.com/aiueo13/tauri-plugin-android-player/main/assets/video-player-demo.webp)

**NOTE**: This page explains how to use this plugin in the Rust backend. For JavaScript bindings in the frontend, see [tauri-plugin-android-player-api on npm](https://www.npmjs.com/package/tauri-plugin-android-player-api?activeTab=readme).

# Setup

Register this plugin in your Tauri project:

`src-tauri/src/lib.rs`

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_android_player::init()) // This
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

# Usage
```rust
use tauri_plugin_android_player::{AndroidPlayerExt, MediaSource};

async fn example<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let api = app.android_player();
    let src = MediaSource::from_uri("https://developer.mozilla.org/shared-assets/videos/flower.mp4");

    api.open_video_player(
        vec![src], 
        Default::default()
    ).await.unwrap();
}
```

Use `MediaSource` to specify the media content to be played.
The following APIs are available for specifying the content:

| API | Description |
| --- | --- |
| [MediaSource::from_uri](https://docs.rs/tauri-plugin-android-player/latest/tauri_plugin_android_player/enum.MediaSource.html#method.from_uri) | Specifies content from a URI. |
| [MediaSource::from_path](https://docs.rs/tauri-plugin-android-player/latest/tauri_plugin_android_player/enum.MediaSource.html#method.from_path) | Specifies content from an absolute file path. |
| [MediaSource::from_tauri_fs_path](https://docs.rs/tauri-plugin-android-player/latest/tauri_plugin_android_player/enum.MediaSource.html#method.from_tauri_fs_path) | Specifies content from a [tauri_plugin_fs::FilePath](https://docs.rs/tauri-plugin-fs/latest/tauri_plugin_fs/enum.FilePath.html), [tauri_plugin_android_fs::FsUri](https://docs.rs/tauri-plugin-android-fs/latest/tauri_plugin_android_fs/struct.FsUri.html), or other supported tauri filesystem path types. |
| [MediaSource::from_read_seek](https://docs.rs/tauri-plugin-android-player/latest/tauri_plugin_android_player/enum.MediaSource.html#method.from_read_seek), [MediaSource::from_read_seek_opener](https://docs.rs/tauri-plugin-android-player/latest/tauri_plugin_android_player/enum.MediaSource.html#method.from_read_seek_opener) | Specifies arbitrary content data provided through [std::io::Read](https://doc.rust-lang.org/std/io/trait.Read.html) and [std::io::Seek](https://doc.rust-lang.org/std/io/trait.Seek.html). |


# License
This project is licensed under either of

 * MIT license
 * Apache License (Version 2.0)

at your option.