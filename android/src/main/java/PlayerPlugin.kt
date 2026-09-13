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
            val hasWindowSecureFlag = (activity.window.attributes.flags and WindowManager.LayoutParams.FLAG_SECURE) != 0
            val args = invoke.parseArgs(StartVideoPlayerArgs::class.java)
            val src: Array<String> = args.src
            val srcType: Array<String> = args.srcType
            val srcIndex: Int? = args.options.initialSrcIndex
            val srcTimeMs: Long? = args.options.initialSrcTimeMs
            val autoplay: Boolean? = args.options.autoplay
            val keepScreenOn: Boolean? = args.options.keepScreenOn
            val preventScreenCapture: Boolean = args.options.preventScreenCapture ?: hasWindowSecureFlag

            val intent = Intent(activity, TpapVideoPlayerActivity::class.java).apply {
                putExtra(TpapVideoPlayerActivity.EXTRA_SRC, src)
                putExtra(TpapVideoPlayerActivity.EXTRA_SRC_TYPE, srcType)
                putExtra(TpapVideoPlayerActivity.EXTRA_SRC_INDEX, srcIndex)
                putExtra(TpapVideoPlayerActivity.EXTRA_SRC_TIME_MS, srcTimeMs)
                putExtra(TpapVideoPlayerActivity.EXTRA_AUTOPLAY, autoplay)
                putExtra(TpapVideoPlayerActivity.EXTRA_KEEP_SCREEN_ON, keepScreenOn)
                putExtra(TpapVideoPlayerActivity.EXTRA_PREVENT_SCREEN_CAPTURE, preventScreenCapture)
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
            val errMsg = intent?.getStringExtra(TpapVideoPlayerActivity.RESULT_EXTRA_ERROR_MESSAGE)
            if (errMsg == null) {
                val args = invoke.parseArgs(StartVideoPlayerArgs::class.java)
                if (args.src.isEmpty()) {
                    throw IllegalArgumentException("Missing media sources")
                }

                val initialSrcIndex = args.options.initialSrcIndex ?: 0
                val lastSrcIndex: Int = intent?.getIntExtra(TpapVideoPlayerActivity.RESULT_EXTRA_LAST_SRC_INDEX, initialSrcIndex) ?: initialSrcIndex
                if (lastSrcIndex !in args.src.indices) {
                    throw IllegalStateException("Invalid media source index: $lastSrcIndex")
                }

                var lastSrcTimeMs: Long = intent?.getLongExtra(TpapVideoPlayerActivity.RESULT_EXTRA_LAST_SRC_TIME_MS, 0) ?: 0
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

object MediaStream {

    @Throws(Exception::class)
    fun activate(streamId: Int): Long {
        val prefixOk = "ok:"
        val prefixErr= "err:"
        val result = MediaStreamBridge.activate(streamId)
        if (result.startsWith(prefixOk)) {
            return result.substring(prefixOk.length).toLong()
        }
        else if (result.startsWith(prefixErr)) {
            throw Exception(result.substring(prefixErr.length))
        }
        else {
            throw Exception("Illegal activate result")
        }
    }

    @Throws(Exception::class)
    fun deactivate(streamId: Int) {
        val result = MediaStreamBridge.deactivate(streamId)
        if (result != null) {
            throw Exception(result)
        }
    }

    @Throws(Exception::class)
    fun read(
        streamId: Int,
        offset: Long,
        length: Int,
        buffer: ByteArray,
        bufferOffset: Int,
    ): Int {

        require(0 <= length) { "length must be non-negative" }
        require(0 <= bufferOffset) { "bufferOffset must be non-negative" }
        require(bufferOffset + length <= buffer.size) { "buffer range is out of bounds" }

        if (length == 0) {
            return 0
        }

        val result = MediaStreamBridge.read(
            id = streamId,
            offset = offset,
            len = length,
        )

        if (result.isEmpty()) {
            throw Exception("Illegal read result: empty result")
        }

        val status = result.last().toInt() and 0xFF
        return when (status) {
            0 -> {
                val dataLength = result.size - 1
                if (length < dataLength) {
                    throw Exception("Illegal read result: data length $dataLength > requested length $length")
                }
                result.copyInto(
                    destination = buffer,
                    destinationOffset = bufferOffset,
                    startIndex = 0,
                    endIndex = dataLength,
                )
                dataLength
            }
            1 -> {
                val messageLength = result.size - 1
                val message = result
                    .copyOfRange(0, messageLength)
                    .toString(Charsets.UTF_8)

                throw Exception(message)
            }
            else -> {
                throw Exception("Illegal read status: $status")
            }
        }
    }
}

private object MediaStreamBridge {

    @JvmStatic
    external fun activate(id: Int): String

    @JvmStatic
    external fun deactivate(id: Int): String?

    @JvmStatic
    external fun read(id: Int, offset: Long, len: Int): ByteArray
}