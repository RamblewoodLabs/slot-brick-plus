import AppKit
import Foundation

/// Menu bar control for the Slot sync hub. It reads /status and index.json only.
/// NSStatusItem is the AppKit status item (what SwiftUI calls MenuBarExtra). swiftc can build it
/// without an Xcode project.

private let hubLabel = "com.slot.sync"
private let defaultPort = 8791

private struct DeviceSync {
    var device: String
    var system: String
    var game: String
    var mtime: Int64
}

private final class SlotSyncMenu: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private let status = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let store = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Application Support/SlotSync", isDirectory: true)

    func applicationDidFinishLaunching(_ notification: Notification) {
        let menu = NSMenu()
        menu.delegate = self
        status.menu = menu
        refresh()
    }

    func menuWillOpen(_ menu: NSMenu) {
        refresh()
    }

    private func refresh() {
        let running = hubIsRunning()
        applyIcon(running: running)
        let live = fetchStatus()
        let host = live?.host ?? localHost()
        let port = live?.port ?? defaultPort
        let devices = live?.devices ?? devicesFromIndex()
        rebuild(running: running, host: host, port: port, devices: devices)
    }

    private func applyIcon(running: Bool) {
        guard let button = status.button else { return }
        button.image = icon(running: running)
        button.appearsDisabled = !running
    }

    private func rebuild(running: Bool, host: String, port: Int, devices: [DeviceSync]) {
        guard let menu = status.menu else { return }
        menu.removeAllItems()
        let state = running ? "On" : "Off"
        let summary = menu.addItem(withTitle: "Hub: \(state) · \(port) · \(host)", action: nil, keyEquivalent: "")
        summary.isEnabled = false
        menu.addItem(.separator())
        if devices.isEmpty {
            let none = menu.addItem(withTitle: "No sync yet", action: nil, keyEquivalent: "")
            none.isEnabled = false
        } else {
            for row in devices {
                let when = Self.when(row.mtime)
                let title = "\(row.device) · \(row.game) · \(when)"
                let item = menu.addItem(withTitle: title, action: nil, keyEquivalent: "")
                item.isEnabled = false
            }
        }
        menu.addItem(.separator())
        let on = menu.addItem(withTitle: "Turn On", action: #selector(turnOn), keyEquivalent: "")
        on.target = self
        on.isEnabled = !running
        let off = menu.addItem(withTitle: "Turn Off", action: #selector(turnOff), keyEquivalent: "")
        off.target = self
        off.isEnabled = running
        menu.addItem(.separator())
        let folder = menu.addItem(withTitle: "Open Sync Folder", action: #selector(openFolder), keyEquivalent: "")
        folder.target = self
        let quit = menu.addItem(withTitle: "Quit", action: #selector(quit), keyEquivalent: "q")
        quit.target = self
    }

    @objc private func turnOn() {
        let uid = Self.uid()
        let target = "gui/\(uid)/\(hubLabel)"
        let plist = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/LaunchAgents/\(hubLabel).plist")
        launchctl(["enable", target])
        _ = launchctl(["bootstrap", "gui/\(uid)", plist.path])
        launchctl(["kickstart", "-k", target])
        refresh()
    }

    @objc private func turnOff() {
        let target = "gui/\(Self.uid())/\(hubLabel)"
        launchctl(["disable", target])
        launchctl(["bootout", target])
        refresh()
    }

    @objc private func openFolder() {
        try? FileManager.default.createDirectory(at: store, withIntermediateDirectories: true)
        NSWorkspace.shared.open(store)
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }

    private func hubIsRunning() -> Bool {
        let output = launchctlOutput(["print", "gui/\(Self.uid())/\(hubLabel)"])
        return output.contains("state = running")
    }

    private func fetchStatus() -> (host: String, port: Int, devices: [DeviceSync])? {
        guard let url = URL(string: "http://127.0.0.1:\(defaultPort)/status") else { return nil }
        var request = URLRequest(url: url, timeoutInterval: 0.4)
        request.httpMethod = "GET"
        let session = URLSession(configuration: .ephemeral)
        let wait = DispatchSemaphore(value: 0)
        var payload: Data?
        session.dataTask(with: request) { data, _, _ in
            payload = data
            wait.signal()
        }.resume()
        _ = wait.wait(timeout: .now() + 0.5)
        guard let payload, let parsed = Self.parseStatus(payload) else { return nil }
        return parsed
    }

    private func devicesFromIndex() -> [DeviceSync] {
        let url = store.appendingPathComponent("index.json")
        guard let data = try? Data(contentsOf: url) else { return [] }
        return Self.devices(inIndex: data)
    }

    private func localHost() -> String {
        let text = command("/usr/sbin/scutil", ["--get", "LocalHostName"])
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if text.isEmpty { return "localhost" }
        return text + ".local"
    }

    private static func uid() -> String {
        String(getuid())
    }

    @discardableResult
    private func launchctl(_ args: [String]) -> Int32 {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/launchctl")
        process.arguments = args
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        do { try process.run() } catch { return 1 }
        process.waitUntilExit()
        return process.terminationStatus
    }

    private func launchctlOutput(_ args: [String]) -> String {
        command("/bin/launchctl", args)
    }

    private func command(_ path: String, _ args: [String]) -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: path)
        process.arguments = args
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        do { try process.run() } catch { return "" }
        process.waitUntilExit()
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        return String(data: data, encoding: .utf8) ?? ""
    }

    static func when(_ nanos: Int64) -> String {
        let seconds = Double(nanos) / 1_000_000_000
        let date = Date(timeIntervalSince1970: seconds)
        let format = DateFormatter()
        format.dateStyle = .none
        format.timeStyle = .short
        return format.string(from: date)
    }

    static func parseStatus(_ data: Data) -> (host: String, port: Int, devices: [DeviceSync])? {
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        let host = json["host"] as? String ?? "localhost"
        let port = json["port"] as? Int ?? defaultPort
        let rows = json["devices"] as? [[String: Any]] ?? []
        return (host, port, devices(from: rows))
    }

    static func devices(inIndex data: Data) -> [DeviceSync] {
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let files = json["files"] as? [String: [String: Any]] else { return [] }
        var rows: [[String: Any]] = []
        for (path, meta) in files {
            var row = meta
            row["path"] = path
            rows.append(row)
        }
        return newest(rows)
    }

    static func devices(from rows: [[String: Any]]) -> [DeviceSync] {
        rows.compactMap { row in
            guard let device = row["device"] as? String, let game = row["game"] as? String else { return nil }
            let system = row["system"] as? String ?? ""
            let mtime = (row["mtime"] as? NSNumber)?.int64Value ?? 0
            return DeviceSync(device: device, system: system, game: game, mtime: mtime)
        }
    }

    /// Same rule as the hub: newest mtime per device, and only save or state paths name a game.
    static func newest(_ rows: [[String: Any]]) -> [DeviceSync] {
        var best: [String: DeviceSync] = [:]
        for row in rows {
            guard let path = row["path"] as? String, let named = game(of: path) else { continue }
            let device = row["device"] as? String ?? ""
            let mtime = (row["mtime"] as? NSNumber)?.int64Value ?? 0
            if let prev = best[device], prev.mtime > mtime { continue }
            best[device] = DeviceSync(device: device, system: named.system, game: named.game, mtime: mtime)
        }
        return best.values.sorted { $0.device < $1.device }
    }

    static func game(of path: String) -> (system: String, game: String)? {
        let parts = path.split(separator: "/").map(String.init)
        if parts.count == 3, parts[0] == "Saves" {
            let file = parts[2]
            let stem = file.split(separator: ".").dropLast().joined(separator: ".")
            return (parts[1], stem.isEmpty ? file : stem)
        }
        if parts.count == 5, parts[0] == "States" {
            return (parts[1], parts[3])
        }
        return nil
    }
}

private func icon(running: Bool) -> NSImage {
    let symbol = NSImage(systemSymbolName: "arrow.triangle.2.circlepath", accessibilityDescription: "Slot Sync")
    let base = symbol ?? NSImage(size: NSSize(width: 18, height: 18))
    base.isTemplate = true
    if running { return base }
    let size = NSSize(width: 18, height: 18)
    let slashed = NSImage(size: size, flipped: false) { rect in
        base.draw(in: rect)
        NSColor.black.setStroke()
        let slash = NSBezierPath()
        slash.lineWidth = 1.6
        slash.move(to: NSPoint(x: rect.minX + 2, y: rect.minY + 2))
        slash.line(to: NSPoint(x: rect.maxX - 2, y: rect.maxY - 2))
        slash.stroke()
        return true
    }
    slashed.isTemplate = true
    return slashed
}

let app = NSApplication.shared
let delegate = SlotSyncMenu()
app.setActivationPolicy(.accessory)
app.delegate = delegate
app.run()
