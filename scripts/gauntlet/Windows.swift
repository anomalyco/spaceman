// Disposable test windows: each is its own process of this bundle, so it has its own pid,
// its own app, and nothing of the user's is ever touched.
import AppKit
import Foundation

/// `gauntlet window --label A --dir DIR --title TITLE`: one standard, resizable window that obeys
/// commands written to `DIR/A.cmd` when it receives SIGUSR1.
final class WindowApp: NSObject, NSApplicationDelegate {
    private let label: String
    private let directory: URL
    private let title: String
    private var window: NSWindow!
    private var signalSource: DispatchSourceSignal?

    init(label: String, directory: URL, title: String) {
        self.label = label
        self.directory = directory
        self.title = title
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let screen = NSScreen.screens.first?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1280, height: 800)
        let offset = CGFloat((label.unicodeScalars.first?.value ?? 0) % 5) * 40
        let frame = NSRect(x: screen.minX + 80 + offset, y: screen.minY + 80 + offset, width: 520, height: 360)
        window = NSWindow(
            contentRect: frame,
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        window.title = title
        window.isRestorable = false
        window.isReleasedWhenClosed = false
        let hue = CGFloat((label.unicodeScalars.first?.value ?? 0) % 7) / 7
        window.backgroundColor = NSColor(hue: hue, saturation: 0.45, brightness: 0.35, alpha: 1)
        let text = NSTextField(labelWithString: label)
        text.font = .systemFont(ofSize: 120, weight: .heavy)
        text.textColor = .white
        text.translatesAutoresizingMaskIntoConstraints = false
        window.contentView?.addSubview(text)
        if let content = window.contentView {
            NSLayoutConstraint.activate([
                text.centerXAnchor.constraint(equalTo: content.centerXAnchor),
                text.centerYAnchor.constraint(equalTo: content.centerYAnchor),
            ])
        }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate()

        signal(SIGUSR1, SIG_IGN)
        let source = DispatchSource.makeSignalSource(signal: SIGUSR1, queue: .main)
        source.setEventHandler { [weak self] in self?.handleCommand() }
        source.resume()
        signalSource = source
        write("\(getpid()) \(window.windowNumber)\n", to: "\(label).pid")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    private func handleCommand() {
        let url = directory.appendingPathComponent("\(label).cmd")
        guard let command = try? String(contentsOf: url, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
        else { return }
        try? FileManager.default.removeItem(at: url)
        switch command {
        case "minimize": window.miniaturize(nil)
        case "deminimize": window.deminiaturize(nil)
        case "hide": NSApp.hide(nil)
        case "unhide": NSApp.unhide(nil)
        case "quit": NSApp.terminate(nil)
        default: break
        }
        write("\(command)\n", to: "\(label).ack")
    }

    private func write(_ text: String, to name: String) {
        try? text.write(to: directory.appendingPathComponent(name), atomically: true, encoding: .utf8)
    }
}

func runWindowMode(label: String, directory: URL, title: String) -> Never {
    let app = NSApplication.shared
    app.setActivationPolicy(.regular)
    let delegate = WindowApp(label: label, directory: directory, title: title)
    app.delegate = delegate
    app.run()
    exit(0)
}

/// The orchestrator's handle on one disposable window.
final class TestWindow {
    let label: String
    let pid: pid_t
    let id: UInt32
    private let directory: URL

    init(label: String, pid: pid_t, id: UInt32, directory: URL) {
        self.label = label
        self.pid = pid
        self.id = id
        self.directory = directory
    }

    var native: NativeWindow? { Native.windows()[id] }

    /// Delivers one command and waits (up to a second) for the window to acknowledge it.
    func send(_ command: String) {
        let ack = directory.appendingPathComponent("\(label).ack")
        try? FileManager.default.removeItem(at: ack)
        try? "\(command)\n".write(
            to: directory.appendingPathComponent("\(label).cmd"), atomically: true, encoding: .utf8)
        kill(pid, SIGUSR1)
        let deadline = Date(timeIntervalSinceNow: 1)
        while Date() < deadline, !FileManager.default.fileExists(atPath: ack.path) {
            usleep(2_000)
        }
    }

    /// Opens a new window process with `open -n`, so LaunchServices owns it like any app the user opens.
    static func spawn(label: String, bundle: URL, directory: URL, title: String) -> Result<TestWindow, Problem> {
        let pidFile = directory.appendingPathComponent("\(label).pid")
        try? FileManager.default.removeItem(at: pidFile)
        let opened = run(
            "/usr/bin/open",
            ["-n", "-a", bundle.path, "--args", "window", "--label", label, "--dir", directory.path, "--title", title])
        guard opened.status == 0 else { return .failure(Problem("open failed: \(opened.err)")) }
        let deadline = Date(timeIntervalSinceNow: 15)
        while Date() < deadline {
            if let text = try? String(contentsOf: pidFile, encoding: .utf8) {
                let parts = text.split(separator: " ").map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                if parts.count == 2, let pid = pid_t(parts[0]), let id = UInt32(parts[1]),
                    Native.windows()[id]?.pid == pid
                {
                    return .success(TestWindow(label: label, pid: pid, id: id, directory: directory))
                }
            }
            Native.pump(0.05)
        }
        return .failure(Problem("window \(label) did not report its pid and window number within 15 s"))
    }
}
