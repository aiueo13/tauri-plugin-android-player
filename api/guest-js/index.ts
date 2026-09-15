import { invoke } from '@tauri-apps/api/core'


/**
 * File system URI for tauri-plugin-android-fs
 */
export type AndroidFsUri = {
  uri: string,
  documentTopTreeUri: string | null
}

export type AndroidVideoPlayerOptions = {

  /**
   * Indicates the index of the media source to start playback from.
   *
   * @defaultValue 0
   */
  initialSrcIndex?: number,

  /**
   * Indicates the time of the media source to start playback from.
   * 
   * Values outside the valid range are clamped.
   * 
   * @defaultValue 0
   */
  initialSrcTimeMs?: number,

  /**
   * Indicates whether to start playback automatically. 
   * 
   * @defaultValue `true`
   */
  autoplay?: boolean,

  /**
   * Indicates whether to keep the screen on during playback.
   *
   * @defaultValue `true`
   */
  keepScreenOn?: boolean,

  /**
   * Indicates whether to prevent screen capture of the video player.
   *
   * See [`FLAG_SECURE`](https://developer.android.com/security/fraud-prevention/activities#flag_secure) for details.
   *
   * If undefined, inherit the `FLAG_SECURE` state of the caller.
   */
  preventScreenCapture?: boolean,

  /**
   * Indicates whether to close the video player and return an error when playback fails.
   * 
   * Defaults to `false`.
   */
  rejectOnPlaybackError?: boolean,
}

export type AndroidVideoPlayerResponse = {
  lastSrcIndex: number,
  lastSrcTimeMs: number,
}

/** 
 * Opens a player to play the specified videos,
 * returning a promise that **resolves when the player closes**.
 * 
 * @remarks
 * If multiple videos are specified, they are played as a playlist.
 * `options.initialSrcIndex` can be used to specify the video to play initially.
 * 
 * @param src - Media sources of the videos to play. An absolute file path, Android content URI, HTTP(S) URL, or similar source can be used. Supports common video and audio formats, as well as HLS and DASH. See [supported-formats](https://developer.android.com/media/media3/exoplayer/supported-formats) for details.
 * @param options - Optional settings: `initialSrcIndex`, `initialSrcTimeMs`, `autoplay`, `keepScreenOn`, `preventScreenCapture`. See `AndroidVideoPlayerOptions`, `rejectOnPlaybackError`. See `AndroidVideoPlayerOptions` for details.
 * 
 * @returns Promise that resolves when the video player closes.
 * @throws The returned Promise is rejected with an error if an illegal value/type is passed as an argument, such as when `options.initialSrcIndex` is out of range, `src` is an empty array, and etc. Note that the Promise is not rejected if the media content specified by `src` cannot be played or `src` contains an invalid value. Instead, an indication that an error occurred is displayed in the video player, and this method returns successfully when the video player is closed. To close the video player and reject the Promise in such cases, enable `options.rejectOnPlaybackError`.
 * 
 * @see {@link https://docs.rs/tauri-plugin-android-player/latest/tauri_plugin_android_player/struct.AndroidPlayer.html#method.open_video_player | AndroidPlayer::open_video_player}
 */
export async function openAndroidVideoPlayer(
  src: string | AndroidFsUri | URL | (string | AndroidFsUri | URL)[],
  options?: AndroidVideoPlayerOptions
): Promise<AndroidVideoPlayerResponse> {

  return await invoke("plugin:android-player|open_android_video_player", {
    src: mapMediaSrc(src),
    options: options ?? null
  })
}

function mapMediaSrc(mediaSrc: string | AndroidFsUri | URL | (string | AndroidFsUri | URL)[]): string[] {
  function inner(mediaSrc: string | AndroidFsUri | URL): string {
    if (isAndroidFsUri(mediaSrc)) {
      return mediaSrc.uri
    }
    return mediaSrc.toString()
  }
  
  if (Array.isArray(mediaSrc)) {
    return mediaSrc.map(s => inner(s))
  }
  else {
    return [inner(mediaSrc)]
  }
}

function isAndroidFsUri(value: unknown): value is AndroidFsUri {
	if (typeof value !== "object" || value === null) {
		return false
	}

	const obj = value as Record<string, unknown>

	return (
		typeof obj.uri === "string" &&
		(typeof obj.documentTopTreeUri === "string" || obj.documentTopTreeUri === null)
	);
}