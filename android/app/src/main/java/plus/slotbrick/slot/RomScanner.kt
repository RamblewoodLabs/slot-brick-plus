package plus.slotbrick.slot

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile

data class RomGame(val name: String, val uri: Uri, val folder: String)

data class ScanResult(val bySystem: Map<String, List<RomGame>>, val folderLabel: String)

object RomScanner {
    fun scan(context: Context, treeUri: Uri): ScanResult {
        val root = DocumentFile.fromTreeUri(context, treeUri)
            ?: return ScanResult(emptyMap(), treeUri.toString())
        val buckets = LinkedHashMap<String, MutableList<RomGame>>()
        for (sys in Registry.systems) buckets[sys.id] = ArrayList()

        val children = root.listFiles()
        for (child in children) {
            if (!child.isDirectory) {
                // Loose files at the root: try to classify by extension only.
                classifyFile(child, child.name ?: continue, buckets, sys = null)
                continue
            }
            val folderName = child.name ?: continue
            val sys = Registry.systemForFolder(folderName) ?: continue
            listGames(child, folderName, sys, buckets)
        }
        val cleaned = buckets.mapValues { (_, games) ->
            games.sortedBy { it.name.lowercase() }
        }.filterValues { it.isNotEmpty() }
        val label = root.name ?: treeUri.lastPathSegment ?: "ROM folder"
        return ScanResult(cleaned, label)
    }

    private fun listGames(
        dir: DocumentFile,
        folderName: String,
        sys: SystemDef,
        buckets: MutableMap<String, MutableList<RomGame>>,
    ) {
        for (file in dir.listFiles()) {
            if (file.isDirectory) {
                // One extra level (e.g. cue/bin folders, multi-disc).
                for (nested in file.listFiles()) {
                    classifyFile(nested, folderName, buckets, sys)
                }
            } else {
                classifyFile(file, folderName, buckets, sys)
            }
        }
    }

    private fun classifyFile(
        file: DocumentFile,
        folderName: String,
        buckets: MutableMap<String, MutableList<RomGame>>,
        sys: SystemDef? = null,
    ) {
        if (file.isDirectory) return
        val name = file.name ?: return
        if (name.startsWith(".")) return
        val ext = name.substringAfterLast('.', missingDelimiterValue = "").lowercase()
        if (ext.isEmpty()) return
        val dotted = ".$ext"
        val target = sys ?: Registry.systems.firstOrNull { dotted in it.extensions } ?: return
        if (dotted !in target.extensions) return
        // Skip obvious metadata.
        if (ext == "txt" || ext == "db" || ext == "dat" || ext == "xml" || ext == "png") return
        buckets.getOrPut(target.id) { ArrayList() }.add(
            RomGame(name = name.substringBeforeLast('.'), uri = file.uri, folder = folderName)
        )
    }
}
