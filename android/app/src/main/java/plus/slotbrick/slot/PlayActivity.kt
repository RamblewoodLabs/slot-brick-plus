package plus.slotbrick.slot

import android.content.Context
import android.content.Intent
import android.graphics.PixelFormat
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.opengl.GLES20
import android.opengl.GLSurfaceView
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer
import java.nio.IntBuffer
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.opengles.GL10

/**
 * In-process libretro player for the first-wave embedded cores. The Rust
 * host runs on the GL thread; audio is pulled into an AudioTrack; Start+Select
 * or Back opens a small pause menu (save/load state, reset, RetroArch, exit).
 */
class PlayActivity : AppCompatActivity() {
    private lateinit var glView: GLSurfaceView
    private lateinit var menu: TextView
    private var renderer: GameRenderer? = null
    private var paused = false
    private var buttons = 0
    private var stickLx = 0
    private var stickLy = 0
    private var stickRx = 0
    private var stickRy = 0
    private var startHeld = false
    private var selectHeld = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        val extras = intent.extras
        if (extras == null) {
            finish(); return
        }
        val core = extras.getString(EXTRA_CORE) ?: run { finish(); return }
        val rom = extras.getString(EXTRA_ROM) ?: run { finish(); return }
        val so = File(applicationInfo.nativeLibraryDir, SlotNative.bundledCoreName(core))
        if (!so.isFile) {
            Toast.makeText(this, "Embedded core not in this APK. Opening RetroArch.", Toast.LENGTH_LONG).show()
            setResult(RESULT_FALLBACK)
            finish()
            return
        }
        val systemDir = File(filesDir, "system/${extras.getString(EXTRA_SYSTEM) ?: "sys"}").apply { mkdirs() }
        val saveDir = File(filesDir, "saves/${extras.getString(EXTRA_SYSTEM) ?: "sys"}").apply { mkdirs() }
        if (core == "ppsspp") {
            PpssppAssets.install(this, systemDir)
        }

        glView = GLSurfaceView(this).apply {
            // GLES3 so mupen64plus-next / PPSSPP can use a libretro HW context.
            // Software cores run on the same context.
            setEGLContextClientVersion(3)
            holder.setFormat(PixelFormat.RGB_565)
            val r = GameRenderer(so.absolutePath, rom, systemDir.absolutePath, saveDir.absolutePath) { err ->
                runOnUiThread {
                    if (err != null) {
                        Toast.makeText(this@PlayActivity, "Couldn't start the game: $err", Toast.LENGTH_LONG).show()
                        setResult(RESULT_FALLBACK)
                        finish()
                    }
                }
            }
            renderer = r
            setRenderer(r)
            renderMode = GLSurfaceView.RENDERMODE_CONTINUOUSLY
        }
        menu = TextView(this).apply {
            text = MENU
            textSize = 16f
            setTextColor(0xFFF0EAD8.toInt())
            setBackgroundColor(0xCC1A120C.toInt())
            setPadding(32, 32, 32, 32)
            visibility = android.view.View.GONE
            setOnClickListener { /* swallow */ }
        }
        val root = FrameLayout(this)
        root.addView(glView, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
        root.addView(menu, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.WRAP_CONTENT).apply {
            gravity = android.view.Gravity.BOTTOM
        })
        setContentView(root)
    }

    override fun onPause() {
        paused = true
        renderer?.paused = true
        SlotNative.saveSram()
        glView.onPause()
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        glView.onResume()
        if (menu.visibility != android.view.View.VISIBLE) {
            paused = false
            renderer?.paused = false
        }
    }

    override fun onDestroy() {
        renderer?.shutdown = true
        SlotNative.stopGame()
        renderer?.releaseAudio()
        super.onDestroy()
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val down = event.action == KeyEvent.ACTION_DOWN
        val bit = buttonBit(event.keyCode)
        if (event.keyCode == KeyEvent.KEYCODE_BUTTON_START || event.keyCode == KeyEvent.KEYCODE_MENU) {
            startHeld = down
        }
        if (event.keyCode == KeyEvent.KEYCODE_BUTTON_SELECT || event.keyCode == KeyEvent.KEYCODE_BUTTON_MODE) {
            selectHeld = down
        }
        if (down && startHeld && selectHeld) {
            toggleMenu()
            return true
        }
        if (event.keyCode == KeyEvent.KEYCODE_BACK && down) {
            toggleMenu()
            return true
        }
        if (menu.visibility == android.view.View.VISIBLE && down) {
            when (event.keyCode) {
                KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_DPAD_CENTER -> {
                    doMenu('S'); return true
                }
                KeyEvent.KEYCODE_BUTTON_X -> { doMenu('L'); return true }
                KeyEvent.KEYCODE_BUTTON_Y -> { doMenu('R'); return true }
                KeyEvent.KEYCODE_BUTTON_B -> { toggleMenu(); return true }
                KeyEvent.KEYCODE_BUTTON_START -> { doMenu('E'); return true }
                KeyEvent.KEYCODE_BUTTON_R1 -> { doMenu('F'); return true }
            }
        }
        if (bit != 0) {
            buttons = if (down) buttons or bit else buttons and bit.inv()
            renderer?.buttons = buttons
            return true
        }
        if (event.keyCode == KeyEvent.KEYCODE_DPAD_UP) {
            buttons = if (down) buttons or BIT_UP else buttons and BIT_UP.inv(); renderer?.buttons = buttons; return true
        }
        if (event.keyCode == KeyEvent.KEYCODE_DPAD_DOWN) {
            buttons = if (down) buttons or BIT_DOWN else buttons and BIT_DOWN.inv(); renderer?.buttons = buttons; return true
        }
        if (event.keyCode == KeyEvent.KEYCODE_DPAD_LEFT) {
            buttons = if (down) buttons or BIT_LEFT else buttons and BIT_LEFT.inv(); renderer?.buttons = buttons; return true
        }
        if (event.keyCode == KeyEvent.KEYCODE_DPAD_RIGHT) {
            buttons = if (down) buttons or BIT_RIGHT else buttons and BIT_RIGHT.inv(); renderer?.buttons = buttons; return true
        }
        return super.dispatchKeyEvent(event)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        val hatX = event.getAxisValue(MotionEvent.AXIS_HAT_X)
        val hatY = event.getAxisValue(MotionEvent.AXIS_HAT_Y)
        fun set(bit: Int, on: Boolean) { buttons = if (on) buttons or bit else buttons and bit.inv() }
        set(BIT_LEFT, hatX <= -0.5f)
        set(BIT_RIGHT, hatX >= 0.5f)
        set(BIT_UP, hatY <= -0.5f)
        set(BIT_DOWN, hatY >= 0.5f)
        renderer?.buttons = buttons
        val (lx, ly) = axisToStick(event, MotionEvent.AXIS_X, MotionEvent.AXIS_Y)
        var rx = 0
        var ry = 0
        if (event.device != null && event.device.motionRanges.any { it.axis == MotionEvent.AXIS_Z }) {
            val r = axisToStick(event, MotionEvent.AXIS_Z, MotionEvent.AXIS_RZ)
            rx = r.first; ry = r.second
        } else {
            val r = axisToStick(event, MotionEvent.AXIS_RX, MotionEvent.AXIS_RY)
            rx = r.first; ry = r.second
        }
        stickLx = lx; stickLy = ly; stickRx = rx; stickRy = ry
        renderer?.stickLx = lx
        renderer?.stickLy = ly
        renderer?.stickRx = rx
        renderer?.stickRy = ry
        val l2 = event.getAxisValue(MotionEvent.AXIS_LTRIGGER).coerceAtLeast(
            event.getAxisValue(MotionEvent.AXIS_BRAKE)
        )
        val r2 = event.getAxisValue(MotionEvent.AXIS_RTRIGGER).coerceAtLeast(
            event.getAxisValue(MotionEvent.AXIS_GAS)
        )
        set(BIT_L2, l2 >= 0.5f)
        set(BIT_R2, r2 >= 0.5f)
        renderer?.buttons = buttons
        return super.dispatchGenericMotionEvent(event)
    }

    private fun axisToStick(event: MotionEvent, axisX: Int, axisY: Int): Pair<Int, Int> {
        fun one(v: Float): Int {
            val d = 0.15f
            if (kotlin.math.abs(v) < d) return 0
            val scaled = ((v - kotlin.math.sign(v) * d) / (1f - d) * 0x7fff).toInt()
            return scaled.coerceIn(-0x7fff, 0x7fff)
        }
        return one(event.getAxisValue(axisX)) to one(event.getAxisValue(axisY))
    }

    private fun toggleMenu() {
        if (menu.visibility == android.view.View.VISIBLE) {
            menu.visibility = android.view.View.GONE
            paused = false
            renderer?.paused = false
        } else {
            paused = true
            renderer?.paused = true
            SlotNative.saveSram()
            menu.visibility = android.view.View.VISIBLE
        }
    }

    private fun doMenu(cmd: Char) {
        when (cmd) {
            'S' -> {
                val path = SlotNative.defaultStatePath()
                val err = if (path.isNotEmpty()) SlotNative.saveState(path) else "no state path"
                toast(if (err.isEmpty()) "State saved" else "Save failed: $err")
            }
            'L' -> {
                val path = SlotNative.defaultStatePath()
                val err = if (path.isNotEmpty()) SlotNative.loadState(path) else "no state path"
                toast(if (err.isEmpty()) "State loaded" else "Load failed: $err")
                toggleMenu()
            }
            'R' -> { SlotNative.resetGame(); toggleMenu() }
            'F' -> {
                setResult(RESULT_FALLBACK)
                finish()
            }
            'E' -> finish()
        }
    }

    private fun toast(msg: String) = Toast.makeText(this, msg, Toast.LENGTH_SHORT).show()

    companion object {
        const val EXTRA_CORE = "core"
        const val EXTRA_ROM = "rom"
        const val EXTRA_SYSTEM = "system"
        const val RESULT_FALLBACK = 2

        // Libretro JOYPAD bits (Nintendo layout on an Xbox-style pad).
        const val BIT_B = 1 shl 0
        const val BIT_Y = 1 shl 1
        const val BIT_SELECT = 1 shl 2
        const val BIT_START = 1 shl 3
        const val BIT_UP = 1 shl 4
        const val BIT_DOWN = 1 shl 5
        const val BIT_LEFT = 1 shl 6
        const val BIT_RIGHT = 1 shl 7
        const val BIT_A = 1 shl 8
        const val BIT_X = 1 shl 9
        const val BIT_L = 1 shl 10
        const val BIT_R = 1 shl 11
        const val BIT_L2 = 1 shl 12
        const val BIT_R2 = 1 shl 13

        private const val MENU =
            "Paused — Start+Select or Back to resume\n\nA  save state    X  load state\nY  reset         R1  open in RetroArch\nStart  exit"

        fun launch(context: Context, system: SystemDef, game: RomGame): Boolean {
            val core = system.core ?: return false
            if (!SlotNative.isCoreEmbedded(core)) return false
            return try {
                val local = RomAccess.materialize(context, game.uri, game.name)
                val intent = Intent(context, PlayActivity::class.java).apply {
                    putExtra(EXTRA_CORE, core)
                    putExtra(EXTRA_ROM, local.path)
                    putExtra(EXTRA_SYSTEM, system.id)
                }
                context.startActivity(intent)
                true
            } catch (e: Exception) {
                Toast.makeText(context, "Couldn't prepare ROM: ${e.message}", Toast.LENGTH_LONG).show()
                false
            }
        }

        private fun buttonBit(keyCode: Int): Int = when (keyCode) {
            // Xbox-layout pad: A south → SNES/GBA B; B east → A; X west → Y; Y north → X.
            KeyEvent.KEYCODE_BUTTON_A -> BIT_B
            KeyEvent.KEYCODE_BUTTON_B -> BIT_A
            KeyEvent.KEYCODE_BUTTON_X -> BIT_Y
            KeyEvent.KEYCODE_BUTTON_Y -> BIT_X
            KeyEvent.KEYCODE_BUTTON_L1 -> BIT_L
            KeyEvent.KEYCODE_BUTTON_R1 -> BIT_R
            KeyEvent.KEYCODE_BUTTON_L2 -> BIT_L2
            KeyEvent.KEYCODE_BUTTON_R2 -> BIT_R2
            KeyEvent.KEYCODE_BUTTON_SELECT -> BIT_SELECT
            KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_MENU -> BIT_START
            else -> 0
        }
    }
}

private class GameRenderer(
    private val coreSo: String,
    private val romPath: String,
    private val systemDir: String,
    private val saveDir: String,
    private val onStart: (String?) -> Unit,
) : GLSurfaceView.Renderer {
    @Volatile var buttons: Int = 0
    @Volatile var stickLx: Int = 0
    @Volatile var stickLy: Int = 0
    @Volatile var stickRx: Int = 0
    @Volatile var stickRy: Int = 0
    @Volatile var paused: Boolean = false
    @Volatile var shutdown: Boolean = false
    private var ready = false
    private var tex = 0
    private var program = 0
    private var frameBuf: ByteBuffer = ByteBuffer.allocateDirect(2)
    private var audioBuf: ByteBuffer = ByteBuffer.allocateDirect(8192 * 2).order(ByteOrder.LITTLE_ENDIAN)
    private var audio: AudioTrack? = null
    private var lastW = 0
    private var lastH = 0
    private var aPos = 0
    private var aUv = 0
    private var uTex = 0
    private val quad: FloatBuffer = ByteBuffer.allocateDirect(4 * 4 * 4).order(ByteOrder.nativeOrder()).asFloatBuffer().apply {
        // x, y, u, v  (triangle strip). V flipped: software frames are top-left.
        put(floatArrayOf(-1f, -1f, 0f, 1f,  1f, -1f, 1f, 1f,  -1f, 1f, 0f, 0f,  1f, 1f, 1f, 0f))
        position(0)
    }
    private val quadBottomLeft: FloatBuffer = ByteBuffer.allocateDirect(4 * 4 * 4).order(ByteOrder.nativeOrder()).asFloatBuffer().apply {
        // HW FBO textures are bottom-left origin.
        put(floatArrayOf(-1f, -1f, 0f, 0f,  1f, -1f, 1f, 0f,  -1f, 1f, 0f, 1f,  1f, 1f, 1f, 1f))
        position(0)
    }

    override fun onSurfaceCreated(gl: GL10?, config: EGLConfig?) {
        val err = SlotNative.startGame(coreSo, romPath, systemDir, saveDir)
        if (err.isNotEmpty()) {
            android.util.Log.e("slot", "startGame: $err")
            ready = false
            onStart(err)
            return
        }
        ready = true
        onStart(null)
        val rate = SlotNative.sampleRate().coerceAtLeast(8000)
        val min = AudioTrack.getMinBufferSize(rate, AudioFormat.CHANNEL_OUT_STEREO, AudioFormat.ENCODING_PCM_16BIT)
        audio = AudioTrack(
            AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build(),
            AudioFormat.Builder().setSampleRate(rate)
                .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT).build(),
            min.coerceAtLeast(rate / 4),
            AudioTrack.MODE_STREAM,
            AudioManager.AUDIO_SESSION_ID_GENERATE,
        ).also { it.play() }

        program = buildProgram(VERT, FRAG)
        aPos = GLES20.glGetAttribLocation(program, "aPos")
        aUv = GLES20.glGetAttribLocation(program, "aUv")
        uTex = GLES20.glGetUniformLocation(program, "uTex")
        val ids = IntBuffer.allocate(1)
        GLES20.glGenTextures(1, ids)
        tex = ids.get(0)
        GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, tex)
        GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_NEAREST)
        GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MAG_FILTER, GLES20.GL_NEAREST)
        GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_WRAP_S, GLES20.GL_CLAMP_TO_EDGE)
        GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_WRAP_T, GLES20.GL_CLAMP_TO_EDGE)
        GLES20.glClearColor(0f, 0f, 0f, 1f)
    }

    override fun onSurfaceChanged(gl: GL10?, width: Int, height: Int) {
        GLES20.glViewport(0, 0, width, height)
    }

    override fun onDrawFrame(gl: GL10?) {
        if (shutdown || !ready) {
            GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)
            return
        }
        if (!paused) {
            SlotNative.setButtons(buttons)
            SlotNative.setSticks(stickLx, stickLy, stickRx, stickRy)
            SlotNative.runFrame()
            drainAudio()
        }
        val w = SlotNative.frameWidth()
        val h = SlotNative.frameHeight()
        if (w <= 0 || h <= 0) {
            GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)
            return
        }
        GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)
        if (SlotNative.isHwFrame()) {
            val hwTex = SlotNative.hwTextureId()
            if (hwTex != 0) {
                drawQuad(hwTex, SlotNative.hwBottomLeftOrigin())
            }
            return
        }
        val bytes = w * h * 2
        if (frameBuf.capacity() < bytes) {
            frameBuf = ByteBuffer.allocateDirect(bytes)
        }
        frameBuf.clear()
        val n = SlotNative.copyFrameRgb565(frameBuf)
        if (n <= 0) return
        frameBuf.position(0)
        frameBuf.limit(n)
        GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, tex)
        if (w != lastW || h != lastH) {
            GLES20.glTexImage2D(
                GLES20.GL_TEXTURE_2D, 0, GLES20.GL_RGB, w, h, 0,
                GLES20.GL_RGB, GLES20.GL_UNSIGNED_SHORT_5_6_5, frameBuf,
            )
            lastW = w; lastH = h
        } else {
            GLES20.glTexSubImage2D(
                GLES20.GL_TEXTURE_2D, 0, 0, 0, w, h,
                GLES20.GL_RGB, GLES20.GL_UNSIGNED_SHORT_5_6_5, frameBuf,
            )
        }
        drawQuad(tex, flipV = false)
    }

    private fun drawQuad(texture: Int, flipV: Boolean) {
        val verts = if (flipV) quadBottomLeft else quad
        GLES20.glUseProgram(program)
        GLES20.glActiveTexture(GLES20.GL_TEXTURE0)
        GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, texture)
        GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_LINEAR)
        GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MAG_FILTER, GLES20.GL_LINEAR)
        GLES20.glUniform1i(uTex, 0)
        verts.position(0)
        GLES20.glEnableVertexAttribArray(aPos)
        GLES20.glVertexAttribPointer(aPos, 2, GLES20.GL_FLOAT, false, 16, verts)
        val uv = verts.duplicate()
        uv.position(2)
        GLES20.glEnableVertexAttribArray(aUv)
        GLES20.glVertexAttribPointer(aUv, 2, GLES20.GL_FLOAT, false, 16, uv)
        GLES20.glDrawArrays(GLES20.GL_TRIANGLE_STRIP, 0, 4)
        GLES20.glDisableVertexAttribArray(aPos)
        GLES20.glDisableVertexAttribArray(aUv)
    }

    private fun drainAudio() {
        val track = audio ?: return
        audioBuf.clear()
        val samples = SlotNative.copyAudio(audioBuf)
        if (samples > 0) {
            audioBuf.position(0)
            audioBuf.limit(samples * 2)
            track.write(audioBuf, samples * 2, AudioTrack.WRITE_NON_BLOCKING)
        }
    }

    fun releaseAudio() {
        try { audio?.pause(); audio?.release() } catch (_: Exception) {}
        audio = null
    }

    companion object {
        private const val VERT = """
            attribute vec2 aPos;
            attribute vec2 aUv;
            varying vec2 vUv;
            void main() {
                vUv = aUv;
                gl_Position = vec4(aPos, 0.0, 1.0);
            }
        """
        private const val FRAG = """
            precision mediump float;
            varying vec2 vUv;
            uniform sampler2D uTex;
            void main() {
                gl_FragColor = texture2D(uTex, vUv);
            }
        """

        private fun buildProgram(vert: String, frag: String): Int {
            fun compile(type: Int, src: String): Int {
                val id = GLES20.glCreateShader(type)
                GLES20.glShaderSource(id, src)
                GLES20.glCompileShader(id)
                return id
            }
            val p = GLES20.glCreateProgram()
            GLES20.glAttachShader(p, compile(GLES20.GL_VERTEX_SHADER, vert))
            GLES20.glAttachShader(p, compile(GLES20.GL_FRAGMENT_SHADER, frag))
            GLES20.glLinkProgram(p)
            return p
        }
    }
}
