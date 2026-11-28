package plus.slotbrick.slot

import android.util.Log
import org.json.JSONObject
import java.util.ArrayDeque

/**
 * Respects ScreenScraper's per-user thread and request quotas. We never
 * invent a higher limit than the API reports, and we stay at one in-flight
 * call until the first successful ssuser / jeuInfos response tells us more.
 */
class QuotaGate {
    @Volatile var maxThreads: Int = 1
        private set
    @Volatile var maxPerMinute: Int = 8
        private set
    @Volatile var requestsToday: Int = 0
        private set
    @Volatile var maxPerDay: Int = Int.MAX_VALUE
        private set
    @Volatile var lastError: String? = null

    private val inflight = java.util.concurrent.atomic.AtomicInteger(0)
    private val window = ArrayDeque<Long>()
    private val lock = Any()

    fun updateFromUser(obj: JSONObject?) {
        if (obj == null) return
        fun num(key: String, fallback: Int): Int {
            val raw = obj.optString(key, "").ifBlank { obj.optInt(key, fallback).toString() }
            return raw.toIntOrNull()?.takeIf { it > 0 } ?: fallback
        }
        val threads = num("maxthreads", 1).coerceAtMost(2)
        val perMin = num("maxrequestspermin", maxPerMinute)
        val perDay = num("maxrequestsperday", maxPerDay)
        val today = obj.optString("requeststoday", obj.optInt("requeststoday", requestsToday).toString())
            .toIntOrNull() ?: requestsToday
        maxThreads = threads.coerceAtLeast(1)
        maxPerMinute = perMin.coerceAtLeast(1)
        maxPerDay = perDay.coerceAtLeast(1)
        requestsToday = today
        Log.i(TAG, "ScreenScraper quota: $maxThreads thread(s), $maxPerMinute/min, $requestsToday/$maxPerDay today")
    }

    /**
     * Blocks briefly until a slot is free, or returns false if the daily /
     * per-minute cap is exhausted.
     */
    fun acquire(timeoutMs: Long = 8_000): Boolean {
        val start = System.currentTimeMillis()
        while (System.currentTimeMillis() - start < timeoutMs) {
            synchronized(lock) {
                prune(System.currentTimeMillis())
                if (requestsToday >= maxPerDay) {
                    lastError = "ScreenScraper daily quota reached ($requestsToday/$maxPerDay)."
                    return false
                }
                if (window.size < maxPerMinute && inflight.get() < maxThreads) {
                    inflight.incrementAndGet()
                    window.addLast(System.currentTimeMillis())
                    requestsToday += 1
                    return true
                }
            }
            try {
                Thread.sleep(200)
            } catch (_: InterruptedException) {
                Thread.currentThread().interrupt()
                return false
            }
        }
        lastError = "ScreenScraper is busy (thread or per-minute limit)."
        return false
    }

    fun release() {
        inflight.updateAndGet { n -> (n - 1).coerceAtLeast(0) }
    }

    fun onHttpCode(code: Int) {
        when (code) {
            423, 429 -> {
                lastError = "ScreenScraper quota (HTTP $code). Falling back to libretro-thumbnails."
                // Back off: pretend the minute is full.
                synchronized(lock) {
                    maxPerMinute = 1.coerceAtMost(maxPerMinute)
                }
            }
            426 -> {
                lastError = "Too many ScreenScraper threads. Slowing down."
                maxThreads = 1
            }
            401, 403 -> lastError = "ScreenScraper rejected the account or developer id."
            423 -> lastError = "ScreenScraper closed the request (quota or closed software id)."
        }
    }

    private fun prune(now: Long) {
        while (window.isNotEmpty() && now - window.first() > 60_000) {
            window.removeFirst()
        }
    }

    companion object {
        private const val TAG = "slot-ss"
    }
}
