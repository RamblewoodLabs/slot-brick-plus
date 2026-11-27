package plus.slotbrick.slot

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.widget.Toast
import java.util.regex.Pattern

data class ResolvedEmulator(val emulator: Emulator, val spec: PackageSpec)

object EmulatorLauncher {
    private val TITLE_ID = Pattern.compile("(?i)(PCSA|PCSB|PCSC|PCSD|PCSE|PCSF|PCSG|PCSH|VLJS|VLJM|VPKA|NPXS)[0-9]{5}")

    fun resolve(context: Context, emulator: Emulator): ResolvedEmulator? {
        val pm = context.packageManager
        for (spec in emulator.packages) {
            if (isInstalled(pm, spec.packageName) && activityResolves(pm, spec)) {
                return ResolvedEmulator(emulator, spec)
            }
        }
        // Package present but activity name moved: still treat as installed if any package hits.
        for (spec in emulator.packages) {
            if (isInstalled(pm, spec.packageName)) {
                return ResolvedEmulator(emulator, spec)
            }
        }
        return null
    }

    fun isInstalled(pm: PackageManager, packageName: String): Boolean {
        return try {
            pm.getPackageInfo(packageName, 0)
            true
        } catch (_: PackageManager.NameNotFoundException) {
            false
        }
    }

    private fun activityResolves(pm: PackageManager, spec: PackageSpec): Boolean {
        val intent = Intent().setClassName(spec.packageName, spec.activity)
        return intent.resolveActivity(pm) != null
    }

    fun launch(context: Context, system: SystemDef, game: RomGame): Boolean {
        val emulator = Registry.emulatorsById[system.emulatorId] ?: return false
        val resolved = resolve(context, emulator)
        if (resolved == null) return false
        return try {
            when (emulator.launch) {
                "autostart_file" -> launchAutoStart(context, resolved, game.uri)
                "view" -> launchView(context, resolved, game.uri)
                "vita_title_id" -> launchVita(context, resolved, game)
                "retroarch" -> launchRetroArch(context, resolved, system, game)
                else -> launchView(context, resolved, game.uri)
            }
            true
        } catch (e: Exception) {
            Toast.makeText(context, "Couldn't start ${emulator.name}: ${e.message}", Toast.LENGTH_LONG).show()
            false
        }
    }

    private fun launchView(context: Context, resolved: ResolvedEmulator, uri: Uri) {
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setClassName(resolved.spec.packageName, resolved.spec.activity)
            setDataAndType(uri, "*/*")
            addFlags(
                Intent.FLAG_ACTIVITY_NEW_TASK or
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                    Intent.FLAG_GRANT_PREFIX_URI_PERMISSION
            )
            clipData = ClipData.newUri(context.contentResolver, "rom", uri)
        }
        grant(context, resolved.spec.packageName, uri)
        context.startActivity(intent)
    }

    private fun launchAutoStart(context: Context, resolved: ResolvedEmulator, uri: Uri) {
        // Dolphin: do not set ACTION_VIEW; MainActivity does not accept it.
        val intent = Intent().apply {
            setClassName(resolved.spec.packageName, resolved.spec.activity)
            putExtra("AutoStartFile", uri.toString())
            data = uri
            addFlags(
                Intent.FLAG_ACTIVITY_NEW_TASK or
                    Intent.FLAG_ACTIVITY_CLEAR_TOP or
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                    Intent.FLAG_GRANT_PREFIX_URI_PERMISSION
            )
            clipData = ClipData.newUri(context.contentResolver, "rom", uri)
        }
        grant(context, resolved.spec.packageName, uri)
        context.startActivity(intent)
    }

    private fun launchVita(context: Context, resolved: ResolvedEmulator, game: RomGame) {
        val titleId = TITLE_ID.matcher(game.name).let { m -> if (m.find()) m.group().uppercase() else null }
        if (titleId == null) {
            Toast.makeText(
                context,
                "Vita3K launches by Title ID after the VPK is installed in Vita3K. Open Vita3K, install this VPK, then rename the file to include the ID (for example PCSE00000) to start it from Slot.",
                Toast.LENGTH_LONG,
            ).show()
            val open = resolved.spec.packageName.let { context.packageManager.getLaunchIntentForPackage(it) }
            if (open != null) {
                open.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                context.startActivity(open)
            }
            return
        }
        val intent = Intent().apply {
            setClassName(resolved.spec.packageName, resolved.spec.activity)
            putExtra("AppStartParameters", arrayOf("-r", titleId))
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        context.startActivity(intent)
    }

    private fun launchRetroArch(context: Context, resolved: ResolvedEmulator, system: SystemDef, game: RomGame) {
        val core = system.core ?: "mgba"
        val so = SlotNative.coreFileName(core)
        val dataDir = "/data/data/${resolved.spec.packageName}"
        val corePath = "$dataDir/cores/$so"
        val config = "/storage/emulated/0/Android/data/${resolved.spec.packageName}/files/retroarch.cfg"
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setClassName(resolved.spec.packageName, resolved.spec.activity)
            setDataAndType(game.uri, "*/*")
            putExtra("ROM", game.uri.toString())
            putExtra("LIBRETRO", corePath)
            putExtra("CONFIGFILE", config)
            putExtra("QUITFOCUS", "1")
            addFlags(
                Intent.FLAG_ACTIVITY_NEW_TASK or
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                    Intent.FLAG_GRANT_PREFIX_URI_PERMISSION
            )
            clipData = ClipData.newUri(context.contentResolver, "rom", game.uri)
        }
        grant(context, resolved.spec.packageName, game.uri)
        context.startActivity(intent)
    }

    private fun grant(context: Context, packageName: String, uri: Uri) {
        try {
            context.grantUriPermission(
                packageName,
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        } catch (_: Exception) {
            // Some tree document URIs refuse a one-shot grant; FLAG_GRANT_* on the intent may still work.
        }
    }
}
