use std::{fmt::Display, io::{Read, Seek, SeekFrom}, num::NonZeroUsize};


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
    /// Optional settings: `initial_src_index`, `initial_src_time_ms`, `autoplay`, `keep_screen_on`, `prevent_screen_capture`. 
    /// See [`VideoPlayerOptions`] for details.
    /// 
    /// # Errors
    /// This method returns an error if an invalid value is passed as an argument,
    /// such as when `options.initial_src_index` is out of range.
    ///
    /// Note that no error is returned
    /// if the media content specified by `src` cannot be played or `src` is an invalid value. 
    /// Instead, an error message is displayed in the video player, 
    /// and this method returns successfully when the video player is closed.
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
}

#[derive(Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VideoPlayerResponse {

    #[serde(rename = "lastSrcIndex")]
    pub last_src_index: usize,

    #[serde(rename = "lastSrcTimeMs")]
    pub last_src_time_ms: u64,
}

pub trait MediaSourceReader: Send + 'static {

    /// Reads and returns up to `len` bytes of data starting from `offset`.
    /// 
    /// Returns empty vec only if the end of the data is reached or if `offset` is out of range.
    fn read(
        &mut self,
        offset: u64,
        len: NonZeroUsize
    ) -> Result<Vec<u8>, String>;

    /// Returns the total length of this media source in bytes.
    /// 
    /// Returns None if the length is unknown.
    fn len(&mut self) -> Result<Option<u64>, String>;
}

pub enum MediaSource {

    /// Media source from a URI, such as HTTP(S) URL, Android Content URI, and etc.
    Uri(String),

    /// Media source from an absolute file path.
    Path(std::path::PathBuf),

    /// Media source from a opener function
    /// that opens a [MediaSourceReader] implementation.
    /// 
    /// The opener function must support being called multiple times.
    /// For example, when a player with multiple media sources switches to a different source, 
    /// the current media source may be closed.
    /// If the same media source is selected again later, it may need to be opened again.
    /// 
    /// Each operation is executed on a dedicated thread, where blocking operations are allowed.
    /// These operations are not executed within a Tokio runtime context.
    /// For asynchronous operations within a Tokio runtime context,
    /// use [`tauri::async_runtime::block_on`](https://docs.rs/tauri/latest/tauri/async_runtime/fn.block_on.html) internally.
    CustomProvider(Box<dyn FnMut() -> Result<Box<dyn MediaSourceReader>, String> + Send + 'static>)
}

impl MediaSource {

    pub fn from_tauri_fs_path(path: impl Into<tauri_plugin_fs::FilePath>) -> Self {
        match path.into() {
            tauri_plugin_fs::FilePath::Url(url) => Self::Uri(url.to_string()),
            tauri_plugin_fs::FilePath::Path(path) => Self::Path(path),
        }
    }

    pub fn from_uri(uri: impl Into<String>) -> Self {
        Self::Uri(uri.into())
    }

    pub fn from_path(path: impl AsRef<std::path::Path>) -> Self {
        Self::Path(path.as_ref().to_path_buf())
    }

    /// Creates a [`MediaSource`] from a opener function
    /// that opens a [`std::io::Read`] and [`std::io::Seek`] implementation.
    /// 
    /// The opener function must support being called multiple times.
    /// For example, when a player with multiple media sources switches to a different source, 
    /// the current media source may be closed.
    /// If the same media source is selected again later, it may need to be opened again.
    /// 
    /// Each operation is executed on a dedicated thread, where blocking operations are allowed.
    /// These operations are not executed within a Tokio runtime context.
    /// For asynchronous operations within a Tokio runtime context, 
    /// use [`tauri::async_runtime::block_on`](https://docs.rs/tauri/latest/tauri/async_runtime/fn.block_on.html) internally.
    pub fn from_read_seek<T, E>(mut read_seek_opener: impl FnMut() -> Result<T, E> + Send + 'static) -> Self
    where 
        T: Read + Seek + Send + 'static,
        E: Display
    {
        struct Impl<C>(C);
        impl<C: Read + Seek + Send + 'static> MediaSourceReader for Impl<C> {
            fn read(
                &mut self,
                offset: u64,
                len: NonZeroUsize
            ) -> Result<Vec<u8>, String> {

                let total_len = self.0.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
                if total_len <= offset {
                    // 範囲外
                    return Ok(Vec::new());
                }
                
                let mut buf = Vec::new();
                self.0.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
                (&mut self.0).take(len.get() as u64).read_to_end(&mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }

            fn len(&mut self) -> Result<Option<u64>, String> {
                self.0.seek(SeekFrom::End(0))
                    .map(|v| Some(v))
                    .map_err(|e| e.to_string())
            }
        }

        Self::CustomProvider(Box::new(move || {
            Ok(Box::new(Impl((read_seek_opener)().map_err(|e| e.to_string())?)))
        }))
    }
}

impl From<tauri_plugin_fs::FilePath> for MediaSource {

    fn from(value: tauri_plugin_fs::FilePath) -> Self {
        Self::from_tauri_fs_path(value)
    }
}

impl From<tauri_plugin_fs::SafeFilePath> for MediaSource {

    fn from(value: tauri_plugin_fs::SafeFilePath) -> Self {
        Self::from_tauri_fs_path(value)
    }
}

impl From<std::path::PathBuf> for MediaSource {

    fn from(value: std::path::PathBuf) -> Self {
        Self::Path(value)
    }
}