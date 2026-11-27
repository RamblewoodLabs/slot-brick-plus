package plus.slotbrick.slot

import java.nio.ByteBuffer

internal object SlotNative {
    init {
        System.loadLibrary("slot_android")
    }

    @JvmStatic external fun registryJson(): String
    @JvmStatic external fun coreFileName(core: String): String
    @JvmStatic external fun isCoreEmbedded(core: String): Boolean
    @JvmStatic external fun bundledCoreName(core: String): String
    @JvmStatic external fun startGame(coreSo: String, romPath: String, systemDir: String, saveDir: String): String
    @JvmStatic external fun stopGame()
    @JvmStatic external fun setButtons(mask: Int)
    @JvmStatic external fun runFrame(): Boolean
    @JvmStatic external fun frameWidth(): Int
    @JvmStatic external fun frameHeight(): Int
    @JvmStatic external fun sampleRate(): Int
    @JvmStatic external fun copyFrameRgb565(buffer: ByteBuffer): Int
    @JvmStatic external fun copyAudio(buffer: ByteBuffer): Int
    @JvmStatic external fun saveSram()
    @JvmStatic external fun saveState(path: String): String
    @JvmStatic external fun loadState(path: String): String
    @JvmStatic external fun resetGame()
    @JvmStatic external fun defaultStatePath(): String
}
