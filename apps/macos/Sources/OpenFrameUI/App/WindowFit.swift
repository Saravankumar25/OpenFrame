// Keeps the main window, and everything in it, inside the screen.
//
// SwiftUI turns the smallest size of a window's content into the window's own minimum. On a
// screen whose free area (without the menu bar and the Dock) is smaller than that minimum, the
// window ran past the bottom of the screen even when zoomed, and the foot of the sidebar (the
// Saved status) sat under the Dock. Here the minimum is 900 by 600 points, less only when the
// screen has less room, and the window's frame is brought back inside the screen at launch,
// after macOS restores it, when it moves to another screen, when the screen's resolution or
// scaling changes, and when a resize leaves it larger than the screen.

import AppKit
import SwiftUI

/// The minimum content size of one window, from the screen the window is on.
@MainActor @Observable
final class WindowFit {
    /// The minimum the app asks for on a screen with room for it.
    static let preferred = CGSize(width: 900, height: 600)

    private(set) var minimum = WindowFit.preferred
    /// The size of the window's content (below the toolbar), for sheets that must not be larger.
    private(set) var content = CGSize(width: 1360, height: 800)

    func update(for window: NSWindow) {
        let vf = WindowFit.visibleFrame(of: window)
        guard vf.width > 0, vf.height > 0 else { return }
        // The title bar and the toolbar sit above the SwiftUI content.
        let chrome = max(0, window.frame.height - window.contentLayoutRect.height)
        let fitting = CGSize(width: min(Self.preferred.width, vf.width.rounded(.down)),
                             height: min(Self.preferred.height, (vf.height - chrome).rounded(.down)))
        if fitting != minimum { minimum = fitting }
        let now = window.contentLayoutRect.size
        if now != content { content = now }
    }

    static func visibleFrame(of window: NSWindow) -> NSRect {
        (window.screen ?? NSScreen.main)?.visibleFrame ?? .zero
    }

    /// The window's frame brought inside the screen's visible frame. A size that fits is kept;
    /// a size that does not is reduced to the screen and the window moved onto it. `move` also
    /// brings a window that fits but lies partly off the screen back onto it (at launch and on a
    /// screen change; a window the person dragged partly away stays where they put it).
    static func fitted(_ frame: NSRect, in vf: NSRect, move: Bool) -> NSRect {
        guard vf.width > 0, vf.height > 0 else { return frame }
        var f = frame
        f.size.width = min(f.width, vf.width)
        f.size.height = min(f.height, vf.height)
        if move || f.size != frame.size {
            // A frame that was cut keeps its top left corner where it can; then it is moved onto the screen.
            if f.height != frame.height { f.origin.y = frame.maxY - f.height }
            f.origin.x = min(max(f.minX, vf.minX), vf.maxX - f.width)
            f.origin.y = min(max(f.minY, vf.minY), vf.maxY - f.height)
        }
        return f
    }
}

extension View {
    /// The root of the main window: the content minimum follows the screen and the window is
    /// kept on the screen.
    func fitsScreen() -> some View { modifier(FitsScreen()) }
}

private struct FitsScreen: ViewModifier {
    @State private var fit = WindowFit()

    func body(content: Content) -> some View {
        content
            .frame(minWidth: fit.minimum.width, minHeight: fit.minimum.height)
            .environment(\.windowContentSize, fit.content)
            .background(WindowFitter(fit: fit))
    }
}

/// The size of the main window's content, which sheets use so that they are never larger
/// than the window they belong to.
private struct WindowContentSizeKey: EnvironmentKey {
    static let defaultValue = CGSize(width: 1360, height: 800)
}

extension EnvironmentValues {
    var windowContentSize: CGSize {
        get { self[WindowContentSizeKey.self] }
        set { self[WindowContentSizeKey.self] = newValue }
    }
}

extension View {
    /// The frame of a sheet: its usual minimum and ideal size, never more than the window it
    /// belongs to leaves room for (a sheet larger than its window runs off the screen). Below its
    /// usual minimum the sheet's content scrolls or wraps instead.
    func sheetFrame(minWidth: CGFloat, idealWidth: CGFloat? = nil, minHeight: CGFloat? = nil, idealHeight: CGFloat? = nil) -> some View {
        modifier(SheetFrame(minWidth: minWidth, idealWidth: idealWidth, minHeight: minHeight, idealHeight: idealHeight))
    }
}

extension View {
    /// A sheet of a fixed size, made smaller (never larger) when the window it belongs to has
    /// less room; its content then scrolls or wraps.
    func sheetSize(width: CGFloat, height: CGFloat) -> some View {
        modifier(SheetSize(width: width, height: height))
    }
}

private struct SheetSize: ViewModifier {
    let width: CGFloat
    let height: CGFloat
    @Environment(\.windowContentSize) private var window

    func body(content: Content) -> some View {
        content.frame(width: min(width, max(320, window.width - 24)), height: min(height, max(240, window.height - 24)))
    }
}

private struct SheetFrame: ViewModifier {
    let minWidth: CGFloat
    let idealWidth: CGFloat?
    let minHeight: CGFloat?
    let idealHeight: CGFloat?
    @Environment(\.windowContentSize) private var window

    /// Room the sheet leaves around itself inside the window.
    private static let margin: CGFloat = 24

    func body(content: Content) -> some View {
        let maxW = max(320, window.width - Self.margin)
        let maxH = max(240, window.height - Self.margin)
        content.frame(minWidth: min(minWidth, maxW), idealWidth: idealWidth.map { min($0, maxW) },
                      maxWidth: maxW,
                      minHeight: minHeight.map { min($0, maxH) }, idealHeight: idealHeight.map { min($0, maxH) },
                      maxHeight: maxH)
    }
}

/// Finds the window the root view is in and keeps it on the screen.
private struct WindowFitter: NSViewRepresentable {
    let fit: WindowFit

    func makeNSView(context: Context) -> FitterView {
        let view = FitterView()
        view.fit = fit
        return view
    }

    func updateNSView(_ view: FitterView, context: Context) { view.fit = fit }

    final class FitterView: NSView {
        var fit: WindowFit?
        private var observers: [NSObjectProtocol] = []
        private weak var watched: NSWindow?
        private var adjusting = false

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            guard window !== watched else { return }
            stopWatching()
            guard let window else { return }
            watched = window
            let center = NotificationCenter.default
            let watch: (Notification.Name, Any?, Bool) -> Void = { name, object, move in
                self.observers.append(center.addObserver(forName: name, object: object, queue: .main) { [weak self] _ in
                    MainActor.assumeIsolated { self?.refit(move: move) }
                })
            }
            watch(NSWindow.didChangeScreenNotification, window, true)
            watch(NSWindow.didChangeBackingPropertiesNotification, window, true)
            watch(NSApplication.didChangeScreenParametersNotification, nil, true)
            watch(NSWindow.didResizeNotification, window, false)
            watch(NSWindow.didEndLiveResizeNotification, window, false)
            refit(move: true)
            // macOS restores the last frame after the window first shows; check again after it.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in self?.refit(move: true) }
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) { [weak self] in self?.refit(move: true) }
        }

        private func stopWatching() {
            observers.forEach(NotificationCenter.default.removeObserver)
            observers = []
        }

        /// Updates the content minimum, then (once SwiftUI has applied it) the frame.
        func refit(move: Bool) {
            guard let window = watched, !adjusting else { return }
            fit?.update(for: window)
            // A full-screen window belongs to macOS; a window in a live resize follows the pointer.
            guard !window.styleMask.contains(.fullScreen), !window.inLiveResize else { return }
            DispatchQueue.main.async { [weak self] in
                guard let self, let window = self.watched, !window.styleMask.contains(.fullScreen) else { return }
                let vf = WindowFit.visibleFrame(of: window)
                let target = WindowFit.fitted(window.frame, in: vf, move: move)
                guard target != window.frame else { return }
                self.adjusting = true
                window.setFrame(target, display: true, animate: false)
                self.adjusting = false
                self.fit?.update(for: window)
            }
        }
    }
}

/// The project sidebar comes back at every launch.
enum SidebarRestore {
    /// AppKit saves the sidebar's width and whether it was collapsed under this key, and on the
    /// next launch restores a collapsed sidebar behind SwiftUI's back: the window then showed no
    /// sidebar while its button still said Hide Sidebar. The saved width is kept; only the
    /// collapsed mark is cleared, before any window is made.
    static func clearCollapsed(in defaults: UserDefaults = .standard) {
        for key in defaults.dictionaryRepresentation().keys
        where key.hasPrefix("NSSplitView Subview Frames main") && key.hasSuffix("SidebarNavigationSplitView") {
            guard var frames = defaults.array(forKey: key) as? [String], let first = frames.first else { continue }
            var parts = first.components(separatedBy: ", ")
            guard parts.count >= 5, parts[4] == "YES" else { continue }
            parts[4] = "NO"
            frames[0] = parts.joined(separator: ", ")
            defaults.set(frames, forKey: key)
        }
    }
}
