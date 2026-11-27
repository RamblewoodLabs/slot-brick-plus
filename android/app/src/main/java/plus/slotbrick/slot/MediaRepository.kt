package plus.slotbrick.slot

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.Future

/**
 * Looks up art and video for the hovered game: ScreenScraper first (when the
 * user and developer credentials are present), then libretro-thumbnails for
 * any missing stills. Results are cached under the app's cache dir.
 */
class MediaRepository(context: Context) {
    private val app = context.applicationContext
    private val main = Handler(Looper.getMainLooper())
    private val io = Executors.newSingleThreadExecutor { r ->
        Thread(r, "slot-media").apply { isDaemon = true }
    }
    val quota = QuotaGate()
    @Volatile private var inFlight: Future<*>? = null
    @Volatile private var generation = 0L

    fun request(system: SystemDef, game: RomGame, callback: (GameMedia) -> Unit) {
        val gen = ++generation
        inFlight?.cancel(false)
        val placeholder = placeholder(system, game, note = "Looking up art…")
        main.post { if (gen == generation) callback(placeholder) }
        inFlight = io.submit {
            val media = try {
                load(system, game)
            } catch (e: Exception) {
                Log.w(TAG, "media load failed: ${e.message}")
                placeholder(system, game, note = e.message ?: "Lookup failed.")
            }
            if (gen == generation) {
                main.post { if (gen == generation) callback(media) }
            }
        }
    }

    fun cancel() {
        generation += 1
        inFlight?.cancel(false)
    }

    fun testLogin(): String {
        val creds = SlotPrefs.screenScraper(app)
        return ScreenScraperClient(creds, quota).refreshQuota()
    }

    fun clearCache() {
        MediaCache.clear(app)
    }

    private fun load(system: SystemDef, game: RomGame): GameMedia {
        val dir = MediaCache.dir(app, system.id, game.name)
        val cached = readMeta(dir, system, game)
        if (cached != null && cached.hasVideo && cached.hasStill) {
            return cached
        }

        val creds = SlotPrefs.screenScraper(app)
        var title = cached?.title ?: game.name
        var year = cached?.year
        var publisher = cached?.publisher
        var players = cached?.players
        var source = cached?.source ?: "none"
        var note = cached?.note.orEmpty()
        var files = GameMediaFiles(
            box = File(dir, "box.png").takeIf { it.isFile },
            snap = File(dir, "snap.png").takeIf { it.isFile },
            wheel = File(dir, "wheel.png").takeIf { it.isFile },
            titleShot = File(dir, "title.png").takeIf { it.isFile },
            video = File(dir, "video.mp4").takeIf { it.isFile },
        )

        if (creds.canCallApi && (files.video == null || files.box == null)) {
            val client = ScreenScraperClient(creds, quota)
            val hit = client.lookup(system.id, romFileName(game), game.name)
            if (hit != null) {
                title = hit.title
                year = hit.year ?: year
                publisher = hit.publisher ?: publisher
                players = hit.players ?: players
                files = client.downloadInto(hit, dir).let { fresh ->
                    GameMediaFiles(
                        box = fresh.box ?: files.box,
                        snap = fresh.snap ?: files.snap,
                        wheel = fresh.wheel ?: files.wheel,
                        titleShot = fresh.titleShot ?: files.titleShot,
                        video = fresh.video ?: files.video,
                    )
                }
                source = "screenscraper"
                note = "Artwork and video from ScreenScraper.fr."
            } else {
                note = quota.lastError
                    ?: if (!creds.hasDeveloper) {
                        "No ScreenScraper developer id yet — using libretro-thumbnails."
                    } else {
                        "No ScreenScraper match. Trying libretro-thumbnails."
                    }
            }
        } else if (!creds.canCallApi) {
            note = if (!creds.hasDeveloper) {
                "No ScreenScraper developer id (register Slotted on their forum). Using libretro-thumbnails; video snaps need ScreenScraper."
            } else {
                "Enter your ScreenScraper username and password for video snaps. Using libretro-thumbnails for stills."
            }
        }

        if (files.box == null || files.snap == null || files.titleShot == null) {
            val filled = LibretroThumbs.downloadMissing(system.id, game.name, dir)
            val gotAny = listOf(filled.box, filled.snap, filled.titleShot).any { it != null && cachedFileNew(it, files) }
            files = GameMediaFiles(
                box = filled.box ?: files.box,
                snap = filled.snap ?: files.snap,
                wheel = filled.wheel ?: files.wheel,
                titleShot = filled.titleShot ?: files.titleShot,
                video = filled.video ?: files.video,
            )
            if (gotAny) {
                source = if (source == "screenscraper") "screenscraper+libretro-thumbnails" else "libretro-thumbnails"
                if (!note.contains("libretro")) {
                    note = listOf(note, "Stills from libretro-thumbnails.").filter { it.isNotBlank() }.joinToString(" ")
                }
            }
        }

        val media = GameMedia(
            systemId = system.id,
            gameName = game.name,
            title = title,
            year = year,
            publisher = publisher,
            players = players,
            raCount = null,
            boxFile = files.box,
            snapFile = files.snap,
            wheelFile = files.wheel,
            titleShotFile = files.titleShot,
            videoFile = files.video,
            source = source,
            note = note,
        )
        writeMeta(dir, media)
        return media
    }

    private fun cachedFileNew(fresh: File?, old: GameMediaFiles): Boolean {
        if (fresh == null) return false
        return fresh != old.box && fresh != old.snap && fresh != old.titleShot
    }

    private fun placeholder(system: SystemDef, game: RomGame, note: String): GameMedia {
        val dir = MediaCache.dir(app, system.id, game.name)
        val fromDisk = readMeta(dir, system, game)
        return fromDisk?.copy(note = note) ?: GameMedia(
            systemId = system.id,
            gameName = game.name,
            title = game.name,
            note = note,
        )
    }

    private fun romFileName(game: RomGame): String {
        val seg = game.uri.lastPathSegment ?: return game.name
        return Uri.decode(seg).substringAfterLast('/')
    }

    private fun writeMeta(dir: File, media: GameMedia) {
        try {
            val o = JSONObject()
            o.put("title", media.title)
            o.put("year", media.year ?: "")
            o.put("publisher", media.publisher ?: "")
            o.put("players", media.players ?: "")
            o.put("source", media.source)
            o.put("note", media.note)
            File(dir, "meta.json").writeText(o.toString())
        } catch (e: Exception) {
            Log.w(TAG, "meta write: ${e.message}")
        }
    }

    private fun readMeta(dir: File, system: SystemDef, game: RomGame): GameMedia? {
        val meta = File(dir, "meta.json")
        if (!meta.isFile) {
            val stills = GameMedia(
                systemId = system.id,
                gameName = game.name,
                title = game.name,
                boxFile = File(dir, "box.png").takeIf { it.isFile },
                snapFile = File(dir, "snap.png").takeIf { it.isFile },
                wheelFile = File(dir, "wheel.png").takeIf { it.isFile },
                titleShotFile = File(dir, "title.png").takeIf { it.isFile },
                videoFile = File(dir, "video.mp4").takeIf { it.isFile },
            )
            return stills.takeIf { it.hasStill || it.hasVideo }
        }
        return try {
            val o = JSONObject(meta.readText())
            GameMedia(
                systemId = system.id,
                gameName = game.name,
                title = o.optString("title").ifBlank { game.name },
                year = o.optString("year").ifBlank { null },
                publisher = o.optString("publisher").ifBlank { null },
                players = o.optString("players").ifBlank { null },
                boxFile = File(dir, "box.png").takeIf { it.isFile },
                snapFile = File(dir, "snap.png").takeIf { it.isFile },
                wheelFile = File(dir, "wheel.png").takeIf { it.isFile },
                titleShotFile = File(dir, "title.png").takeIf { it.isFile },
                videoFile = File(dir, "video.mp4").takeIf { it.isFile },
                source = o.optString("source"),
                note = o.optString("note"),
            )
        } catch (_: Exception) {
            null
        }
    }

    companion object {
        private const val TAG = "slot-media"
    }
}
