package plus.slotbrick.slot

import android.app.Activity
import android.content.Context
import android.hardware.display.DisplayManager
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Display
import android.view.View

/**
 * Hover preview: on a dual-screen device (AYN Thor), play the video snap — or
 * fall back to a still plus metadata — on the secondary display. On a
 * single-screen device, the same content appears in a small panel on the
 * main shelf after [SlotPrefs.HOVER_DELAY_MS]. The Presentation is dismissed
 * before an external emulator launches so the two don't fight over the
 * second screen.
 */
class PreviewController(
    private val activity: Activity,
    private val inlinePanel: View,
) : DisplayManager.DisplayListener {

    private val displays = activity.getSystemService(Context.DISPLAY_SERVICE) as DisplayManager
    private val main = Handler(Looper.getMainLooper())
    private val binder = PreviewBinder(inlinePanel)
    private var presentation: GamePreviewPresentation? = null
    private var hoverKey: String? = null
    private val hoverRunnable = Runnable { }
    private var pending: Pair<SystemDef, RomGame>? = null
    private var mediaRepo: MediaRepository? = null
    private var lastMedia: GameMedia? = null

    fun attach(repo: MediaRepository) {
        mediaRepo = repo
        displays.registerDisplayListener(this, main)
        inlinePanel.visibility = View.GONE
    }

    fun onHover(system: SystemDef?, game: RomGame?) {
        main.removeCallbacks(runHover)
        if (system == null || game == null) {
            hoverKey = null
            hideInline()
            return
        }
        val key = "${system.id}:${game.name}"
        hoverKey = key
        pending = system to game
        main.postDelayed(runHover, SlotPrefs.HOVER_DELAY_MS)
    }

    private val runHover = Runnable {
        val pair = pending ?: return@Runnable
        val key = "${pair.first.id}:${pair.second.name}"
        if (key != hoverKey) return@Runnable
        revealTarget()
        mediaRepo?.request(pair.first, pair.second) { media ->
            if (hoverKey == key) apply(media)
        }
    }

    private fun revealTarget() {
        if (useSecondScreen()) {
            ensurePresentation()
            hideInline()
        } else {
            dismissPresentation()
            inlinePanel.visibility = View.VISIBLE
        }
    }

    private fun apply(media: GameMedia) {
        lastMedia = media
        val volume = SlotPrefs.previewVolume(activity)
        if (useSecondScreen() && presentation != null) {
            presentation?.showMedia(media, volume)
            hideInline()
        } else {
            inlinePanel.visibility = View.VISIBLE
            binder.bind(media, volume)
        }
    }

    /**
     * Drop the secondary Presentation so a launched emulator can use that
     * display. The inline pane is also stopped.
     */
    fun releaseForLaunch() {
        main.removeCallbacks(runHover)
        mediaRepo?.cancel()
        binder.release()
        hideInline()
        dismissPresentation()
    }

    fun shutdown() {
        releaseForLaunch()
        try {
            displays.unregisterDisplayListener(this)
        } catch (_: Exception) {
        }
    }

    fun onSettingsChanged() {
        lastMedia?.let { apply(it) } ?: run {
            if (!useSecondScreen()) dismissPresentation()
        }
    }

    private fun useSecondScreen(): Boolean {
        if (!SlotPrefs.secondScreenEnabled(activity)) return false
        return secondaryDisplay() != null
    }

    private fun secondaryDisplay(): Display? {
        val all = displays.displays
        val candidates = all.filter { display ->
            display.displayId != Display.DEFAULT_DISPLAY &&
                display.state != Display.STATE_OFF
        }
        if (candidates.isEmpty()) return null
        val flagged = candidates.filter { it.flags and Display.FLAG_PRESENTATION != 0 }
        val pool = flagged.ifEmpty { candidates }
        return pool.maxByOrNull { it.mode.physicalWidth * it.mode.physicalHeight }
    }

    private fun ensurePresentation() {
        val display = secondaryDisplay() ?: run {
            dismissPresentation()
            return
        }
        val existing = presentation
        if (existing != null && existing.display.displayId == display.displayId && existing.isShowing) {
            return
        }
        dismissPresentation()
        try {
            val next = GamePreviewPresentation(activity, display)
            next.show()
            presentation = next
            lastMedia?.let { next.showMedia(it, SlotPrefs.previewVolume(activity)) }
        } catch (e: Exception) {
            Log.w(TAG, "Presentation failed: ${e.message}")
            presentation = null
        }
    }

    private fun dismissPresentation() {
        val p = presentation
        presentation = null
        if (p != null) {
            try {
                p.releasePlayer()
                p.dismiss()
            } catch (e: Exception) {
                Log.w(TAG, "dismiss Presentation: ${e.message}")
            }
        }
    }

    private fun hideInline() {
        inlinePanel.visibility = View.GONE
    }

    override fun onDisplayAdded(displayId: Int) {
        if (lastMedia != null && SlotPrefs.secondScreenEnabled(activity)) {
            revealTarget()
            lastMedia?.let { apply(it) }
        }
    }

    override fun onDisplayRemoved(displayId: Int) {
        if (presentation?.display?.displayId == displayId) {
            dismissPresentation()
            lastMedia?.let {
                inlinePanel.visibility = View.VISIBLE
                binder.bind(it, SlotPrefs.previewVolume(activity))
            }
        }
    }

    override fun onDisplayChanged(displayId: Int) = Unit

    companion object {
        private const val TAG = "slot-preview"
    }
}
