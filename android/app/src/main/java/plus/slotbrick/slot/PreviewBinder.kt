package plus.slotbrick.slot

import android.graphics.BitmapFactory
import android.net.Uri
import android.view.View
import android.widget.ImageView
import android.widget.TextView
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.ui.PlayerView
import java.io.File

/** Binds [GameMedia] to a [preview_panel] layout, shared by the on-device pane and the Presentation. */
class PreviewBinder(private val root: View) {
    private val playerView: PlayerView = root.findViewById(R.id.previewPlayer)
    private val image: ImageView = root.findViewById(R.id.previewImage)
    private val title: TextView = root.findViewById(R.id.previewTitle)
    private val meta: TextView = root.findViewById(R.id.previewMeta)
    private val source: TextView = root.findViewById(R.id.previewSource)
    private var player: ExoPlayer? = null
    private var boundVideo: File? = null

    fun bind(media: GameMedia, volume: Float) {
        title.text = media.title
        meta.text = media.metaLine()
        source.text = media.note.ifBlank {
            when {
                media.source.contains("screenscraper") && media.source.contains("libretro") ->
                    "ScreenScraper.fr + libretro-thumbnails"
                media.source == "screenscraper" -> "Media from ScreenScraper.fr"
                media.source == "libretro-thumbnails" -> "Stills from libretro-thumbnails"
                else -> ""
            }
        }

        if (media.hasVideo) {
            image.visibility = View.GONE
            playerView.visibility = View.VISIBLE
            play(media.videoFile!!, volume)
        } else {
            stopPlayer()
            playerView.visibility = View.GONE
            val still = media.bestStill()
            if (still != null) {
                image.visibility = View.VISIBLE
                showBitmap(still)
            } else {
                image.setImageDrawable(null)
                image.visibility = View.GONE
            }
        }
    }

    fun release() {
        stopPlayer()
        player?.release()
        player = null
        playerView.player = null
        boundVideo = null
        image.setImageDrawable(null)
    }

    private fun play(file: File, volume: Float) {
        if (boundVideo == file && player != null) {
            player?.volume = volume
            if (player?.isPlaying != true) player?.play()
            return
        }
        val p = player ?: ExoPlayer.Builder(root.context).build().also { created ->
            created.repeatMode = Player.REPEAT_MODE_ONE
            playerView.player = created
            player = created
        }
        p.volume = volume
        p.setMediaItem(MediaItem.fromUri(Uri.fromFile(file)))
        p.prepare()
        p.playWhenReady = true
        boundVideo = file
    }

    private fun stopPlayer() {
        player?.stop()
        player?.clearMediaItems()
        boundVideo = null
    }

    private fun showBitmap(file: File) {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(file.absolutePath, bounds)
        val max = 1280
        var sample = 1
        val w = bounds.outWidth
        val h = bounds.outHeight
        while (w / sample > max || h / sample > max) sample *= 2
        val opts = BitmapFactory.Options().apply { inSampleSize = sample }
        val bmp = BitmapFactory.decodeFile(file.absolutePath, opts)
        image.setImageBitmap(bmp)
    }
}
