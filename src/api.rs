use crate::MediaSource;


pub struct AndroidPlayer<R: tauri::Runtime> {
    #[cfg(target_os = "android")]
    pub(crate) handle: tauri::plugin::PluginHandle<R>,

    #[cfg(not(target_os = "android"))]
    #[allow(unused)]
    pub(crate) handle: std::marker::PhantomData<fn() -> R>
}

impl<R: tauri::Runtime> AndroidPlayer<R> {

    /// Opens a player to play the specified videos
    /// and **waits until it closes**.
    /// 
    /// If you don’t need to wait until the video player closes,
    /// wrap this function with [`tauri::async_runtime::spawn`](https://docs.rs/tauri/2/tauri/async_runtime/fn.spawn.html).
    /// 
    /// If multiple videos are specified, they are played as a playlist.
    /// `options.initial_src_index` can be used to specify the video to play initially.
    /// 
    /// # Arguments
    /// - **src**: 
    /// Media sources of the videos to play. 
    /// An absolute file path, Android content URI, HTTP(S) URL, custom data provider, or similar source can be used.
    /// Supports common video and audio formats, as well as HLS and DASH. 
    /// See [supported-formats](https://developer.android.com/media/media3/exoplayer/supported-formats) for details.
    /// 
    /// - **options**:
    /// Optional settings: `initial_src_index`, `initial_src_time_ms`, `autoplay`, `keep_screen_on`, `prevent_screen_capture`, `reject_on_playback_error`. 
    /// See [`VideoPlayerOptions`] for details.
    /// 
    /// # Errors
    /// This method returns an error if an illegal value is passed as an argument,
    /// such as when `options.initial_src_index` is out of range, `src` is empty vec, and etc.
    ///
    /// Note that no error is returned
    /// if the media content specified by `src` cannot be played or `src` contains an invalid value. 
    /// Instead, an indication that an error occurred is displayed in the video player,
    /// and this method returns successfully when the video player is closed.
    /// To close the video player and return an error in such cases, 
    /// enable `options.reject_on_playback_error`.
    /// 
    /// # Android Version Behavior
    /// Nothing in particular.
    pub async fn open_video_player(
        &self,
        src: Vec<MediaSource>,
        options: VideoPlayerOptions
    ) -> Result<VideoPlayerResponse, crate::Error> {

        #[cfg(not(target_os = "android"))] {
            Err(crate::Error::NOT_ANDROID)
        }
        #[cfg(target_os = "android")] {
            crate::impls::start_video_player(src, options, &self.handle).await
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct VideoPlayerOptions {

    /// Indicates the index of the media source to start playback from.
    ///
    /// Defaults to `0`.
    #[serde(rename = "initialSrcIndex")]
    pub initial_src_index: Option<usize>,

    /// Indicates the time of the media source to start playback from.
    /// 
    /// Values outside the valid range are clamped.
    ///
    /// Defaults to `0`.
    #[serde(rename = "initialSrcTimeMs")]
    pub initial_src_time_ms: Option<u64>,

    /// Indicates whether to start playback automatically. 
    /// 
    /// Defaults to `true`.
    #[serde(rename = "autoplay")]
    pub autoplay: Option<bool>,

    /// Indicates whether to keep the screen on during playback.
    /// 
    /// Defaults to `true`.
    #[serde(rename = "keepScreenOn")]
    pub keep_screen_on: Option<bool>,

    /// Indicates whether to prevent screen capture of the video player.
    /// 
    /// See [`FLAG_SECURE`](https://developer.android.com/security/fraud-prevention/activities#flag_secure) for details.
    /// 
    /// If not specified, inherit the `FLAG_SECURE` state of the caller.
    #[serde(rename = "preventScreenCapture")]
    pub prevent_screen_capture: Option<bool>,

    /// Indicates whether to close the video player and return an error when playback fails.
    /// 
    /// Defaults to `false`.
    #[serde(rename = "rejectOnPlaybackError")]
    pub reject_on_playback_error: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VideoPlayerResponse {

    #[serde(rename = "lastSrcIndex")]
    pub last_src_index: usize,

    #[serde(rename = "lastSrcTimeMs")]
    pub last_src_time_ms: u64,
}