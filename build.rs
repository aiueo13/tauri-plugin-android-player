fn main() {
    tauri_plugin::Builder::new(&[
            "open_android_video_player"
        ])
        .android_path("android")
        .build();
}
