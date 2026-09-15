use std::{any::Any, io::{Read, Seek, SeekFrom}, sync::{Arc, Mutex as SyncMutex}};


pub struct MediaSource {
    pub(crate) repr: MediaSourceRepr
}

pub(crate) enum MediaSourceRepr {
    Uri(String),
    Path(std::path::PathBuf),
    CustomProvider(DataSourceOpener)
}

pub(crate) type DataSourceOpener = Box<dyn FnMut() -> Result<Box<dyn DataSource>, DataSourceError> + Send + 'static>;

impl MediaSource {

    fn new(repr: MediaSourceRepr) -> Self {
        Self { repr }
    }

    /// Creates a media source from a tauri filesystem path,
    /// such as [tauri_plugin_fs::FilePath](https://docs.rs/tauri-plugin-fs/latest/tauri_plugin_fs/enum.FilePath.html), [tauri_plugin_android_fs::FsUri](https://docs.rs/tauri-plugin-android-fs/latest/tauri_plugin_android_fs/struct.FsUri.html), and etc.
    pub fn from_tauri_fs_path(path: impl Into<tauri_plugin_fs::FilePath>) -> Self {
        match path.into() {
            tauri_plugin_fs::FilePath::Url(url) => Self::new(MediaSourceRepr::Uri(url.to_string())),
            tauri_plugin_fs::FilePath::Path(path) => Self::new(MediaSourceRepr::Path(path)),
        }
    }

    /// Creates a media source from a URI, such as HTTP(S) URL, Android Content URI, and etc.
    pub fn from_uri(uri: impl Into<String>) -> Self {
        Self::new(MediaSourceRepr::Uri(uri.into()))
    }

    /// Creates a media source from an absolute file path.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Self {
        Self::new(MediaSourceRepr::Path(path.as_ref().to_path_buf()))
    }

    /// Creates a media source from an opener function
    /// that opens an implementation of [`std::io::Read`] and [`std::io::Seek`].
    ///
    /// The total length is determined by calling [`Seek::seek`](std::io::Seek::seek)
    /// with `SeekFrom::End(0)` each time the implementation is opened.
    /// If the length is unknown or may change,
    /// use [`from_data_source_opener`](Self::from_data_source_opener) instead.
    ///
    /// The opener function must support being called multiple times.
    /// For example, when a player with multiple media sources switches to a different source,
    /// the current source may be closed.
    /// If the same source is selected again later, it may need to be opened again.
    /// Do not assume that it will be called only once,
    /// even if the player has only one media source.
    ///
    /// Each operation is executed on a dedicated Java thread
    /// for performing blocking operations.
    /// Therefore these operations are not executed within a Tauri's Tokio runtime context.
    /// To perform asynchronous operations that require a Tauri's Tokio runtime context,
    /// use [`tauri::async_runtime::block_on`](https://docs.rs/tauri/latest/tauri/async_runtime/fn.block_on.html)
    /// inside the implementation.
    pub fn from_read_seek_opener<T>(
        mut read_seek_opener: impl FnMut() -> Result<T, std::io::Error> + Send + 'static
    ) -> Self
    where 
        T: Read + Seek + Send + 'static,
    {
        Self::new(MediaSourceRepr::CustomProvider(Box::new(move || {
            let data_source = ReadSeekDataSource::new((read_seek_opener)()?)?;
            Ok(Box::new(data_source))
        })))
    }

    /// Creates a media source from an implementation of [`std::io::Read`] and [`std::io::Seek`].
    ///
    /// The total length is determined by calling [`Seek::seek`](std::io::Seek::seek) with `SeekFrom::End(0)`.
    /// If the length is unknown or may change,
    /// use [`from_data_source`](Self::from_data_source) instead.
    ///
    /// Each operation is executed on a dedicated Java thread
    /// for performing blocking operations.
    /// Therefore these operations are not executed within a Tauri's Tokio runtime context.
    /// To perform asynchronous operations that require a Tauri's Tokio runtime context,
    /// use [`tauri::async_runtime::block_on`](https://docs.rs/tauri/latest/tauri/async_runtime/fn.block_on.html)
    /// inside the implementation.
    pub fn from_read_seek<T>(read_seek: T) -> Self
    where 
        T: Read + Seek + Send + 'static,
    {
        let read_seek = Shareable::new(read_seek);
        Self::from_read_seek_opener(move || Ok(read_seek.clone()))
    }

    /// Creates a media source from an opener function
    /// that opens a [`DataSource`] implementation.
    ///
    /// The opener function must support being called multiple times.
    /// For example, when a player with multiple media sources switches to a different source,
    /// the current source may be closed.
    /// If the same source is selected again later, it may need to be opened again.
    /// Do not assume that it will be called only once,
    /// even if the player has only one media source.
    /// 
    /// Each operation is executed on a dedicated Java thread
    /// for performing blocking operations.
    /// Therefore these operations are not executed within a Tauri's Tokio runtime context.
    /// To perform asynchronous operations that require a Tauri's Tokio runtime context,
    /// use [`tauri::async_runtime::block_on`](https://docs.rs/tauri/latest/tauri/async_runtime/fn.block_on.html) 
    /// inside the implementations.
    pub fn from_data_source_opener<T>(
        mut data_source_opener: impl FnMut() -> Result<T, DataSourceError> + Send + 'static
    ) -> Self
    where 
        T: DataSource
    {
        Self::new(MediaSourceRepr::CustomProvider(Box::new(move || {
            let data_source = (data_source_opener)()?;
            Ok(Box::new(data_source))
        })))
    }

    /// Creates a media source from a [`DataSource`] implementation.
    /// 
    /// Each operation is executed on a dedicated Java thread
    /// for performing blocking operations.
    /// Therefore these operations are not executed within a Tauri's Tokio runtime context.
    /// To perform asynchronous operations that require a Tauri's Tokio runtime context,
    /// use [`tauri::async_runtime::block_on`](https://docs.rs/tauri/latest/tauri/async_runtime/fn.block_on.html) 
    /// inside the implementations.
    pub fn from_data_source<T>(data_source: T) -> Self
    where 
        T: DataSource
    {
        let data_source = Shareable::new(data_source);
        Self::from_data_source_opener(move || Ok(data_source.clone()))
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
        Self::new(MediaSourceRepr::Path(value))
    }
}

pub trait DataSource: Send + 'static {

    /// Reads up to `buf.len()` bytes of the content data beginning at `offset` into `buf`
    /// and returns the number of bytes read.
    ///
    /// Note that `offset` is an offset into the entire content data,
    /// **not** into `buf`.
    /// 
    /// Returns `Ok(0)` if the end of the data has been reached
    /// or if `offset` is out of range.
    /// Otherwise, this method must read at least one byte
    /// (an empty `buf` is never passed) or return an error.
    fn read(
        &mut self,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<usize, DataSourceError>;

    /// Returns the total length of this media source in bytes.
    ///
    /// Returns `Ok(None)` if the length is unknown.
    fn len(&mut self) -> Result<Option<u64>, DataSourceError>;
}

#[derive(Debug, thiserror::Error)]
#[error("data source error (code={error_code})")]
pub struct DataSourceError {
    error_code: i32,

    #[source]
    source: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl DataSourceError {

    
    /// Creates a data source error with the specified error code.
    ///
    /// Use one of the [`playback_error_code::IO_*`](crate::playback_error_code) constants as the error code.
    /// To use a custom error code, specify a value greater than
    /// [`playback_error_code::CUSTOM_BASE`](crate::playback_error_code::CUSTOM_BASE).
    pub fn new<E>(
        error_code: i32,
        error: E,
    ) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>
    {
        Self { error_code, source: error.into() }
    }

    pub fn io_unspecified<E>(error: E) -> Self
    where 
        E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>
    {
        Self::new(playback_error_code::IO_UNSPECIFIED, error)
    }

    pub fn error_code(&self) -> i32 {
        self.error_code
    }

    #[cfg(target_os = "android")]
    pub(crate) fn to_datasource_exception_message(&self) -> String {
        let mut msg = "Data source error".to_string();
        let mut src: Option<&dyn std::error::Error> = std::error::Error::source(self);
        while let Some(e) = src {
            msg.push_str("; ");
            msg.push_str(&e.to_string());
            src = e.source();
        }
        msg
    }

    #[cfg(target_os = "android")]
    pub(crate) fn panic(panic_payload: Box<dyn Any + Send + 'static>) -> Self {
        use crate::utils::{SyncWrapper, panic_payload_as_str};
        
        let panic = SyncWrapper::new(panic_payload);
        let msg = match panic_payload_as_str(&panic).map(|v| v.to_string()) {
            Some(msg) => format!("rust function panicked; {msg}"),
            None => format!("rust function panicked")
        };

        Self::io_unspecified(msg)
    }
}

impl From<std::io::Error> for DataSourceError {

    fn from(error: std::io::Error) -> Self {
        use std::io::ErrorKind as Ek;
        use playback_error_code as ec;

        let error_code = match error.kind() {
            Ek::NotFound => ec::IO_FILE_NOT_FOUND,
            Ek::PermissionDenied => ec::IO_NO_PERMISSION,
            Ek::ConnectionRefused
            | Ek::ConnectionReset
            | Ek::ConnectionAborted
            | Ek::HostUnreachable
            | Ek::NetworkUnreachable
            | Ek::NetworkDown
            | Ek::NotConnected => ec::IO_NETWORK_CONNECTION_FAILED,
            _ => ec::IO_UNSPECIFIED,
        };

        DataSourceError::new(error_code, error)
    }
}

/// Playback error code.
///
/// This is only a partial list.
/// For the full list, see
/// [media3 PlaybackException constants](https://developer.android.com/reference/androidx/media3/common/PlaybackException#constants).
pub mod playback_error_code {

    /// Error codes greater than this value are reserved for
    /// custom error codes defined by player implementations,
    /// so that they do not collide with the codes defined by media3.
    ///
    /// error code: 1000000
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#CUSTOM_ERROR_CODE_BASE()>
    pub const CUSTOM_BASE: i32 = 1000000;

    /// An I/O error whose cause could not be identified.
    ///
    /// error code: 2000
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_UNSPECIFIED()>
    pub const IO_UNSPECIFIED: i32 = 2000;

    /// The network connection could not be established.
    ///
    /// error code: 2001
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_NETWORK_CONNECTION_FAILED()>
    pub const IO_NETWORK_CONNECTION_FAILED: i32 = 2001;

    /// The network request timed out
    /// because the server took too long to respond.
    ///
    /// error code: 2002
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_NETWORK_CONNECTION_TIMEOUT()>
    pub const IO_NETWORK_CONNECTION_TIMEOUT: i32 = 2002;

    /// The server returned a resource
    /// whose "Content-Type" HTTP header value is invalid.
    ///
    /// error code: 2003
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_INVALID_HTTP_CONTENT_TYPE()>
    pub const IO_INVALID_HTTP_CONTENT_TYPE: i32 = 2003;

    /// The server returned an HTTP status code
    /// that the player does not expect.
    ///
    /// error code: 2004
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_BAD_HTTP_STATUS()>
    pub const IO_BAD_HTTP_STATUS: i32 = 2004;

    /// The file being accessed does not exist.
    ///
    /// error code: 2005
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_FILE_NOT_FOUND()>
    pub const IO_FILE_NOT_FOUND: i32 = 2005;

    /// The player lacks the permission required for the I/O operation.
    ///
    /// error code: 2006
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_NO_PERMISSION()>
    pub const IO_NO_PERMISSION: i32 = 2006;

    /// The player attempted to access cleartext HTTP traffic (http://),
    /// which the app's Network Security Configuration does not permit.
    ///
    /// error code: 2007
    ///
    /// <https://developer.android.com/reference/androidx/media3/common/PlaybackException#ERROR_CODE_IO_CLEARTEXT_NOT_PERMITTED()>
    pub const IO_CLEARTEXT_NOT_PERMITTED: i32 = 2007;
}


struct ReadSeekDataSource<C> {
    read_seek: C,
    len: u64,
}

impl<C: Read + Seek + Send + 'static> ReadSeekDataSource<C> {

    pub fn new(mut read_seek: C) -> std::io::Result<Self> {
        let len = with_retry_if_io_interrupted(|| read_seek.seek(SeekFrom::End(0)))?;
        Ok(Self { read_seek, len })
    }
}

impl<C: Read + Seek + Send + 'static> DataSource for ReadSeekDataSource<C> {

    fn read(
        &mut self,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<usize, DataSourceError> {

        // 範囲外の場合は 0 を返す
        if self.len <= offset {
            return Ok(0);
        }

        // 読み込み開始地点へ移動
        let pos = with_retry_if_io_interrupted(|| self.read_seek.stream_position())?;
        if offset != pos {
            with_retry_if_io_interrupted(|| self.read_seek.seek(SeekFrom::Start(offset)))?;
        }

        // 読み込む
        let mut totaln = 0;
        loop {
            let buf = &mut buf[totaln..];
            if buf.is_empty() {
                break;
            }

            let n = with_retry_if_io_interrupted(|| self.read_seek.read(buf))?;
            if n == 0 {
                break;
            }

            totaln += n;
        }

        Ok(totaln)
    }

    fn len(&mut self) -> Result<Option<u64>, DataSourceError> {
        Ok(Some(self.len))
    }
}

struct Shareable<T>(Arc<SyncMutex<T>>);

impl<T> Shareable<T> {

    pub fn new(read_seek: T) -> Self {
        Self(Arc::new(SyncMutex::new(read_seek)))
    }
}

impl<T> Clone for Shareable<T> {

    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Read> Read for Shareable<T> {

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.lock()
            .map_err(|_| std::io::Error::other("content poisoned"))?
            .read(buf)
    }
}

impl<T: DataSource> DataSource for Shareable<T> {

    fn read(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize, DataSourceError> {
        self.0.lock()
            .map_err(|_| std::io::Error::other("content poisoned"))?
            .read(offset, buf)
    }

    fn len(&mut self) -> Result<Option<u64>, DataSourceError> {
        self.0.lock()
            .map_err(|_| std::io::Error::other("content poisoned"))?
            .len()
    }
}


impl<T: Seek> Seek for Shareable<T> {

    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.0.lock()
            .map_err(|_| std::io::Error::other("content poisoned"))?
            .seek(pos)
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.0.lock()
            .map_err(|_| std::io::Error::other("content poisoned"))?
            .stream_position()
    }
}

fn with_retry_if_io_interrupted<F, T>(mut operation: F) -> std::io::Result<T>
where
    F: FnMut() -> std::io::Result<T>,
{
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}