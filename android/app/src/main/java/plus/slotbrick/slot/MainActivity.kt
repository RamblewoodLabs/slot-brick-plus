package plus.slotbrick.slot

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView

class MainActivity : AppCompatActivity() {
    private var treeUri: Uri? = null
    private var scan: ScanResult = ScanResult(emptyMap(), "")
    private var systemIndex = 0
    private var gameIndex = 0
    private var hatX = 0f
    private var hatY = 0f
    private var stickHold = 0L

    private val systemsOnShelf: List<SystemDef>
        get() {
            val present = scan.bySystem.keys
            val ordered = Registry.systems.filter { it.id in present }
            return ordered.ifEmpty { Registry.systems }
        }

    private val currentSystem: SystemDef?
        get() = systemsOnShelf.getOrNull(systemIndex)

    private val currentGames: List<RomGame>
        get() = currentSystem?.let { scan.bySystem[it.id].orEmpty() } ?: emptyList()

    private val pickTree = registerForActivityResult(
        ActivityResultContracts.OpenDocumentTree()
    ) { uri ->
        if (uri != null) persistTree(uri)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // ViewBinding is not enabled; inflate the layout directly.
        setContentView(R.layout.activity_main)
        bindViews()
        val saved = getSharedPreferences(PREFS, MODE_PRIVATE).getString(KEY_TREE, null)
        if (saved != null) {
            try {
                treeUri = Uri.parse(saved)
                rescan()
            } catch (_: Exception) {
                treeUri = null
            }
        }
        if (treeUri == null) {
            promptForFolder(firstTime = true)
        }
        refreshUi()
    }

    private lateinit var titleView: TextView
    private lateinit var folderBtn: TextView
    private lateinit var systemRow: RecyclerView
    private lateinit var gameList: RecyclerView
    private lateinit var statusView: TextView
    private lateinit var hintView: TextView
    private val systemAdapter = LabelAdapter { pos ->
        systemIndex = pos
        gameIndex = 0
        refreshUi()
    }
    private val gameAdapter = LabelAdapter { pos ->
        gameIndex = pos
        refreshUi()
        onActivate()
    }

    private fun bindViews() {
        titleView = findViewById(R.id.title)
        folderBtn = findViewById(R.id.folderButton)
        systemRow = findViewById(R.id.systemRow)
        gameList = findViewById(R.id.gameList)
        statusView = findViewById(R.id.status)
        hintView = findViewById(R.id.hint)
        folderBtn.setOnClickListener { pickTree.launch(null) }
        systemRow.layoutManager = LinearLayoutManager(this, LinearLayoutManager.HORIZONTAL, false)
        gameList.layoutManager = LinearLayoutManager(this)
        systemRow.adapter = systemAdapter
        gameList.adapter = gameAdapter
    }

    private fun persistTree(uri: Uri) {
        try {
            contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        } catch (e: SecurityException) {
            android.util.Log.w("slot", "persistable URI permission: $e")
        }
        getSharedPreferences(PREFS, MODE_PRIVATE).edit().putString(KEY_TREE, uri.toString()).apply()
        treeUri = uri
        rescan()
        refreshUi()
    }

    private fun rescan() {
        val uri = treeUri ?: return
        Toast.makeText(this, "Scanning ROM folder…", Toast.LENGTH_SHORT).show()
        scan = try {
            RomScanner.scan(this, uri)
        } catch (e: Exception) {
            Toast.makeText(this, "Scan failed: ${e.message}", Toast.LENGTH_LONG).show()
            ScanResult(emptyMap(), "")
        }
        systemIndex = 0
        gameIndex = 0
    }

    private fun refreshUi() {
        val systems = systemsOnShelf
        if (systemIndex >= systems.size) systemIndex = 0
        val games = currentGames
        if (gameIndex >= games.size) gameIndex = 0
        folderBtn.text = if (treeUri == null) {
            "Choose ROM folder"
        } else {
            "ROM folder: ${scan.folderLabel.ifEmpty { "set" }}"
        }
        systemAdapter.submit(systems.map { it.name }, systemIndex)
        gameAdapter.submit(games.map { it.name }, gameIndex)
        if (games.isNotEmpty()) {
            gameList.scrollToPosition(gameIndex)
        }
        if (systemIndex in systems.indices) {
            systemRow.scrollToPosition(systemIndex)
        }
        statusView.text = statusText()
        hintView.text = "D-pad: move   L/R or up/down: system   A: play   Start: folder"
    }

    private fun statusText(): String {
        val sys = currentSystem ?: return "No systems found. Choose the folder that holds gc, PS2, Switch, 3DS…"
        val emu = Registry.emulatorsById[sys.emulatorId]
        val games = currentGames
        val count = if (games.isEmpty()) "no games in this folder yet" else "${games.size} game(s)"
        if (emu == null) return "${sys.name}: $count"
        val resolved = EmulatorLauncher.resolve(this, emu)
        val coreNote = if (sys.core != null) {
            "  Interim: RetroArch + ${sys.core} core."
        } else ""
        return if (resolved != null) {
            "${sys.name}: $count   ${emu.name} is installed.$coreNote"
        } else {
            "${sys.name}: $count   ${emu.name} is not installed. Press A for the install page.$coreNote"
        }
    }

    private fun onActivate() {
        val sys = currentSystem ?: return
        val emu = Registry.emulatorsById[sys.emulatorId] ?: return
        val resolved = EmulatorLauncher.resolve(this, emu)
        if (resolved == null) {
            promptInstall(emu)
            return
        }
        val game = currentGames.getOrNull(gameIndex)
        if (game == null) {
            Toast.makeText(this, "No game selected. Put games in a ${sys.folders.first()} folder inside the ROM root.", Toast.LENGTH_LONG).show()
            return
        }
        if (emu.id == "eden") {
            Toast.makeText(this, "Eden needs your own prod.keys and firmware, dumped from a Switch you own. Slot never bundles them.", Toast.LENGTH_LONG).show()
        }
        if (emu.id == "retroarch") {
            val so = SlotNative.coreFileName(sys.core ?: "")
            Toast.makeText(this, "Opening RetroArch. If it fails, install the $so core in RetroArch’s updater (interim until Slot embeds cores).", Toast.LENGTH_LONG).show()
        }
        EmulatorLauncher.launch(this, sys, game)
    }

    private fun promptInstall(emu: Emulator) {
        val msg = buildString {
            append("${emu.name} is not installed.\n\n")
            append("Slot will open the official download page. We never bundle BIOS, firmware, or console keys.\n")
            if (emu.notes.isNotBlank()) {
                append("\n")
                append(emu.notes)
            }
        }
        AlertDialog.Builder(this)
            .setTitle("Install ${emu.name}?")
            .setMessage(msg)
            .setPositiveButton("Install") { _, _ -> openInstall(emu.installUrl) }
            .apply {
                if (emu.installAltUrl != null) {
                    setNeutralButton("Other download") { _, _ -> openInstall(emu.installAltUrl) }
                }
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun openInstall(url: String) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            startActivity(intent)
        } catch (_: Exception) {
            Toast.makeText(this, "Couldn't open $url", Toast.LENGTH_LONG).show()
        }
    }

    private fun promptForFolder(firstTime: Boolean) {
        val msg = if (firstTime) {
            "Choose the folder that holds your games (the one with gc, PS2, Switch, 3DS, dreamcast, saturn, psvita, xbox, and the older system folders)."
        } else {
            "Choose a different ROM folder."
        }
        AlertDialog.Builder(this)
            .setTitle("ROM folder")
            .setMessage(msg)
            .setPositiveButton("Choose") { _, _ -> pickTree.launch(null) }
            .setNegativeButton("Later", null)
            .show()
    }

    private fun moveSystem(delta: Int) {
        val n = systemsOnShelf.size
        if (n == 0) return
        systemIndex = (systemIndex + delta + n) % n
        gameIndex = 0
        refreshUi()
    }

    private fun moveGame(delta: Int) {
        val n = currentGames.size
        if (n == 0) return
        gameIndex = (gameIndex + delta).coerceIn(0, n - 1)
        refreshUi()
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val action = GamepadMap.actionFromKey(event)
        if (action != SlotAction.NONE && event.action == KeyEvent.ACTION_DOWN) {
            handleAction(action)
            return true
        }
        return super.dispatchKeyEvent(event)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        val (hatAction, hat) = GamepadMap.actionFromHat(event, hatX, hatY)
        hatX = hat.first
        hatY = hat.second
        if (hatAction != SlotAction.NONE) {
            handleAction(hatAction)
            return true
        }
        val now = android.os.SystemClock.uptimeMillis()
        if (now - stickHold > 220) {
            val (lx, ly) = GamepadMap.leftStick(event)
            val next = when {
                lx <= -0.6f -> SlotAction.LEFT
                lx >= 0.6f -> SlotAction.RIGHT
                ly <= -0.6f -> SlotAction.UP
                ly >= 0.6f -> SlotAction.DOWN
                else -> SlotAction.NONE
            }
            if (next != SlotAction.NONE) {
                stickHold = now
                handleAction(next)
                return true
            }
        }
        return super.dispatchGenericMotionEvent(event)
    }

    private fun handleAction(action: SlotAction) {
        when (action) {
            SlotAction.LEFT -> moveGame(-1)
            SlotAction.RIGHT -> moveGame(1)
            SlotAction.UP -> moveSystem(-1)
            SlotAction.DOWN -> moveSystem(1)
            SlotAction.LAUNCH -> onActivate()
            SlotAction.BACK -> { /* stay on the shelf */ }
            SlotAction.PICK_FOLDER -> pickTree.launch(null)
            SlotAction.NONE -> {}
        }
    }

    companion object {
        private const val PREFS = "slot"
        private const val KEY_TREE = "rom_tree"
    }
}

private class LabelAdapter(
    private val onClick: (Int) -> Unit,
) : RecyclerView.Adapter<LabelAdapter.Holder>() {
    private var items: List<String> = emptyList()
    private var selected = 0

    fun submit(newItems: List<String>, newSelected: Int) {
        items = newItems
        selected = newSelected
        notifyDataSetChanged()
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): Holder {
        val v = LayoutInflater.from(parent.context).inflate(R.layout.item_label, parent, false)
        return Holder(v)
    }

    override fun getItemCount() = items.size

    override fun onBindViewHolder(holder: Holder, position: Int) {
        holder.label.text = items[position]
        holder.itemView.isSelected = position == selected
        holder.itemView.setOnClickListener { onClick(position) }
    }

    class Holder(view: View) : RecyclerView.ViewHolder(view) {
        val label: TextView = view.findViewById(R.id.label)
    }
}
