Note: **I’m using a translation tool, so some expressions may be awkward or inaccurate.**

# Overview
This plugin provides a video player based on `media3 ExoPlayer`.

![demo image](https://raw.githubusercontent.com/aiueo13/tauri-plugin-android-player/main/assets/video-player-demo.webp)

# Setup
First, install the plugin to your Tauri project:

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri-plugin-android-player = "0.2.1"
```

Next, register the plugin:

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

Then, configure the APIs that can be called from the frontend JavaScript bindings:

`src-tauri/capabilities/*.json`
```json
{
    "permissions": [
        "android-player:allow-open-android-video-player"
    ]
}
```

Finally, install the frontend JavaScript bindings:

```bash
pnpm add tauri-plugin-android-player-api@0.2.1 -E
# or
npm install tauri-plugin-android-player-api@0.2.1 --save-exact
# or
yarn add tauri-plugin-android-player-api@0.2.1 --exact
```

**NOTE**: Please ensure that the backend package, `tauri-plugin-android-player` (crates io), and the frontend package, `tauri-plugin-android-player-api` (npm), have exactly matching versions.

[![crates.io](https://img.shields.io/crates/v/tauri-plugin-android-palyer.svg?color=yellow)](https://crates.io/crates/tauri-plugin-android-player) [![npm version](https://img.shields.io/npm/v/tauri-plugin-android-player-api.svg?color=red)](https://www.npmjs.com/package/tauri-plugin-android-player-api)

# Usage

```typescript
import { openAndroidVideoPlayer } from "tauri-plugin-android-player-api";

await openAndroidVideoPlayer("https://developer.mozilla.org/shared-assets/videos/flower.mp4")
```

# License
This project is licensed under either of

 * MIT license
 * Apache License (Version 2.0)

at your option.