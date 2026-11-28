package plus.slotbrick.slot

import java.io.File

/**
 * Cached art / video / metadata for one game, ready to show on the preview
 * panel or the Thor's second screen.
 *
 * [raCount] is reserved for RetroAchievements once that Android path exists;
 * it stays null for now and the UI shows an em dash.
 */
data class GameMedia(
    val systemId: String,
    val gameName: String,
    val title: String,
    val year: String? = null,
    val publisher: String? = null,
    val players: String? = null,
    val raCount: Int? = null,
    val boxFile: File? = null,
    val snapFile: File? = null,
    val wheelFile: File? = null,
    val titleShotFile: File? = null,
    val videoFile: File? = null,
    val source: String = "none",
    val note: String = "",
) {
    val hasVideo: Boolean get() = videoFile?.isFile == true
    val hasStill: Boolean
        get() = listOf(snapFile, boxFile, titleShotFile, wheelFile).any { it?.isFile == true }

    fun bestStill(): File? =
        listOf(snapFile, boxFile, titleShotFile, wheelFile).firstOrNull { it?.isFile == true }

    fun metaLine(): String {
        val bits = ArrayList<String>()
        year?.takeIf { it.isNotBlank() }?.let { bits.add(it) }
        publisher?.takeIf { it.isNotBlank() }?.let { bits.add(it) }
        players?.takeIf { it.isNotBlank() }?.let { bits.add("$it players") }
        bits.add("RA ${raCount?.toString() ?: "—"}")
        return bits.joinToString("   ·   ")
    }
}

object MediaCache {
    fun dir(context: android.content.Context, systemId: String, gameName: String): File {
        val safeSys = sanitize(systemId)
        val safeGame = sanitize(gameName)
        val dir = File(File(context.applicationContext.cacheDir, "media"), "$safeSys/$safeGame")
        dir.mkdirs()
        return dir
    }

    fun sanitize(name: String): String {
        val s = name.lowercase().replace(Regex("[^a-z0-9]+"), "_").trim('_')
        return s.take(80).ifBlank { "game" }
    }

    fun clear(context: android.content.Context) {
        File(context.applicationContext.cacheDir, "media").deleteRecursively()
    }

    fun sizeBytes(context: android.content.Context): Long {
        return walkSize(File(context.applicationContext.cacheDir, "media"))
    }

    private fun walkSize(file: File): Long {
        if (!file.exists()) return 0L
        if (file.isFile) return file.length()
        return file.listFiles()?.sumOf { walkSize(it) } ?: 0L
    }
}
