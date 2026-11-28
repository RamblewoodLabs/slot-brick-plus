package plus.slotbrick.slot

import android.content.Context
import java.io.File

/**
 * Copies PPSSPP's own files (fonts, shaders, compat.ini, the PPGe atlas)
 * from the APK into the core's system directory. The source lives in
 * vendor/ppsspp-assets and is packed by android/build-apk.sh. BIOS and
 * firmware are not included.
 */
internal object PpssppAssets {
    fun install(context: Context, systemDir: File) {
        systemDir.mkdirs()
        val marker = File(systemDir, "compat.ini")
        if (marker.isFile && marker.length() > 0) return
        copyAssetTree(context, "ppsspp", systemDir)
    }

    private fun copyAssetTree(context: Context, assetPath: String, destDir: File) {
        val am = context.assets
        val children = am.list(assetPath)
        if (children.isNullOrEmpty()) {
            destDir.parentFile?.mkdirs()
            am.open(assetPath).use { input ->
                destDir.outputStream().use { output -> input.copyTo(output) }
            }
            return
        }
        destDir.mkdirs()
        for (name in children) {
            copyAssetTree(context, "$assetPath/$name", File(destDir, name))
        }
    }
}
