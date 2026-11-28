package plus.slotbrick.slot

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import java.io.File
import java.io.FileOutputStream

/**
 * Gives a libretro core a real filesystem path for a SAF URI. Small games
 * are copied into the app cache (reused when the size matches). Larger
 * games (PS1 images) are used in place when a readable path can be
 * resolved, otherwise they are also copied.
 */
object RomAccess {
    private const val COPY_LIMIT = 512L * 1024L * 1024L

    data class LocalRom(val path: String, val displayName: String)

    fun materialize(context: Context, uri: Uri, fallbackName: String): LocalRom {
        val name = displayName(context, uri) ?: fallbackName
        filePathIfReadable(context, uri)?.let { return LocalRom(it, name) }

        val size = sizeOf(context, uri)
        val dir = File(context.cacheDir, "roms")
        dir.mkdirs()
        val dest = File(dir, sanitize(name))
        if (dest.isFile && (size < 0 || dest.length() == size)) {
            return LocalRom(dest.absolutePath, name)
        }
        if (size > COPY_LIMIT) {
            // Still copy: a path is required for cores with need_fullpath.
            android.util.Log.w("slot", "Copying large ROM ($size bytes) to cache for the embedded core")
        }
        context.contentResolver.openInputStream(uri)?.use { input ->
            FileOutputStream(dest).use { output -> input.copyTo(output) }
        } ?: throw IllegalStateException("Couldn't open $name")
        return LocalRom(dest.absolutePath, name)
    }

    private fun displayName(context: Context, uri: Uri): String? {
        val fromPath = uri.lastPathSegment?.substringAfterLast('/')
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            ?.use { c ->
                if (c.moveToFirst()) {
                    val i = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    if (i >= 0) {
                        val n = c.getString(i)
                        if (!n.isNullOrBlank()) return n
                    }
                }
            }
        return fromPath
    }

    private fun sizeOf(context: Context, uri: Uri): Long {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.SIZE), null, null, null)
            ?.use { c ->
                if (c.moveToFirst()) {
                    val i = c.getColumnIndex(OpenableColumns.SIZE)
                    if (i >= 0) return c.getLong(i)
                }
            }
        return -1L
    }

    private fun filePathIfReadable(context: Context, uri: Uri): String? {
        val path = uri.path
        if (uri.scheme == "file" && path != null && File(path).canRead()) return path
        if (DocumentsContract.isDocumentUri(context, uri)) {
            val docId = DocumentsContract.getDocumentId(uri)
            if (docId.startsWith("primary:")) {
                val rel = docId.removePrefix("primary:")
                val candidate = File("/storage/emulated/0/$rel")
                if (candidate.canRead()) return candidate.absolutePath
            }
        }
        return null
    }

    private fun sanitize(name: String): String {
        val cleaned = name.replace(Regex("[^A-Za-z0-9._-]+"), "_")
        return cleaned.ifBlank { "rom.bin" }
    }
}
