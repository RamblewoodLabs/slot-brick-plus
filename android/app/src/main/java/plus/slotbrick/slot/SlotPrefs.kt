package plus.slotbrick.slot

import android.content.Context
import java.util.Properties

/**
 * Settings stored on the device. ScreenScraper passwords never go in logs or
 * into the git tree. Developer credentials start empty; Jeff has to register
 * on the ScreenScraper forum and type the issued devid / devpassword in
 * Settings (or drop a local, gitignored properties file next to the APK
 * sources and rebuild).
 */
object SlotPrefs {
    const val PREFS = "slot"
    const val KEY_TREE = "rom_tree"

    private const val KEY_SS_USER = "ss_user"
    private const val KEY_SS_PASSWORD = "ss_password"
    private const val KEY_SS_DEVID = "ss_devid"
    private const val KEY_SS_DEVPASS = "ss_devpassword"
    private const val KEY_SECOND_SCREEN = "second_screen"
    private const val KEY_PREVIEW_LOW_VOLUME = "preview_low_volume"
    const val HOVER_DELAY_MS = 500L

    data class ScreenScraperCreds(
        val user: String,
        val password: String,
        val devid: String,
        val devpassword: String,
        val softname: String,
    ) {
        val hasUser: Boolean get() = user.isNotBlank() && password.isNotBlank()
        val hasDeveloper: Boolean get() = devid.isNotBlank() && devpassword.isNotBlank()
        val canCallApi: Boolean get() = hasUser && hasDeveloper
    }

    private fun prefs(context: Context) =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    fun screenScraper(context: Context): ScreenScraperCreds {
        val baked = bakedDefaults(context)
        val p = prefs(context)
        return ScreenScraperCreds(
            user = p.getString(KEY_SS_USER, "") ?: "",
            password = p.getString(KEY_SS_PASSWORD, "") ?: "",
            devid = p.getString(KEY_SS_DEVID, "").orEmpty().ifBlank { baked.devid },
            devpassword = p.getString(KEY_SS_DEVPASS, "").orEmpty().ifBlank { baked.devpassword },
            softname = baked.softname.ifBlank { "Slotted" },
        )
    }

    fun saveScreenScraper(
        context: Context,
        user: String,
        password: String,
        devid: String,
        devpassword: String,
    ) {
        prefs(context).edit()
            .putString(KEY_SS_USER, user.trim())
            .putString(KEY_SS_PASSWORD, password)
            .putString(KEY_SS_DEVID, devid.trim())
            .putString(KEY_SS_DEVPASS, devpassword)
            .apply()
    }

    fun secondScreenEnabled(context: Context): Boolean =
        prefs(context).getBoolean(KEY_SECOND_SCREEN, true)

    fun setSecondScreenEnabled(context: Context, enabled: Boolean) {
        prefs(context).edit().putBoolean(KEY_SECOND_SCREEN, enabled).apply()
    }

    /** When false, video snaps play muted. When true, they play at a low volume. */
    fun previewLowVolume(context: Context): Boolean =
        prefs(context).getBoolean(KEY_PREVIEW_LOW_VOLUME, false)

    fun setPreviewLowVolume(context: Context, low: Boolean) {
        prefs(context).edit().putBoolean(KEY_PREVIEW_LOW_VOLUME, low).apply()
    }

    fun previewVolume(context: Context): Float =
        if (previewLowVolume(context)) 0.2f else 0f

    data class BakedDev(val devid: String, val devpassword: String, val softname: String)

    fun bakedDefaults(context: Context): BakedDev {
        return try {
            context.assets.open("screenscraper.properties").use { input ->
                val props = Properties()
                props.load(input)
                BakedDev(
                    devid = props.getProperty("devid", "").trim(),
                    devpassword = props.getProperty("devpassword", "").trim(),
                    softname = props.getProperty("softname", "Slotted").trim().ifBlank { "Slotted" },
                )
            }
        } catch (_: Exception) {
            BakedDev("", "", "Slotted")
        }
    }
}
