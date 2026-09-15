package okayu.tauri.plugin.android.player

import android.app.Activity
import android.content.Intent
import android.view.WindowManager
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import java.lang.IllegalStateException

@InvokeArg
class VideoPlayerOptions {
    val initialSrcIndex: Int? = null
    val initialSrcTimeMs: Long? = null
    val autoplay: Boolean? = null
    val keepScreenOn: Boolean? = null
    val preventScreenCapture: Boolean? = null
    val rejectOnPlaybackError: Boolean? = null
}

@InvokeArg
class StartVideoPlayerArgs {
    lateinit var src: Array<String>
    lateinit var srcType: Array<String>
    lateinit var options: VideoPlayerOptions
}

@TauriPlugin
class PlayerPlugin(private val activity: Activity): Plugin(activity) {

    @Command
    fun startVideoPlayer(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(StartVideoPlayerArgs::class.java)
            val src: Array<String> = args.src
            val srcType: Array<String> = args.srcType
            val srcIndex: Int? = args.options.initialSrcIndex
            val srcTimeMs: Long? = args.options.initialSrcTimeMs
            val autoplay: Boolean? = args.options.autoplay
            val keepScreenOn: Boolean? = args.options.keepScreenOn
            val preventScreenCapture: Boolean = args.options.preventScreenCapture.let {
                if (it == null) {
                    val hasWindowSecureFlag = (activity.window.attributes.flags and WindowManager.LayoutParams.FLAG_SECURE) != 0
                    hasWindowSecureFlag
                }
                else {
                    it
                }
            }
            val rejectOnPlaybackException: Boolean? = args.options.rejectOnPlaybackError

            val intent = Intent(activity, VideoPlayerActivity::class.java).apply {
                putExtra(VideoPlayerActivity.EXTRA_SRC, src)
                putExtra(VideoPlayerActivity.EXTRA_SRC_TYPE, srcType)
                putExtra(VideoPlayerActivity.EXTRA_PREVENT_SCREEN_CAPTURE, preventScreenCapture)
                
                srcIndex?.let { putExtra(VideoPlayerActivity.EXTRA_SRC_INDEX, it) }
                srcTimeMs?.let { putExtra(VideoPlayerActivity.EXTRA_SRC_TIME_MS, it) }
                autoplay?.let { putExtra(VideoPlayerActivity.EXTRA_AUTOPLAY, it) }
                keepScreenOn?.let { putExtra(VideoPlayerActivity.EXTRA_KEEP_SCREEN_ON, it) }
                rejectOnPlaybackException?.let { putExtra(VideoPlayerActivity.EXTRA_REJECT_ON_PLAYBACK_EXCEPTION, it) }
            }

            startActivityForResult(invoke, intent, "videoPlayerCallback")
        }
        catch (e: Exception) {
            invoke.reject(e.message ?: "unknown error")
        }
    }

    @ActivityCallback
    fun videoPlayerCallback(invoke: Invoke, result: ActivityResult) {
        try {
            val intent = result.data
            val errMsg = intent?.getStringExtra(VideoPlayerActivity.RESULT_EXTRA_ERROR_MESSAGE)
            if (errMsg == null) {
                val args = invoke.parseArgs(StartVideoPlayerArgs::class.java)
                if (args.src.isEmpty()) {
                    throw IllegalArgumentException("Missing media sources")
                }

                val initialSrcIndex = args.options.initialSrcIndex ?: 0
                val lastSrcIndex: Int = intent?.getIntExtra(VideoPlayerActivity.RESULT_EXTRA_LAST_SRC_INDEX, initialSrcIndex) ?: initialSrcIndex
                if (lastSrcIndex !in args.src.indices) {
                    throw IllegalStateException("Invalid media source index: $lastSrcIndex")
                }

                var lastSrcTimeMs: Long = intent?.getLongExtra(VideoPlayerActivity.RESULT_EXTRA_LAST_SRC_TIME_MS, 0) ?: 0
                if (lastSrcTimeMs < 0) {
                    lastSrcTimeMs = 0
                }

                invoke.resolve(JSObject().apply {
                    put("lastSrcIndex", lastSrcIndex)
                    put("lastSrcTimeMs", lastSrcTimeMs)
                })
                return
            }
            else {
                invoke.reject(errMsg)
                return
            }
        }
        catch(e: Exception) {
            invoke.reject(e.message ?: "Callback error")
        }
    }
}