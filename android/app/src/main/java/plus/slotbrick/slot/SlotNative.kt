package plus.slotbrick.slot

internal object SlotNative {
    init {
        System.loadLibrary("slot_android")
    }

    @JvmStatic external fun registryJson(): String
    @JvmStatic external fun coreFileName(core: String): String
}
