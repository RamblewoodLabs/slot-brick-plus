package plus.slotbrick.slot

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.widget.EditText
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SwitchCompat
import java.util.concurrent.Executors

class SettingsActivity : AppCompatActivity() {
    private val io = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)

        val user = findViewById<EditText>(R.id.ssUser)
        val password = findViewById<EditText>(R.id.ssPassword)
        val devid = findViewById<EditText>(R.id.ssDevid)
        val devpassword = findViewById<EditText>(R.id.ssDevpassword)
        val status = findViewById<TextView>(R.id.ssStatus)
        val cacheStatus = findViewById<TextView>(R.id.cacheStatus)
        val second = findViewById<SwitchCompat>(R.id.secondScreen)
        val volume = findViewById<SwitchCompat>(R.id.lowVolume)

        val creds = SlotPrefs.screenScraper(this)
        user.setText(creds.user)
        password.setText(creds.password)
        devid.setText(creds.devid)
        devpassword.setText(creds.devpassword)
        second.isChecked = SlotPrefs.secondScreenEnabled(this)
        volume.isChecked = SlotPrefs.previewLowVolume(this)
        status.text = currentStatus(creds)
        cacheStatus.text = cacheLabel()

        second.setOnCheckedChangeListener { _, on ->
            SlotPrefs.setSecondScreenEnabled(this, on)
        }
        volume.setOnCheckedChangeListener { _, on ->
            SlotPrefs.setPreviewLowVolume(this, on)
        }

        findViewById<TextView>(R.id.saveSs).setOnClickListener {
            SlotPrefs.saveScreenScraper(
                this,
                user.text.toString(),
                password.text.toString(),
                devid.text.toString(),
                devpassword.text.toString(),
            )
            status.text = currentStatus(SlotPrefs.screenScraper(this))
            Toast.makeText(this, "Saved on this device.", Toast.LENGTH_SHORT).show()
        }
        findViewById<TextView>(R.id.testSs).setOnClickListener {
            SlotPrefs.saveScreenScraper(
                this,
                user.text.toString(),
                password.text.toString(),
                devid.text.toString(),
                devpassword.text.toString(),
            )
            status.text = "Checking ScreenScraper…"
            io.execute {
                val msg = try {
                    MediaRepository(this).testLogin()
                } catch (e: Exception) {
                    e.message ?: "Request failed."
                }
                main.post { status.text = msg }
            }
        }
        findViewById<TextView>(R.id.clearCache).setOnClickListener {
            MediaCache.clear(this)
            cacheStatus.text = cacheLabel()
            Toast.makeText(this, "Media cache cleared.", Toast.LENGTH_SHORT).show()
        }
        findViewById<TextView>(R.id.done).setOnClickListener { finish() }
    }

    private fun currentStatus(creds: SlotPrefs.ScreenScraperCreds): String {
        return when {
            !creds.hasDeveloper && !creds.hasUser ->
                "No developer id and no user account. libretro-thumbnails will supply stills; video snaps need ScreenScraper."
            !creds.hasDeveloper ->
                "User account is saved, but there is no developer id yet. Register Slot on the ScreenScraper forum, then paste the issued id and password here. Do not invent one."
            !creds.hasUser ->
                "Developer id is present. Enter your own ScreenScraper username and password to fetch art and video."
            else ->
                "Credentials are present on this device. Use Test login to check the quota. Passwords are not logged or committed."
        }
    }

    private fun cacheLabel(): String {
        val bytes = MediaCache.sizeBytes(this)
        val mb = bytes / (1024.0 * 1024.0)
        return "Cached media on device: %.1f MB".format(mb)
    }
}
