package plus.slotbrick.slot

import android.app.Presentation
import android.content.Context
import android.os.Bundle
import android.view.Display
import android.view.View

/**
 * Drives the AYN Thor's (or any) secondary display via
 * [android.app.Presentation] / DisplayManager. Released before an external
 * emulator is launched so the second screen is free.
 */
class GamePreviewPresentation(
    context: Context,
    display: Display,
) : Presentation(context, display, R.style.Theme_Slot) {

    private var binder: PreviewBinder? = null
    private var pending: GameMedia? = null
    private var pendingVolume: Float = 0f

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val view = layoutInflater.inflate(R.layout.preview_panel, null)
        setContentView(view)
        binder = PreviewBinder(view)
        pending?.let { binder?.bind(it, pendingVolume) }
    }

    fun showMedia(media: GameMedia, volume: Float) {
        pending = media
        pendingVolume = volume
        binder?.bind(media, volume)
    }

    fun releasePlayer() {
        binder?.release()
        binder = null
        pending = null
    }
}
