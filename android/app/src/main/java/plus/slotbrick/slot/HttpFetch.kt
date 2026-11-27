package plus.slotbrick.slot

import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL

internal object HttpFetch {
    data class TextResult(val code: Int, val body: String)

    fun getText(url: String, timeoutMs: Int = 15_000): TextResult {
        val conn = open(url, timeoutMs)
        return try {
            val code = conn.responseCode
            val stream = if (code in 200..299) conn.inputStream else conn.errorStream
            val body = stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText() } ?: ""
            TextResult(code, body)
        } finally {
            conn.disconnect()
        }
    }

    fun download(url: String, dest: File, timeoutMs: Int = 30_000): Boolean {
        val tmp = File(dest.parentFile, dest.name + ".part")
        val conn = open(url, timeoutMs)
        return try {
            val code = conn.responseCode
            if (code !in 200..299) return false
            conn.inputStream.use { input ->
                FileOutputStream(tmp).use { output ->
                    input.copyTo(output)
                }
            }
            if (tmp.length() == 0L) {
                tmp.delete()
                return false
            }
            dest.delete()
            tmp.renameTo(dest)
        } catch (_: Exception) {
            tmp.delete()
            false
        } finally {
            conn.disconnect()
        }
    }

    private fun open(url: String, timeoutMs: Int): HttpURLConnection {
        var current = url
        var conn: HttpURLConnection
        // Follow a handful of redirects ourselves so https → https works uniformly.
        repeat(5) {
            conn = (URL(current).openConnection() as HttpURLConnection).apply {
                instanceFollowRedirects = true
                connectTimeout = timeoutMs
                readTimeout = timeoutMs
                setRequestProperty("User-Agent", "Slot/0.3 (Android; GPL-3.0-or-later)")
                setRequestProperty("Accept", "*/*")
            }
            val code = conn.responseCode
            if (code in 300..399) {
                val next = conn.getHeaderField("Location")
                conn.disconnect()
                if (next.isNullOrBlank()) return conn
                current = if (next.startsWith("http")) next else URL(URL(current), next).toString()
            } else {
                return conn
            }
        }
        return URL(current).openConnection() as HttpURLConnection
    }
}
