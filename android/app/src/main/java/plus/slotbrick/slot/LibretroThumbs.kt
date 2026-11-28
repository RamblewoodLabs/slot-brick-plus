package plus.slotbrick.slot

import android.net.Uri
import java.io.File

/**
 * Fallback stills from libretro-thumbnails. No account is required.
 * This repo has box art, snaps and title screens — not video. Video snaps
 * only come from ScreenScraper when credentials are present.
 *
 * https://github.com/libretro-thumbnails/libretro-thumbnails
 * served at https://thumbnails.libretro.com/
 */
object LibretroThumbs {
    private const val ROOT = "https://thumbnails.libretro.com"

    data class Urls(
        val box: String?,
        val snap: String?,
        val title: String?,
    )

    fun urls(systemId: String, gameName: String): Urls? {
        val playlist = SystemMediaIds.of(systemId).libretroPlaylist ?: return null
        val file = thumbName(gameName)
        if (file.isBlank()) return null
        val encodedList = Uri.encode(playlist)
        val encodedFile = Uri.encode(file)
        return Urls(
            box = "$ROOT/$encodedList/Named_Boxarts/$encodedFile.png",
            snap = "$ROOT/$encodedList/Named_Snaps/$encodedFile.png",
            title = "$ROOT/$encodedList/Named_Titles/$encodedFile.png",
        )
    }

    fun downloadMissing(systemId: String, gameName: String, dir: File): GameMediaFiles {
        val existingBox = File(dir, "box.png").takeIf { it.isFile }
        val existingSnap = File(dir, "snap.png").takeIf { it.isFile }
        val existingTitle = File(dir, "title.png").takeIf { it.isFile }
        val existingWheel = File(dir, "wheel.png").takeIf { it.isFile }
        val existingVideo = File(dir, "video.mp4").takeIf { it.isFile }
        val u = urls(systemId, gameName) ?: return GameMediaFiles(
            existingBox, existingSnap, existingWheel, existingTitle, existingVideo
        )
        val box = existingBox ?: save(u.box, File(dir, "box.png"))
        val snap = existingSnap ?: save(u.snap, File(dir, "snap.png"))
        val title = existingTitle ?: save(u.title, File(dir, "title.png"))
        return GameMediaFiles(box, snap, existingWheel, title, existingVideo)
    }

    /**
     * libretro-thumbnails replace a handful of characters with `_` so the
     * filename matches the No-Intro / Redump name the playlist uses.
     */
    fun thumbName(gameName: String): String {
        val trimmed = gameName.trim()
        if (trimmed.isEmpty()) return ""
        val chars = CharArray(trimmed.length) { i ->
            val c = trimmed[i]
            if (c in "&*/:`<>?\\|\"") '_' else c
        }
        return String(chars)
    }

    private fun save(url: String?, dest: File): File? {
        if (url.isNullOrBlank()) return null
        return if (HttpFetch.download(url, dest, timeoutMs = 15_000)) dest else null
    }
}
