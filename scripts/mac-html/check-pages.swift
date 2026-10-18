// Renders every MacHTML page in a WebKit view at the window's own size (the "#bare" view of the page) and
// compares the render with the capture picture. Prints a Markdown table; exits 1 when a page is over the threshold.
//
// Usage: swift scripts/mac-html/check-pages.swift <MacHTML folder> [max mean difference in percent, default 1.5]
import AppKit
import ImageIO
import WebKit

/// A window that keeps any size, even one wider than the screen.
final class FreeWindow: NSWindow {
    override func constrainFrameRect(_ frameRect: NSRect, to screen: NSScreen?) -> NSRect { frameRect }
}

final class Checker: NSObject, WKNavigationDelegate {
    let dir: URL
    let threshold: Double
    var pages: [(file: String, name: String, w: Int, h: Int)] = []
    var results: [(String, Double)] = []
    var index = 0
    let window = FreeWindow(contentRect: NSRect(x: 0, y: 0, width: 1600, height: 1000), styleMask: [.borderless], backing: .buffered, defer: false)
    var web: WKWebView!

    init(dir: URL, threshold: Double) {
        self.dir = dir
        self.threshold = threshold
        super.init()
    }

    func load() {
        guard let data = try? Data(contentsOf: dir.appendingPathComponent("pages.json")),
              let list = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else { fputs("no pages.json\n", stderr); exit(2) }
        for p in list {
            guard let f = p["file"] as? String, let n = p["name"] as? String else { continue }
            let w = (p["width"] as? Double).map(Int.init) ?? 1440, h = (p["height"] as? Double).map(Int.init) ?? 652
            pages.append((f, n, w, h))
        }
        let cfg = WKWebViewConfiguration()
        web = WKWebView(frame: NSRect(x: 0, y: 0, width: 1600, height: 1000), configuration: cfg)
        web.navigationDelegate = self
        window.contentView = web
        window.setFrameOrigin(NSPoint(x: -4000, y: 0))
        window.orderBack(nil)
        next()
    }

    func next() {
        if index >= pages.count { finish(); return }
        let p = pages[index]
        web.setFrameSize(NSSize(width: p.w, height: p.h))
        window.setFrame(NSRect(x: -4000, y: 0, width: CGFloat(p.w), height: CGFloat(p.h)), display: true)
        var comps = URLComponents(url: dir.appendingPathComponent(p.file), resolvingAgainstBaseURL: false)!
        comps.fragment = "bare"
        web.loadFileURL(comps.url!, allowingReadAccessTo: dir)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        let p = pages[index]
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) {
            let cfg = WKSnapshotConfiguration()
            cfg.rect = CGRect(x: 0, y: 0, width: p.w, height: p.h)
            cfg.snapshotWidth = NSNumber(value: p.w)
            webView.takeSnapshot(with: cfg) { image, error in
                var diff = 100.0
                if let image, let d = self.compare(image, self.dir.appendingPathComponent("assets/\(p.name).png"), p.w * 2, p.h * 2) { diff = d }
                if let d = ProcessInfo.processInfo.environment["CHECK_DUMP"], let image, let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
                   let png = rep.representation(using: .png, properties: [:]) { try? png.write(to: URL(fileURLWithPath: d).appendingPathComponent("\(p.name).png")) }
                self.results.append((p.name, diff))
                self.index += 1
                self.next()
            }
        }
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        results.append((pages[index].name, 100)); index += 1; next()
    }

    /// Mean absolute difference over all channels, in percent, at the picture's own (2x) size.
    func compare(_ snapshot: NSImage, _ pngURL: URL, _ w: Int, _ h: Int) -> Double? {
        func pixels(_ cg: CGImage?) -> [UInt8]? {
            guard let cg, let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
            ctx.interpolationQuality = .high
            ctx.setFillColor(NSColor.black.cgColor); ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
            ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
            guard let data = ctx.data else { return nil }
            return Array(UnsafeBufferPointer(start: data.assumingMemoryBound(to: UInt8.self), count: w * h * 4))
        }
        // Full-resolution pictures, not the point-size rendition an NSImage offers.
        var snapCG: CGImage?
        if let tiff = snapshot.tiffRepresentation, let src = CGImageSourceCreateWithData(tiff as CFData, nil) { snapCG = CGImageSourceCreateImageAtIndex(src, 0, nil) }
        var pngCG: CGImage?
        if let src = CGImageSourceCreateWithURL(pngURL as CFURL, nil) { pngCG = CGImageSourceCreateImageAtIndex(src, 0, nil) }
        guard let a = pixels(snapCG), let b = pixels(pngCG) else { return nil }
        var total = 0
        for i in stride(from: 0, to: a.count, by: 4) {
            total += abs(Int(a[i]) - Int(b[i])) + abs(Int(a[i + 1]) - Int(b[i + 1])) + abs(Int(a[i + 2]) - Int(b[i + 2]))
        }
        return Double(total) / Double(w * h * 3) / 255 * 100
    }

    func finish() {
        var out = "| Page | Mean difference |\n|---|---|\n"
        var failed = 0
        for (name, d) in results {
            let bad = d > threshold
            if bad { failed += 1 }
            out += String(format: "| %@ | %.3f%%%@ |\n", name, d, bad ? " FAIL" : "")
        }
        let mean = results.isEmpty ? 0 : results.map(\.1).reduce(0, +) / Double(results.count)
        out += String(format: "\n%d pages, mean difference %.3f%%, %d over %.2f%%.\n", results.count, mean, failed, threshold)
        print(out)
        try? out.write(to: dir.appendingPathComponent("check-results.md"), atomically: true, encoding: .utf8)
        exit(failed == 0 ? 0 : 1)
    }
}

let args = CommandLine.arguments
guard args.count >= 2 else { fputs("usage: check-pages.swift <MacHTML folder> [threshold percent]\n", stderr); exit(2) }
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let checker = Checker(dir: URL(fileURLWithPath: args[1]), threshold: args.count > 2 ? Double(args[2]) ?? 1.5 : 1.5)
DispatchQueue.main.async { checker.load() }
app.run()
