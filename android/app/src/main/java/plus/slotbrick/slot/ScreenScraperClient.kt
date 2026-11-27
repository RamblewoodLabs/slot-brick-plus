package plus.slotbrick.slot

import android.net.Uri
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * ScreenScraper WebAPI v2 client. Requires a developer id issued for this
 * app and the user's own account. We never invent those credentials.
 *
 * Media preference: box-2D, screenshot (`ss`), wheel / marquee, and
 * `video-normalized` then `video`. Files land in [MediaCache].
 */
class ScreenScraperClient(
    private val creds: SlotPrefs.ScreenScraperCreds,
    val quota: QuotaGate,
) {
    data class Hit(
        val title: String,
        val year: String?,
        val publisher: String?,
        val players: String?,
        val boxUrl: String?,
        val snapUrl: String?,
        val wheelUrl: String?,
        val videoUrl: String?,
    )

    fun refreshQuota(): String {
        if (!creds.canCallApi) {
            return if (!creds.hasDeveloper) {
                "No ScreenScraper developer id yet. Register Slot on the forum, then enter the issued id in Settings. libretro-thumbnails still works without an account."
            } else {
                "Enter your ScreenScraper username and password in Settings."
            }
        }
        if (!quota.acquire()) return quota.lastError ?: "ScreenScraper quota reached."
        return try {
            val url = api("ssuserInfos.php")
            val res = HttpFetch.getText(url)
            quota.onHttpCode(res.code)
            if (res.code !in 200..299) {
                return "ScreenScraper login failed (HTTP ${res.code}). Check the account and developer id."
            }
            val json = JSONObject(res.body)
            val err = headerError(json)
            if (err != null) return err
            val user = json.optJSONObject("response")?.optJSONObject("ssuser")
            quota.updateFromUser(user)
            val id = user?.optString("id").orEmpty().ifBlank { creds.user }
            "Signed in as $id. ${quota.maxThreads} thread(s), ${quota.maxPerMinute}/min, ${quota.requestsToday}/${if (quota.maxPerDay == Int.MAX_VALUE) "—" else quota.maxPerDay} today."
        } catch (e: Exception) {
            "ScreenScraper request failed: ${e.message}"
        } finally {
            quota.release()
        }
    }

    fun lookup(systemId: String, romFileName: String, gameName: String): Hit? {
        if (!creds.canCallApi) return null
        val ssId = SystemMediaIds.of(systemId).screenscraperId ?: return null
        val byRom = queryJeuInfos(ssId, romFileName)
        if (byRom != null) return byRom
        return queryRecherche(ssId, gameName)
    }

    fun downloadInto(hit: Hit, dir: File): GameMediaFiles {
        val box = save(hit.boxUrl, File(dir, "box.png"))
        val snap = save(hit.snapUrl, File(dir, "snap.png"))
        val wheel = save(hit.wheelUrl, File(dir, "wheel.png"))
        val video = save(hit.videoUrl, File(dir, "video.mp4"))
        return GameMediaFiles(box, snap, wheel, null, video)
    }

    private fun save(url: String?, dest: File): File? {
        if (url.isNullOrBlank()) return dest.takeIf { it.isFile }
        if (dest.isFile && dest.length() > 0L) return dest
        return if (HttpFetch.download(url, dest)) dest else dest.takeIf { it.isFile }
    }

    private fun queryJeuInfos(systemeid: Int, romnom: String): Hit? {
        if (!quota.acquire()) return null
        return try {
            val url = api("jeuInfos.php") +
                "&systemeid=$systemeid" +
                "&romtype=rom" +
                "&romnom=" + Uri.encode(romnom)
            parseJeu(HttpFetch.getText(url))
        } finally {
            quota.release()
        }
    }

    private fun queryRecherche(systemeid: Int, name: String): Hit? {
        val term = name.replace(Regex("""\s*\([^)]*\)"""), "").trim().ifBlank { name }
        if (term.isBlank()) return null
        if (!quota.acquire()) return null
        return try {
            val url = api("jeuRecherche.php") +
                "&systemeid=$systemeid" +
                "&recherche=" + Uri.encode(term)
            val res = HttpFetch.getText(url)
            quota.onHttpCode(res.code)
            if (res.code !in 200..299) return null
            val json = JSONObject(res.body)
            if (headerError(json) != null) return null
            val jeux = json.optJSONObject("response")?.opt("jeux")
            val first = when (jeux) {
                is JSONArray -> jeux.optJSONObject(0)
                is JSONObject -> jeux
                else -> null
            } ?: return null
            if (first.length() == 0) return null
            // Recherche often returns a short card; prefer fetching the full jeu
            // when an id is present, otherwise parse what we have.
            val id = first.optString("id").ifBlank { first.optJSONObject("id")?.optString("text").orEmpty() }
            if (id.isNotBlank()) {
                queryJeuById(id, alreadyHeld = true)
            } else {
                mediaFromJeu(first)
            }
        } catch (e: Exception) {
            Log.w(TAG, "recherche failed: ${e.message}")
            null
        } finally {
            quota.release()
        }
    }

    private fun queryJeuById(gameId: String, alreadyHeld: Boolean = false): Hit? {
        if (!alreadyHeld && !quota.acquire()) return null
        return try {
            val url = api("jeuInfos.php") + "&gameid=" + Uri.encode(gameId)
            parseJeu(HttpFetch.getText(url))
        } finally {
            if (!alreadyHeld) quota.release()
        }
    }

    private fun parseJeu(res: HttpFetch.TextResult): Hit? {
        quota.onHttpCode(res.code)
        if (res.code !in 200..299) return null
        return try {
            val json = JSONObject(res.body)
            if (headerError(json) != null) return null
            val response = json.optJSONObject("response") ?: return null
            quota.updateFromUser(response.optJSONObject("ssuser"))
            val jeu = response.optJSONObject("jeu") ?: return null
            mediaFromJeu(jeu)
        } catch (e: Exception) {
            Log.w(TAG, "parse jeuInfos: ${e.message}")
            null
        }
    }

    private fun mediaFromJeu(jeu: JSONObject): Hit? {
        if (jeu.length() == 0) return null
        val title = pickName(jeu) ?: return null
        val year = pickYear(jeu)
        val publisher = firstText(jeu.opt("editeur")) ?: firstText(jeu.opt("editeurs"))
        val players = firstText(jeu.opt("joueurs")) ?: firstText(jeu.opt("joueurs_max"))
        val medias = asArray(jeu.opt("medias"))
        return Hit(
            title = title,
            year = year,
            publisher = publisher,
            players = players,
            boxUrl = pickMedia(medias, BOX_TYPES),
            snapUrl = pickMedia(medias, SNAP_TYPES),
            wheelUrl = pickMedia(medias, WHEEL_TYPES),
            videoUrl = pickMedia(medias, VIDEO_TYPES),
        )
    }

    private fun pickName(jeu: JSONObject): String? {
        val noms = asArray(jeu.opt("noms"))
        if (noms.length() > 0) {
            val preferred = listOf("ss", "us", "wor", "eu", "jp", "ame", "iusa")
            for (region in preferred) {
                for (i in 0 until noms.length()) {
                    val o = noms.optJSONObject(i) ?: continue
                    if (o.optString("region").equals(region, true)) {
                        val t = o.optString("text").ifBlank { o.optString("nom") }
                        if (t.isNotBlank()) return t
                    }
                }
            }
            for (i in 0 until noms.length()) {
                val o = noms.optJSONObject(i) ?: continue
                val t = o.optString("text").ifBlank { o.optString("nom") }
                if (t.isNotBlank()) return t
            }
        }
        return firstText(jeu.opt("nom")) ?: jeu.optString("nom").ifBlank { null }
    }

    private fun pickYear(jeu: JSONObject): String? {
        val dates = asArray(jeu.opt("dates"))
        for (i in 0 until dates.length()) {
            val o = dates.optJSONObject(i) ?: continue
            val raw = o.optString("text").ifBlank { o.optString("date") }
            val year = Regex("""(\d{4})""").find(raw)?.groupValues?.get(1)
            if (year != null) return year
        }
        val raw = firstText(jeu.opt("date")) ?: return null
        return Regex("""(\d{4})""").find(raw)?.groupValues?.get(1)
    }

    private fun pickMedia(medias: JSONArray, types: List<String>): String? {
        val preferredRegions = listOf("us", "wor", "ss", "eu", "jp", "ame")
        for (type in types) {
            val matches = ArrayList<JSONObject>()
            for (i in 0 until medias.length()) {
                val o = medias.optJSONObject(i) ?: continue
                if (o.optString("type").equals(type, true)) matches.add(o)
            }
            if (matches.isEmpty()) continue
            for (region in preferredRegions) {
                val hit = matches.firstOrNull { it.optString("region").equals(region, true) }
                val url = hit?.optString("url")
                if (!url.isNullOrBlank()) return url
            }
            val any = matches.firstOrNull()?.optString("url")
            if (!any.isNullOrBlank()) return any
        }
        return null
    }

    private fun asArray(value: Any?): JSONArray {
        return when (value) {
            is JSONArray -> value
            is JSONObject -> JSONArray().put(value)
            else -> JSONArray()
        }
    }

    private fun firstText(value: Any?): String? {
        return when (value) {
            is JSONObject -> value.optString("text").ifBlank { value.optString("nom") }.ifBlank { null }
            is JSONArray -> {
                for (i in 0 until value.length()) {
                    val t = firstText(value.opt(i))
                    if (!t.isNullOrBlank()) return t
                }
                null
            }
            is String -> value.ifBlank { null }
            else -> null
        }
    }

    private fun headerError(json: JSONObject): String? {
        val header = json.optJSONObject("header") ?: return null
        val err = header.optString("erreur").ifBlank { header.optString("error") }
        return err.ifBlank { null }
    }

    private fun api(endpoint: String): String {
        val b = Uri.parse("https://api.screenscraper.fr/api2/$endpoint").buildUpon()
            .appendQueryParameter("devid", creds.devid)
            .appendQueryParameter("devpassword", creds.devpassword)
            .appendQueryParameter("softname", creds.softname)
            .appendQueryParameter("ssid", creds.user)
            .appendQueryParameter("sspassword", creds.password)
            .appendQueryParameter("output", "json")
        return b.build().toString()
    }

    companion object {
        private const val TAG = "slot-ss"
        private val BOX_TYPES = listOf("box-2D", "box-2D-side", "box-3D", "box-texture", "box")
        private val SNAP_TYPES = listOf("ss", "sstitle", "screenshot")
        private val WHEEL_TYPES = listOf("wheel-hd", "wheel", "wheel-carbon", "wheel-steel", "marquee")
        private val VIDEO_TYPES = listOf("video-normalized", "video")
    }
}

data class GameMediaFiles(
    val box: File?,
    val snap: File?,
    val wheel: File?,
    val titleShot: File?,
    val video: File?,
)
