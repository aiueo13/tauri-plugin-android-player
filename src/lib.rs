#![cfg_attr(not(target_os = "android"), allow(unused))]

mod api;
mod cmds;
mod err;
mod impls;
mod utils;
mod media_source;

pub use api::*;
pub use err::*;
pub use media_source::*;

/// Initializes the plugin.
/// 
/// # Usage
/// `src-tauri/src/lib.rs`
/// ```ignore
/// #[cfg_attr(mobile, tauri::mobile_entry_point)]
/// pub fn run() {
///     tauri::Builder::default()
///         .plugin(tauri_plugin_android_player::init())
///         .run(tauri::generate_context!())
///         .expect("error while running tauri application");
/// }
/// ```
pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R, ()> {
    let builder = tauri::plugin::Builder::new("android-player")
        .setup(|app, api| {
            use tauri::Manager as _;

            #[cfg(target_os = "android")] {

                let handle = api.register_android_plugin(
                    crate::impls::PLUGIN_PACKAGE_NAME,
                    crate::impls::PLUGIN_MAIN_CLASS_NAME
                )?;
                app.manage(AndroidPlayer { handle });
            }
            #[cfg(not(target_os = "android"))] {
                app.manage(AndroidPlayer::<R> { handle: Default::default() });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmds::open_android_video_player,
        ]);

    builder.build()
}

pub trait AndroidPlayerExt<R: tauri::Runtime> {

    fn android_player(&self) -> &api::AndroidPlayer<R>;
}

impl<R: tauri::Runtime, T: tauri::Manager<R>> AndroidPlayerExt<R> for T {

    fn android_player(&self) -> &api::AndroidPlayer<R> {
        self.try_state::<AndroidPlayer<R>>()
            .map(|i| i.inner())
            .expect("tauri_plugin_android_player should be initialized to use; see https://crates.io/crates/tauri-plugin-android-player")
    }
}


#[cfg(test)]
#[allow(unused)]
mod readme_example {
use crate as tauri_plugin_android_player;


use tauri_plugin_android_player::{AndroidPlayerExt, MediaSource};

async fn example<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let api = app.android_player();
    let src = MediaSource::from_uri("https://storage.googleapis.com/cloud-samples-data/video/animals.mp4");

    api.open_video_player(
        vec![src], 
        Default::default()
    ).await.unwrap();
}
}