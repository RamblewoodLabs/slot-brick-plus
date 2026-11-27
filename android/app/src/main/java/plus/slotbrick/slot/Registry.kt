package plus.slotbrick.slot

import org.json.JSONArray
import org.json.JSONObject

data class PackageSpec(val packageName: String, val activity: String)

data class Emulator(
    val id: String,
    val name: String,
    val packages: List<PackageSpec>,
    val launch: String,
    val installUrl: String,
    val installAltUrl: String?,
    val license: String,
    val notes: String,
)

data class SystemDef(
    val id: String,
    val name: String,
    val folders: List<String>,
    val extensions: List<String>,
    val emulatorId: String,
    val core: String?,
    val onHandheld: Boolean,
)

object Registry {
    val emulators: List<Emulator>
    val systems: List<SystemDef>
    val emulatorsById: Map<String, Emulator>
    val systemsById: Map<String, SystemDef>

    init {
        val root = JSONObject(SlotNative.registryJson())
        emulators = parseEmulators(root.getJSONArray("emulators"))
        systems = parseSystems(root.getJSONArray("systems"))
        emulatorsById = emulators.associateBy { it.id }
        systemsById = systems.associateBy { it.id }
    }

    private fun parseEmulators(arr: JSONArray): List<Emulator> {
        val out = ArrayList<Emulator>(arr.length())
        for (i in 0 until arr.length()) {
            val o = arr.getJSONObject(i)
            val pkgs = o.getJSONArray("packages")
            val list = ArrayList<PackageSpec>(pkgs.length())
            for (j in 0 until pkgs.length()) {
                val p = pkgs.getJSONObject(j)
                list.add(PackageSpec(p.getString("package"), p.getString("activity")))
            }
            out.add(
                Emulator(
                    id = o.getString("id"),
                    name = o.getString("name"),
                    packages = list,
                    launch = o.getString("launch"),
                    installUrl = o.getString("install_url"),
                    installAltUrl = o.optString("install_alt_url").ifEmpty { null },
                    license = o.optString("license"),
                    notes = o.optString("notes"),
                )
            )
        }
        return out
    }

    private fun parseSystems(arr: JSONArray): List<SystemDef> {
        val out = ArrayList<SystemDef>(arr.length())
        for (i in 0 until arr.length()) {
            val o = arr.getJSONObject(i)
            out.add(
                SystemDef(
                    id = o.getString("id"),
                    name = o.getString("name"),
                    folders = stringList(o.getJSONArray("folders")),
                    extensions = stringList(o.getJSONArray("extensions")).map { it.lowercase() },
                    emulatorId = o.getString("emulator"),
                    core = o.optString("core").ifEmpty { null },
                    onHandheld = o.optBoolean("on_handheld"),
                )
            )
        }
        return out
    }

    private fun stringList(arr: JSONArray): List<String> {
        val out = ArrayList<String>(arr.length())
        for (i in 0 until arr.length()) out.add(arr.getString(i))
        return out
    }

    fun systemForFolder(folderName: String): SystemDef? {
        val n = folderName.lowercase()
        return systems.firstOrNull { sys -> sys.folders.any { it.equals(n, ignoreCase = true) } }
    }
}
