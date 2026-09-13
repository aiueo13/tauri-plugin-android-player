use crate::{AndroidPlayerExt, MediaSource, VideoPlayerOptions, VideoPlayerResponse};


#[tauri::command]
pub async fn open_android_video_player<R: tauri::Runtime>(
    src: Vec<tauri_plugin_fs::SafeFilePath>,
    options: Option<VideoPlayerOptions>,
    app: tauri::AppHandle<R>,
) -> Result<VideoPlayerResponse, crate::Error> {

    app.android_player().open_video_player(
        src.into_iter().map(MediaSource::from_tauri_fs_path).collect(),
        options.unwrap_or_default()
    ).await
}