package okayu.tauri.plugin.android.player

import android.app.Activity
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.OnBackPressedCallback
import androidx.annotation.OptIn
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.Player
import androidx.media3.common.TrackSelectionParameters
import androidx.media3.common.util.RepeatModeUtil
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DataSource
import androidx.media3.datasource.DataSourceException
import androidx.media3.datasource.DataSpec
import androidx.media3.datasource.TransferListener
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import androidx.media3.ui.PlayerView
import java.io.File

@OptIn(UnstableApi::class) class TpapVideoPlayerActivity: AppCompatActivity() {
    companion object {
        const val EXTRA_SRC = "src"
        const val EXTRA_SRC_TYPE = "srcType"
        const val EXTRA_SRC_INDEX = "srcIndex"
        const val EXTRA_SRC_TIME_MS = "srcTimeMs"
        const val EXTRA_AUTOPLAY = "autoPlay"
        const val EXTRA_KEEP_SCREEN_ON = "keepScreenOn"
        const val EXTRA_PREVENT_SCREEN_CAPTURE = "preventScreenCapture"

        const val RESULT_EXTRA_ERROR_MESSAGE = "resultErrMsg"
        const val RESULT_EXTRA_LAST_SRC_TIME_MS = "resultLastSrcTimeMs"
        const val RESULT_EXTRA_LAST_SRC_INDEX = "resultLastSrcIndex"
    }

    private enum class SrcType {
        Uri,
        Path,
        StreamId
    }

    private data class Extra(
        val src: List<Pair<String, SrcType>>,
        val srcIndex: Int,
        val srcTimeMs: Long,
        val autoplay: Boolean,
        val keepScreenOn: Boolean,
        val preventScreenCapture: Boolean,
    )

    private data class PlayerState(
        val playWhenReady: Boolean,
        val mediaItemIndex: Int,
        val position: Long,
        val trackSelectionParameters: TrackSelectionParameters,
        val playbackParameters: PlaybackParameters,
    )

    private var player: ExoPlayer? = null
    private lateinit var playerView: PlayerView
    private var lastPlayerState: PlayerState? = null
    private lateinit var extra: Extra
    private val prefs by lazy {
        getSharedPreferences("tpap_video_player_settings", MODE_PRIVATE)
    }
    private var err: Boolean = false

    override fun onCreate(savedInstanceState: Bundle?) {
        try {
            super.onCreate(savedInstanceState)

            onBackPressedDispatcher.addCallback(
                this,
                object : OnBackPressedCallback(true) {
                    override fun handleOnBackPressed() {
                        setResultExtra()
                        finish()
                    }
                }
            )

            extra = buildExtra()

            setContentView(R.layout.tpap_video_player_activity_main)
            playerView = findViewById(R.id.tpap_video_player_view)

            // player のエラーメッセージの表示
            playerView.setErrorMessageProvider {
                android.util.Pair(it.errorCode, "This content cannot be played: ${it.errorCode}")
            }

            playerView.setShowSubtitleButton(true)
            playerView.setShowShuffleButton(false)

            // プレイリストボタンの設定
            if (1 < extra.src.size) {
                playerView.setRepeatToggleModes(
                    RepeatModeUtil.REPEAT_TOGGLE_MODE_ONE or RepeatModeUtil.REPEAT_TOGGLE_MODE_ALL
                )
                playerView.setShowNextButton(true)
                playerView.setShowPreviousButton(true)
            }
            else {
                playerView.setRepeatToggleModes(RepeatModeUtil.REPEAT_TOGGLE_MODE_ONE)
                playerView.setShowNextButton(false)
                playerView.setShowPreviousButton(false)
            }

            // 没入モードの設定
            isImmersiveMode().apply {
                setImmersiveMode(this)
                playerView.setFullscreenButtonState(this)
            }
            playerView.setFullscreenButtonClickListener {
                setImmersiveMode(it)
            }
            
            // 背景色の設定
            playerView.setBackgroundColor(Color.TRANSPARENT)
            window.setBackgroundDrawableResource(android.R.color.black)
            WindowInsetsControllerCompat(window, window.decorView).apply {
                isAppearanceLightStatusBars = false
                isAppearanceLightNavigationBars = false
            }

            playerView.keepScreenOn = extra.keepScreenOn

            if (extra.preventScreenCapture) {
                window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
            }
        }
        catch(e: Exception) {
            finishWithError(e.message ?: "Playback error")
        }
    }

    override fun onStart() {
        try {
            super.onStart()

            player = ExoPlayer.Builder(this).build().also {
                playerView.player = it

                val srcStreamFactory by lazy {
                    ProgressiveMediaSource.Factory(MediaStreamDataSourceFactory())
                }
                for (src in extra.src) {
                    when (src.second) {
                        SrcType.StreamId -> {
                            val streamId = src.first.toInt()
                            val mediaItem = MediaItem.fromUri(MediaStreamDataSource.buildUri(streamId))
                            val mediaSource = srcStreamFactory.createMediaSource(mediaItem)
                            it.addMediaSource(mediaSource)
                        }
                        SrcType.Uri -> {
                            it.addMediaItem(MediaItem.fromUri(src.first))
                        }
                        SrcType.Path -> {
                            it.addMediaItem(MediaItem.fromUri(Uri.fromFile(File(src.first))))
                        }
                    }
                }

                if (extra.src.size == 1 && repeatMode == Player.REPEAT_MODE_ALL) {
                    it.repeatMode = Player.REPEAT_MODE_ONE
                }
                else {
                    it.repeatMode = repeatMode
                }

                lastPlayerState?.let { state ->
                    it.seekTo(state.mediaItemIndex, state.position)
                    it.trackSelectionParameters = state.trackSelectionParameters
                    it.playbackParameters = state.playbackParameters
                    it.playWhenReady = state.playWhenReady
                } ?: run {
                    it.seekTo(extra.srcIndex, extra.srcTimeMs)
                    it.playWhenReady = extra.autoplay
                }

                val mediaSourceNum = extra.src.size
                var currentMediaItem: MediaItem? = it.currentMediaItem
                it.addListener(object: Player.Listener {
                    override fun onRepeatModeChanged(newRepeatMode: Int) {
                        repeatMode = newRepeatMode
                    }

                    override fun onMediaItemTransition(
                        mediaItem: MediaItem?,
                        reason: Int
                    ) {
                        if (mediaItem != null && currentMediaItem == mediaItem) return

                        if (1 < mediaSourceNum) {
                            val streamId = currentMediaItem?.localConfiguration?.uri?.let {
                                MediaStreamDataSource.getStreamIdFromUri(it.toString())
                            }
                            if (streamId != null) {
                                try {
                                    MediaStream.deactivate(streamId)
                                }
                                catch (e: Throwable) {
                                    finishWithError(e.message ?: "Failed to deactivate media stream")
                                }
                            }
                        }

                        currentMediaItem = mediaItem
                    }
                })

                it.prepare()
            }
        }
        catch(e: Exception) {
            finishWithError(e.message ?: "Playback error")
        }
    }

    override fun onStop() {
        super.onStop()

        setResultExtra()
        player?.let {
            lastPlayerState = PlayerState(
                playWhenReady = it.playWhenReady,
                mediaItemIndex = it.currentMediaItemIndex,
                position = it.currentPosition,
                trackSelectionParameters = it.trackSelectionParameters,
                playbackParameters = it.playbackParameters
            )
        }
        player?.release()
        player = null
        playerView.player = null
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        // 回転後にステータスバーが表示される場合があるので念の為に再度設定する
        setImmersiveMode(isImmersiveMode())
    }


    private fun buildExtra(): Extra {
        val src = intent.getStringArrayExtra(EXTRA_SRC)
            ?.takeIf { it.isNotEmpty() }
            ?.toList()
            ?: throw IllegalArgumentException("Missing media sources")

        val srcType = intent.getStringArrayExtra(EXTRA_SRC_TYPE)
            ?.takeIf { it.isNotEmpty() }
            ?.map {
                when (it) {
                    "StreamId" -> SrcType.StreamId
                    "Uri" -> SrcType.Uri
                    "Path" -> SrcType.Path
                    else -> throw IllegalArgumentException("Illegal src type: $it")
                }
            }
            ?: throw IllegalArgumentException("Missing media source types")

        require(src.size == srcType.size) {
            "The number of media sources and source types must match"
        }

        val srcIndex = intent.getIntExtra(EXTRA_SRC_INDEX, 0)
        if (srcIndex !in src.indices) {
            throw IllegalArgumentException("Invalid media source index: $srcIndex")
        }

        return Extra(
            src = src.zip(srcType),
            srcIndex = srcIndex,
            srcTimeMs = intent.getLongExtra(EXTRA_SRC_TIME_MS, 0),
            autoplay = intent.getBooleanExtra(EXTRA_AUTOPLAY, true),
            keepScreenOn = intent.getBooleanExtra(EXTRA_KEEP_SCREEN_ON, true),
            preventScreenCapture = intent.getBooleanExtra(EXTRA_PREVENT_SCREEN_CAPTURE, true),
        )
    }

    private var repeatMode: Int
        get() = prefs.getInt("repeatMode", Player.REPEAT_MODE_OFF)
        set(v) {
            prefs.edit()
                .putInt("repeatMode", v)
                .apply()
        }

    private fun isImmersiveMode(): Boolean {
        return prefs.getBoolean("immersiveMode", false)
    }

    private fun setImmersiveMode(flag: Boolean) {
        prefs.edit()
            .putBoolean("immersiveMode", flag)
            .apply()

        if (flag) {
            WindowCompat.setDecorFitsSystemWindows(window, false)
            WindowInsetsControllerCompat(window, window.decorView).let {
                it.hide(WindowInsetsCompat.Type.systemBars())
                it.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            }
        }
        else {
            WindowCompat.setDecorFitsSystemWindows(window, false)
            ViewCompat.setOnApplyWindowInsetsListener(playerView) { v, insets ->
                val systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
                v.setPadding(systemBars.left, systemBars.top, systemBars.right, systemBars.bottom)
                insets
            }
            WindowInsetsControllerCompat(window, window.decorView).show(
                WindowInsetsCompat.Type.systemBars()
            )
        }
    }

    private fun setResultExtra() {
        player?.let {
            if (!err) {
                setResult(
                    Activity.RESULT_CANCELED,
                    Intent()
                        .putExtra(RESULT_EXTRA_LAST_SRC_INDEX, it.currentMediaItemIndex)
                        .putExtra(RESULT_EXTRA_LAST_SRC_TIME_MS, it.currentPosition)
                )
            }
        }
    }

    private fun finishWithError(errMsg: String) {
        err = true
        val intent = Intent().putExtra(RESULT_EXTRA_ERROR_MESSAGE, errMsg)
        setResult(Activity.RESULT_CANCELED, intent)
        finish()
    }
}

@OptIn(UnstableApi::class)
class MediaStreamDataSourceFactory : DataSource.Factory {

    override fun createDataSource(): DataSource {
        return MediaStreamDataSource()
    }
}

@OptIn(UnstableApi::class)
class MediaStreamDataSource : DataSource {
    companion object {

        fun buildUri(streamId: Int): String {
            return "tpapmediastream://$streamId"
        }

        fun getStreamIdFromUri(uri: String): Int? {
            val prefix = "tpapmediastream://"
            if (!uri.startsWith(prefix)) {
                return null
            }
            return uri.substring(prefix.length).toIntOrNull()
        }
    }

    private data class State(
        val uri: Uri,
        val streamId: Int,
        val totalLength: Long,
        var position: Long,
    )

    private var state: State? = null

    override fun open(dataSpec: DataSpec): Long {
        try {
            val streamId = getStreamIdFromUri(dataSpec.uri.toString()) ?: throw DataSourceException(
                "Illegal uri format: missing streamId",
                PlaybackException.ERROR_CODE_IO_UNSPECIFIED
            )

            val length = MediaStream.len(streamId);
            val position = dataSpec.position
            if (0 <= length && length < position) {
                throw DataSourceException(
                    "Position out of range: $position > $length",
                    PlaybackException.ERROR_CODE_IO_READ_POSITION_OUT_OF_RANGE
                )
            }

            state = State(
                uri = dataSpec.uri,
                streamId = streamId,
                position = position,
                totalLength = length
            )

            return if (length < 0) {
                C.LENGTH_UNSET.toLong()
            }
            else if (dataSpec.length == C.LENGTH_UNSET.toLong()) {
                length - position
            }
            else {
                minOf(dataSpec.length, length - position)
            }
        }
        catch (e: DataSourceException) {
            throw e
        }
        catch (e: MediaStreamBridge.MediaStreamException) {
            throw DataSourceException(
                e.message ?: "Failed to open media stream",
                e.errorCode,
            )
        }
        catch (e: Throwable) {
            throw DataSourceException(
                e.message ?: "Failed to open media stream",
                PlaybackException.ERROR_CODE_IO_UNSPECIFIED
            )
        }
    }

    override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
        try {
            val state = state ?: throw DataSourceException(
                "Missing state",
                PlaybackException.ERROR_CODE_IO_UNSPECIFIED
            )

            if (state.totalLength == 0L) {
                return C.RESULT_END_OF_INPUT
            }
            if (0 <= state.totalLength && state.totalLength <= state.position) {
                return C.RESULT_END_OF_INPUT
            }

            val n = MediaStream.read(state.streamId, state.position, length, buffer, offset)
            return when {
                0 < n -> {
                    state.position += n;
                    n
                }
                else -> C.RESULT_END_OF_INPUT
            }
        }
        catch (e: DataSourceException) {
            throw e
        }
        catch (e: MediaStreamBridge.MediaStreamException) {
            throw DataSourceException(
                e.message ?: "Failed to read media stream",
                e.errorCode
            )
        }
        catch (e: Throwable) {
            throw DataSourceException(
                e.message ?: "Failed to read media stream",
                PlaybackException.ERROR_CODE_IO_UNSPECIFIED
            )
        }
    }

    override fun close() {
        state = null
        // MediaStreamBridge.deactivateはここでは呼ばない。
    }

    override fun getUri(): Uri? = state?.uri

    override fun addTransferListener(listener: TransferListener) {}
}