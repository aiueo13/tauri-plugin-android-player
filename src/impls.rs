#![cfg(target_os = "android")]
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex as SyncMutex;
use jni::objects::JObject;
use jni::objects::{JByteArray, JClass, JString};
use jni::JNIEnv;
use jni::sys::jint;
use jni::sys::jlong;
use crate::*;


pub const PLUGIN_PACKAGE_NAME: &str = "okayu.tauri.plugin.android.player";
pub const PLUGIN_MAIN_CLASS_NAME: &str = "PlayerPlugin";

#[no_mangle]
extern "system" fn Java_okayu_tauri_plugin_android_player_MediaStreamBridge_activate<'l>(
    env: JNIEnv<'l>,
    _class: JClass<'l>,
    id: jint,
) -> JString<'l> {

    match media_stream_bridge_impl::activate(id) {
        Ok(len) => env.new_string(format!("ok:{len}")).unwrap_or_default(),
        Err(e) => env.new_string(format!("err:{e}")).unwrap_or_default(),
    }
}

#[no_mangle]
extern "system" fn Java_okayu_tauri_plugin_android_player_MediaStreamBridge_deactivate<'l>(
    env: JNIEnv<'l>,
    _class: JClass<'l>,
    id: jint,
) -> JString<'l> {

    match media_stream_bridge_impl::deactivate(id) {
        Ok(_) => JObject::null().into(),
        Err(e) => env.new_string(e).unwrap_or_default()
    }
}

#[no_mangle]
extern "system" fn Java_okayu_tauri_plugin_android_player_MediaStreamBridge_read<'l>(
    env: JNIEnv<'l>,
    _class: JClass<'l>,
    id: jint,
    offset: jlong,
    len: jint,
) -> JByteArray<'l> {

    let offset = offset as u64;
    let len = len as usize;
    let data = match media_stream_bridge_impl::read(id, offset, len) {
        Ok(mut data) => {
            // 最後のバイトを 0 にして正常に読み込めたことを示す。
            data.reserve_exact(1);
            data.push(0);
            data
        },
        Err(err) => {
            let mut data = err.into_bytes();
            // 最後のバイトを 1 にして正常に読み込めなかったため utf8 形式のエラー文が入っていることを示す。
            data.reserve_exact(1);
            data.push(1);
            data
        },
    };

    env.byte_array_from_slice(&data).unwrap_or_default()
}

// MEDIA_STREAMS 自体のロックが長時間保持されることはなく、
// 個々の MediaStream が同時にアクセスされることもないので両方とも SyncMutex を用いる。
static MEDIA_STREAMS: LazyLock<SyncMutex<HashMap<i32, Arc<SyncMutex<MediaStream>>>>> = LazyLock::new(|| {
    SyncMutex::new(HashMap::new())
});

fn next_media_stream_id() -> i32 {
    static NEXT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

mod media_stream_bridge_impl {
    use super::*;

    fn get_stream(stream_id: i32) -> Result<Arc<SyncMutex<MediaStream>>, String> {
        MEDIA_STREAMS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&stream_id)
            .map(Arc::clone)
            .ok_or_else(|| "stream id not found".into())
    }

    pub fn activate(stream_id: i32) -> Result<i64, String> {
        let stream = get_stream(stream_id)?;
        let mut stream = stream.lock().unwrap();
        if stream.reader.is_none() {
            stream.reader = Some((stream.opener)()?);
        }

        let len = stream.reader.as_mut().unwrap().len()?
            .and_then(|v| v.try_into().ok())
            .unwrap_or(-1);

        Ok(len)
    }

    pub fn deactivate(stream_id: i32) -> Result<(), String> {
        let stream = get_stream(stream_id)?;
        let mut stream = stream.lock().unwrap();
        stream.reader = None;
        Ok(())
    }

    pub fn read(
        stream_id: i32,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, String> {

        let stream = get_stream(stream_id)?;
        let mut stream = stream.lock().unwrap();
        let reader = stream.reader.as_mut().ok_or_else(|| "missing stream")?;
        
        if let Some(len) = NonZeroUsize::new(len) {
            reader.read(offset, len)
        }
        else {
            Ok(Vec::new())
        }
    }
}

pub async fn start_video_player<R: tauri::Runtime>(
    src: Vec<MediaSource>,
    options: VideoPlayerOptions,
    handle: &tauri::plugin::PluginHandle<R>,
) -> Result<VideoPlayerResponse, Error> {

    let (src, src_type, src_stream_ids) = {
        let mut src_buffer = Vec::new();
        let mut src_type_buffer = Vec::new();
        let mut src_stream_ids_buffer = Vec::new();
        for s in src {
            match s {
                MediaSource::Uri(uri) => {
                    src_buffer.push(uri);
                    src_type_buffer.push("Uri");
                },
                MediaSource::Path(path) => {
                    src_buffer.push(path.to_string_lossy().to_string());
                    src_type_buffer.push("Path");
                },
                MediaSource::CustomProvider(opener) => {
                    let stream_id = next_media_stream_id();
                    MEDIA_STREAMS.lock().unwrap_or_else(|e| e.into_inner()).insert(
                        stream_id.clone(),
                        Arc::new(SyncMutex::new(MediaStream::new(opener)))
                    );
                    src_stream_ids_buffer.push(stream_id);
                    src_buffer.push(stream_id.to_string());
                    src_type_buffer.push("StreamId");
                }
            }
        }
        (src_buffer, src_type_buffer, src_stream_ids_buffer)
    };

    let result = handle.run_mobile_plugin_async::<VideoPlayerResponse>(
        "startVideoPlayer",
        serde_json::json!({
            "src": src,
            "srcType": src_type,
            "options": options
        })
    ).await;

    // stream を破棄する
    {
        let mut streams = MEDIA_STREAMS.lock().unwrap_or_else(|e| e.into_inner());
        for stream_id in src_stream_ids {
            streams.remove(&stream_id);
        }
    }

    // Activity の終了後にすぐに frontend 側に戻ると、
    // その frontend 側の関数の呼び出しが終了しないことが偶にある。
    // よって遅延を強制的に追加してこれを回避する。
    crate::utils::sleep(std::time::Duration::from_millis(200)).await;

    result.map_err(Into::into)
}

struct MediaStream {
    opener: Box<dyn Send + 'static + FnMut() -> Result<Box<dyn MediaSourceReader>, String>>,
    reader: Option<Box<dyn MediaSourceReader>>
}

impl MediaStream {

    fn new(opener: Box<dyn Send + 'static + FnMut() -> Result<Box<dyn MediaSourceReader>, String>>) -> Self {
        Self {
            opener,
            reader: None
        }
    }
}