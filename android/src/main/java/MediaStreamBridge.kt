package okayu.tauri.plugin.android.player

import androidx.annotation.Keep
import kotlin.jvm.Throws

@Keep
object MediaStreamBridge {

    @Keep
    class MediaStreamException(message: String, val errorCode: Int): Exception(message)

    @JvmStatic
    @Keep
    @Throws(MediaStreamException::class)
    external fun deactivate(streamId: Int)

    /**
     * 長さが不明な場合は -1 を返す。
     */
    @JvmStatic
    @Keep
    @Throws(MediaStreamException::class)
    external fun len(streamId: Int): Long

    /**
     * SAFETY:
     * このメソッド呼び出し中に他のスレッドが buffer にアクセスすることはなく、
     * 同じ配列に対してこのメソッドを複数回同時に呼び出さない必要がある。
     */
    @JvmStatic
    @Keep
    @Throws(MediaStreamException::class)
    external fun read(
        streamId: Int,
        offset: Long,
        length: Int,
        buffer: ByteArray,
        bufferOffset: Int,
    ): Int
}