#![cfg(target_os = "android")]
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;
use std::panic;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex as SyncMutex;
use jni::objects::AutoElements;
use jni::objects::JThrowable;
use jni::objects::JValue;
use jni::objects::ReleaseMode;
use jni::objects::{JByteArray, JClass};
use jni::JNIEnv;
use jni::sys::jbyte;
use jni::sys::jint;
use jni::sys::jlong;
use crate::utils::ScopeGuard;
use crate::*;


pub const PLUGIN_PACKAGE_NAME: &str = "okayu.tauri.plugin.android.player";
pub const PLUGIN_MAIN_CLASS_NAME: &str = "PlayerPlugin";

const MEDIA_STREAM_EXCEPTION_CLASS: &str = "okayu/tauri/plugin/android/player/MediaStreamBridge$MediaStreamException";

/// 指定した stream に対する興味が失われたことを示す。
/// これにより stream のリソースが解放される可能性があるが、
/// 解放されても len や read を呼ぶことで再び開かれる。
/// stream はこのメソッドに依存せず close される。
/// よってこれはあくまで最適化のためであり、呼ぶことが必須ではない。
/// 
/// このメソッドは stream が存在しなくてもエラーは発生せず、ブロックも行われずにすぐに終了する。
#[no_mangle]
extern "system" fn Java_okayu_tauri_plugin_android_player_MediaStreamBridge_deactivate<'l>(
    env: JNIEnv<'l>,
    _class: JClass<'l>,
    stream_id: jint,
) {

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        media_stream::deactivate_reader(stream_id)
    }))
    .map_err(DataSourceError::panic);

    match result {
        Ok(()) => (),
        Err(err) => throw_media_stream_exception(env, err),
    }
}

/// 長さが不明の場合は -1 を返す。
#[no_mangle]
extern "system" fn Java_okayu_tauri_plugin_android_player_MediaStreamBridge_len<'l>(
    env: JNIEnv<'l>,
    _class: JClass<'l>,
    stream_id: jint,
) -> jlong {

    let result = panic::catch_unwind(AssertUnwindSafe(|| 
        media_stream::use_reader(stream_id, |reader| {
            let len = reader
                .len()?
                .and_then(|len| len.try_into().ok())
                .unwrap_or(-1);

            Ok(len)
        })
    ))
    .map_err(DataSourceError::panic)
    .and_then(|v| v);

    match result {
        Ok(len) => len,
        Err(err) => {
            throw_media_stream_exception(env, err);
            Default::default()
        }
    }
}

/// # SAFETY
/// buf は有効な Java byte 配列へのローカル参照である。
/// 実行中に他の Rust thread や Java thread から buf にアクセスされない。
/// 同じ buf に対して、この関数が同時に複数回呼ばれない。
#[no_mangle]
unsafe extern "system" fn Java_okayu_tauri_plugin_android_player_MediaStreamBridge_read<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    stream_id: jint,
    offset: jlong,
    len: jint,
    buf: JByteArray,
    buf_offset: jint,
) -> jint {

    let result = panic::catch_unwind(AssertUnwindSafe(|| 
        media_stream::use_reader(stream_id, |reader| {
            let offset = try_into::<jlong, u64>(offset, "offset")?;
            let len = try_into::<jint, usize>(len, "len")?;
            let buf_offset = try_into::<jint, usize>(buf_offset, "buf_offset")?;

            // SAFETY:
            // 存在期間中に他の Rust thread や Java thread から同じ配列にアクセスされず、
            // 同じ配列に対する ByteArrayBuf が複数存在しない。
            let mut buf = unsafe { ByteArrayBuf::new(&mut env, &buf) }?;

            let buf_end = buf_offset
                .checked_add(len)
                .ok_or_else(|| DataSourceError::io_unspecified("buffer range is too large"))?;

            if buf.len() < buf_end {
                return Err(DataSourceError::io_unspecified("buffer range is out of bounds"));
            }
            if len == 0 {
                return Ok(0);
            }

            let nread = reader.read(offset, &mut buf[buf_offset..buf_end])?;
            let nread = try_into::<usize, jint>(nread, "nread")?;
            Ok(nread)
        })
    ))
    .map_err(DataSourceError::panic)
    .and_then(|v| v);

    match result {
        Ok(nread) => nread,
        Err(err) => {
            throw_media_stream_exception(env, err);
            Default::default()
        }
    }
}

fn try_into<T: Display + Copy + TryInto<R>, R>(
    value: T,
    value_name: impl Display
) -> Result<R, DataSourceError> {

    value.try_into().map_err(|_| DataSourceError::io_unspecified(
        format!("illegal {value_name}: {value}")
    ))
}

fn throw_media_stream_exception<'local>(
    mut env: JNIEnv<'local>,
    err: DataSourceError,
) {

    if env.exception_check().unwrap_or(true) {
        return;
    }

    let result = (|| -> jni::errors::Result<()> {
        let msg = env.new_string(err.to_datasource_exception_message())?;
        let exception = env.new_object(
            MEDIA_STREAM_EXCEPTION_CLASS,
            "(Ljava/lang/String;I)V",
            &[JValue::Object(&msg), JValue::Int(err.error_code())],
        )?;
        let _ = env.throw(JThrowable::from(exception));
        Ok(())
    })();

    // クロージャを抜けて env の借用が解けてからフォールバック
    if let Err(e) = result {
        if !env.exception_check().unwrap_or(true) {
            let _ = env.throw(format!("Failed to create MediaStreamException: {e}"));
        }
    }
}

struct ByteArrayBuf<'local, 'other_local, 'env>(
    AutoElements<'local, 'other_local, 'env, jbyte>,
);

impl<'local, 'other_local, 'env> ByteArrayBuf<'local, 'other_local, 'env> {

    /// # SAFETY
    /// 返される ByteArrayBuf の存在期間中に他の Rust thread や Java thread から同じ配列にアクセスされず、
    /// 同じ配列に対する ByteArrayBuf が複数存在しない。
    pub unsafe fn new(
        env: &'env mut JNIEnv<'local>,
        array: &'other_local JByteArray<'other_local>,
    ) -> Result<Self, DataSourceError> {

        let array = env
            .get_array_elements(&array, ReleaseMode::CopyBack)
            .map_err(|err| DataSourceError::io_unspecified(err))?;

        Ok(Self(array))
    }
}

impl Deref for ByteArrayBuf<'_, '_, '_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        bytemuck::cast_slice(&self.0)
    }
}

impl DerefMut for ByteArrayBuf<'_, '_, '_> {

    fn deref_mut(&mut self) -> &mut [u8] {
        bytemuck::cast_slice_mut(&mut self.0)
    }
}

mod media_stream {
    use std::sync::TryLockError;

    use super::*;

    pub type DataSourceOpener = Box<dyn FnMut() -> Result<Box<dyn DataSource>, DataSourceError> + Send + 'static>;
    
    struct MediaStream {
        opener: DataSourceOpener,
        reader: Option<Box<dyn DataSource>>
    }

    struct MediaStreams {
        map: HashMap<i32, Arc<SyncMutex<MediaStream>>>,
        next_id: i32,
    }

    static MEDIA_STREAMS: LazyLock<SyncMutex<MediaStreams>> = LazyLock::new(|| {
        SyncMutex::new(MediaStreams { map: HashMap::new(), next_id: 0 })
    });

    fn get_stream(stream_id: i32) -> Result<Arc<SyncMutex<MediaStream>>, DataSourceError> {
        MEDIA_STREAMS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .map
            .get(&stream_id)
            .map(Arc::clone)
            .ok_or_else(|| DataSourceError::io_unspecified("Missing stream"))
    }
    
    pub fn add(opener: DataSourceOpener) -> i32 {
        let mut locked_streams = MEDIA_STREAMS.lock().unwrap_or_else(|e| e.into_inner());
        let stream = Arc::new(SyncMutex::new(MediaStream { opener, reader: None }));
        let stream_id = loop {
            let id = locked_streams.next_id;
            locked_streams.next_id = locked_streams.next_id.wrapping_add(1);
            if !locked_streams.map.contains_key(&id) {
                break id;
            }
        };
        locked_streams.map.insert(stream_id, stream);
        stream_id
    }

    pub fn close(stream_ids: impl IntoIterator<Item = i32>) {
        let mut streams = MEDIA_STREAMS.lock().unwrap_or_else(|e| e.into_inner());
        for stream_id in stream_ids {
            streams.map.remove(&stream_id);
        }
    }

    pub fn use_reader<F, T>(
        stream_id: i32,
        f: F
    ) -> Result<T, DataSourceError>
    where
        F: FnOnce(&mut dyn DataSource) -> Result<T, DataSourceError>
    {

        let stream = get_stream(stream_id)?;
        let (mut locked_stream, poisoned) = stream
            .lock()
            .map(|v| (v, false))
            .unwrap_or_else(|e| (e.into_inner(), true));

        if poisoned {
            // 壊れた可能性のある reader は念のために opener 実行前に捨てる
            locked_stream.reader = None;
        }
        if locked_stream.reader.is_none() {
            locked_stream.reader = Some((locked_stream.opener)()?);
        }
        if poisoned {
            stream.clear_poison();
        }

        let reader = locked_stream.reader
            .as_deref_mut()
            .expect("should set reader before");

        (f)(reader)
    }

    pub fn deactivate_reader(stream_id: i32) {
        let Ok(stream) = get_stream(stream_id) else {
            return
        };

        match stream.try_lock() {
            Ok(mut locked_stream) => {
                locked_stream.reader = None;
                return
            },
            Err(TryLockError::Poisoned(err)) => {
                err.into_inner().reader = None;
                return
            },
            Err(TryLockError::WouldBlock) => {
                // lock の scope を抜けてから処理
            }
        }

        // deactivate は Java main thread からも呼ばれる可能性があるので block しないようにする。
        tauri::async_runtime::spawn_blocking(move || {
            let mut locked_stream = stream.lock().unwrap_or_else(|e| e.into_inner());
            locked_stream.reader = None;
        });
    }
}

pub async fn start_video_player<R: tauri::Runtime>(
    src: Vec<MediaSource>,
    options: VideoPlayerOptions,
    handle: &tauri::plugin::PluginHandle<R>,
) -> Result<VideoPlayerResponse, Error> {

    let (src, src_type, src_streams_guard) = {
        let mut src_buffer = Vec::new();
        let mut src_type_buffer = Vec::new();
        let mut src_stream_ids_buffer = Vec::new();
        for s in src {
            match s.repr {
                MediaSourceRepr::Uri(uri) => {
                    src_buffer.push(uri);
                    src_type_buffer.push("Uri");
                },
                MediaSourceRepr::Path(path) => {
                    src_buffer.push(path.to_string_lossy().to_string());
                    src_type_buffer.push("Path");
                },
                MediaSourceRepr::CustomProvider(opener) => {
                    let stream_id = media_stream::add(opener);
                    src_stream_ids_buffer.push(stream_id);
                    src_buffer.push(stream_id.to_string());
                    src_type_buffer.push("StreamId");
                }
            }
        }
        let src_streams_guard = ScopeGuard::new(|| {
            media_stream::close(src_stream_ids_buffer);
        });

        (src_buffer, src_type_buffer, src_streams_guard)
    };

    let result = handle.run_mobile_plugin_async::<VideoPlayerResponse>(
        "startVideoPlayer",
        serde_json::json!({
            "src": src,
            "srcType": src_type,
            "options": options
        })
    ).await;

    // streams を破棄
    drop(src_streams_guard);

    // Activity の終了後にすぐに frontend 側に戻ると、
    // その frontend 側の関数の呼び出しが終了しないことが偶にある。
    // よって遅延を強制的に追加してこれを回避する。
    crate::utils::sleep(std::time::Duration::from_millis(200)).await;

    result.map_err(Into::into)
}