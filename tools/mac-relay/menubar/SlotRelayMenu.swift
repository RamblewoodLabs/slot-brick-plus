// Slot Relay menu bar item: shows whether the Mac relay is running and turns it on or off.
// It only drives the com.slot.relay launchd job. It never reads or shows the token.
import AppKit
import Darwin

let relayLabel = "com.slot.relay"
let home = FileManager.default.homeDirectoryForCurrentUser
let relayPlist = home.appendingPathComponent("Library/LaunchAgents/\(relayLabel).plist").path
let configPath = home.appendingPathComponent("Library/Application Support/SlotRelay/config.json").path
let logsFolder = home.appendingPathComponent("Library/Logs")
let relayFolder = home.appendingPathComponent("Library/Application Support/SlotRelay")
let domain = "gui/\(getuid())"

enum RelayState { case on, starting, off }

@discardableResult
func launchctl(_ args: [String]) -> (Int32, String) {
    let p = Process()
    p.executableURL = URL(fileURLWithPath: "/bin/launchctl")
    p.arguments = args
    let pipe = Pipe()
    p.standardOutput = pipe
    p.standardError = pipe
    do { try p.run() } catch { return (-1, "") }
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    p.waitUntilExit()
    return (p.terminationStatus, String(data: data, encoding: .utf8) ?? "")
}

/// Only the port is read from config.json; the token is never touched.
func relayPort() -> Int {
    guard let data = FileManager.default.contents(atPath: configPath),
          let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          let port = obj["port"] as? Int else { return 8787 }
    return port
}

func lanAddress() -> String {
    var best = ""
    var ifaddr: UnsafeMutablePointer<ifaddrs>?
    guard getifaddrs(&ifaddr) == 0, let first = ifaddr else { return "unknown" }
    defer { freeifaddrs(ifaddr) }
    for ptr in sequence(first: first, next: { $0.pointee.ifa_next }) {
        let ifa = ptr.pointee
        guard let sa = ifa.ifa_addr, sa.pointee.sa_family == UInt8(AF_INET) else { continue }
        let name = String(cString: ifa.ifa_name)
        guard name.hasPrefix("en") else { continue }
        var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
        if getnameinfo(sa, socklen_t(sa.pointee.sa_len), &host, socklen_t(host.count), nil, 0, NI_NUMERICHOST) == 0 {
            let ip = String(cString: host)
            if name == "en0" { return ip }
            if best.isEmpty { best = ip }
        }
    }
    return best.isEmpty ? "unknown" : best
}

func localHostName() -> String {
    let p = Process()
    p.executableURL = URL(fileURLWithPath: "/usr/sbin/scutil")
    p.arguments = ["--get", "LocalHostName"]
    let pipe = Pipe()
    p.standardOutput = pipe
    p.standardError = Pipe()
    guard (try? p.run()) != nil else { return "" }
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    p.waitUntilExit()
    let name = (String(data: data, encoding: .utf8) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
    return name.isEmpty ? "" : name + ".local"
}

/// Any HTTP answer (the relay replies 401 without a token) means it is serving.
func relayAnswers(port: Int) -> Bool {
    guard let url = URL(string: "http://127.0.0.1:\(port)/jobs/ping") else { return false }
    var req = URLRequest(url: url)
    req.timeoutInterval = 2
    let sem = DispatchSemaphore(value: 0)
    var ok = false
    URLSession.shared.dataTask(with: req) { _, resp, _ in
        ok = resp is HTTPURLResponse
        sem.signal()
    }.resume()
    _ = sem.wait(timeout: .now() + 3)
    return ok
}

func jobLoaded() -> Bool { launchctl(["print", "\(domain)/\(relayLabel)"]).0 == 0 }

func currentState(port: Int) -> RelayState {
    if relayAnswers(port: port) { return .on }
    return jobLoaded() ? .starting : .off
}

func turnOn() {
    launchctl(["enable", "\(domain)/\(relayLabel)"])
    if !jobLoaded() { launchctl(["bootstrap", domain, relayPlist]) }
    launchctl(["kickstart", "\(domain)/\(relayLabel)"])
}

/// Off survives logins and restarts until turned back on: disable, then unload.
func turnOff() {
    launchctl(["disable", "\(domain)/\(relayLabel)"])
    launchctl(["bootout", "\(domain)/\(relayLabel)"])
}

final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    let menu = NSMenu()
    let statusLine = NSMenuItem(title: "Relay: …", action: nil, keyEquivalent: "")
    let portLine = NSMenuItem(title: "", action: nil, keyEquivalent: "")
    let hostLine = NSMenuItem(title: "", action: nil, keyEquivalent: "")
    let toggle = NSMenuItem(title: "Turn On", action: #selector(toggleRelay), keyEquivalent: "")
    var state: RelayState = .off
    var busy = false
    var timer: Timer?
    let queue = DispatchQueue(label: "slot.relay.menu")

    func applicationDidFinishLaunching(_ note: Notification) {
        for line in [statusLine, portLine, hostLine] { line.isEnabled = false; menu.addItem(line) }
        menu.addItem(.separator())
        toggle.target = self
        menu.addItem(toggle)
        menu.addItem(.separator())
        let logs = NSMenuItem(title: "Open Logs Folder", action: #selector(openLogs), keyEquivalent: "")
        logs.target = self
        menu.addItem(logs)
        let folder = NSMenuItem(title: "Open Relay Folder", action: #selector(openFolder), keyEquivalent: "")
        folder.target = self
        menu.addItem(folder)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit Slot Relay Menu", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        menu.delegate = self
        item.menu = menu
        render()
        refresh()
        timer = Timer.scheduledTimer(withTimeInterval: 5, repeats: true) { [weak self] _ in self?.refresh() }
    }

    func menuWillOpen(_ menu: NSMenu) { refresh() }

    func refresh() {
        queue.async {
            let port = relayPort()
            let s = currentState(port: port)
            let ip = lanAddress()
            let name = localHostName()
            DispatchQueue.main.async {
                self.state = s
                self.portLine.title = "Port \(port)"
                self.hostLine.title = name.isEmpty ? "Host \(ip)" : "Host \(ip)  (\(name))"
                self.render()
            }
        }
    }

    func render() {
        let symbol: String
        let title: String
        switch state {
        case .on: symbol = "play.tv.fill"; title = "Relay: On"
        case .starting: symbol = "play.tv"; title = "Relay: Starting…"
        case .off: symbol = "tv.slash"; title = "Relay: Off"
        }
        let img = NSImage(systemSymbolName: symbol, accessibilityDescription: title)
        img?.isTemplate = true
        item.button?.image = img
        item.button?.appearsDisabled = (state == .off)
        item.button?.toolTip = "Slot \(title)"
        statusLine.title = busy ? "Relay: Working…" : title
        toggle.title = (state == .off) ? "Turn On" : "Turn Off"
        toggle.isEnabled = !busy
    }

    @objc func toggleRelay() {
        let wasOff = (state == .off)
        busy = true
        render()
        queue.async {
            if wasOff { turnOn() } else { turnOff() }
            Thread.sleep(forTimeInterval: 1.5)
            DispatchQueue.main.async { self.busy = false; self.refresh() }
        }
    }

    @objc func openLogs() { NSWorkspace.shared.open(logsFolder) }
    @objc func openFolder() { NSWorkspace.shared.open(relayFolder) }
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
