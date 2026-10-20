// Checks the MacHTML pages of one mode folder (MacHTML/light or MacHTML/dark).
//
// 1. Pixels: each page is rendered in WebKit in its normal view (picture visible) and the window part is taken at the
//    picture's own scale (2x). A pixel differs when any colour channel differs by more than 24 of 255. A page fails when
//    more than 0.5 percent of its pixels differ. Pixels where the picture itself is not fully opaque (the window's
//    rounded corners) are not compared.
// 2. HTML layer: every non-empty label and value of the accessibility tree (listed in pages.json) must appear in the page
//    as text, a control value, an option, an aria-label, aria-valuenow or a title. Any miss fails the page.
//
// Writes check-results.md in the folder and prints it. Exits 1 when a page fails.
// Usage: swift scripts/mac-html/check-pages.swift <MacHTML/light or MacHTML/dark> [percent, default 0.5] [channel, default 24]
// (For hundreds of pages, compile first: swiftc -O check-pages.swift -o check-pages; capture.sh does this.)
import AppKit
import ImageIO
import WebKit

/// A window that keeps any size and stays off screen.
final class FreeWindow: NSWindow {
    override func constrainFrameRect(_ frameRect: NSRect, to screen: NSScreen?) -> NSRect { frameRect }
}

struct Page {
    let file: String, name: String, picture: String
    let w: Int, h: Int, scale: Int
    let expect: [String]
}

struct Result {
    let name: String
    let differing: Double
    let misses: [String]
    let note: String
}

final class Checker: NSObject, WKNavigationDelegate {
    let dir: URL
    let maxPercent: Double
    let channel: Int
    var pages: [Page] = []
    var results: [Result] = []
    var index = 0
    let window = FreeWindow(contentRect: NSRect(x: -6000, y: 0, width: 1600, height: 1200), styleMask: [.borderless], backing: .buffered, defer: false)
    var web: WKWebView!
    var timeout: DispatchWorkItem?

    init(dir: URL, maxPercent: Double, channel: Int) {
        self.dir = dir
        self.maxPercent = maxPercent
        self.channel = channel
        super.init()
    }

    func start() {
        guard let data = try? Data(contentsOf: dir.appendingPathComponent("pages.json")),
              let list = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
            fputs("no pages.json in \(dir.path)\n", stderr); exit(2)
        }
        // CHECK_ONLY=name,name checks only those pages; CHECK_DUMP=<folder> saves each render there.
        let only = Set((ProcessInfo.processInfo.environment["CHECK_ONLY"] ?? "").split(separator: ",").map(String.init))
        for p in list {
            if !only.isEmpty, let n = p["name"] as? String, !only.contains(n) { continue }
            guard let f = p["file"] as? String, let n = p["name"] as? String, let pic = p["picture"] as? String else { continue }
            pages.append(Page(file: f, name: n, picture: pic, w: (p["width"] as? Int) ?? 1280, h: (p["height"] as? Int) ?? 630,
                              scale: (p["scale"] as? Int) ?? 2, expect: (p["expect"] as? [String]) ?? []))
        }
        let cfg = WKWebViewConfiguration()
        cfg.suppressesIncrementalRendering = true
        web = WKWebView(frame: NSRect(x: 0, y: 0, width: 1600, height: 1200), configuration: cfg)
        web.navigationDelegate = self
        window.contentView = web
        window.orderBack(nil)
        next()
    }

    func next() {
        timeout?.cancel()
        if index >= pages.count { finish(); return }
        let p = pages[index]
        let size = NSSize(width: max(p.w + 200, 1000), height: p.h + 400)
        window.setContentSize(size)
        web.frame = NSRect(origin: .zero, size: size)
        let item = DispatchWorkItem { [weak self] in self?.record(Result(name: p.name, differing: 100, misses: [], note: "did not load")) }
        timeout = item
        DispatchQueue.main.asyncAfter(deadline: .now() + 20, execute: item)
        web.loadFileURL(dir.appendingPathComponent(p.file), allowingReadAccessTo: dir)
    }

    func record(_ r: Result) {
        // A late answer for a page that already has its result (after a time-out) is dropped.
        guard index < pages.count, pages[index].name == r.name, results.count == index else { return }
        timeout?.cancel()
        results.append(r)
        index += 1
        if index % 25 == 0 { fputs("checked \(index) of \(pages.count)\n", stderr) }
        DispatchQueue.main.async { self.next() }
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        record(Result(name: pages[index].name, differing: 100, misses: [], note: "load failed"))
    }

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        record(Result(name: pages[index].name, differing: 100, misses: [], note: "load failed"))
    }

    /// Runs in the page: waits until the picture is decoded and drawn, then looks for every expected string.
    static let probe = """
      const urls = new Set();
      for (const r of document.querySelectorAll('.region')) {
        const m = /url\\("?([^")]*)"?\\)/.exec(getComputedStyle(r).backgroundImage);
        if (m) urls.add(m[1]);
      }
      for (const u of urls) { const i = new Image(); i.src = u; try { await i.decode(); } catch (e) {} }
      await new Promise((r) => setTimeout(r, 30));
      const hay = [];
      const add = (s) => { if (s) hay.push(String(s).replace(/\\s+/g, ' ').trim()); };
      const win = document.querySelector('.win');
      for (const el of win.querySelectorAll('*')) {
        add(el.getAttribute('aria-label')); add(el.getAttribute('title')); add(el.getAttribute('aria-valuenow'));
        if ('value' in el && typeof el.value === 'string') add(el.value);
        if (el.tagName === 'OPTION') add(el.textContent);
        if (!el.children.length) add(el.textContent);
      }
      const all = hay.join('\\n');
      const miss = [];
      for (const s of expect) {
        const t = String(s).replace(/\\s+/g, ' ').trim();
        if (t && !all.includes(t)) miss.push(t.slice(0, 60));
      }
      const r = win.getBoundingClientRect();
      return JSON.stringify({ x: r.left + window.scrollX, y: r.top + window.scrollY, miss });
    """

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        let p = pages[index]
        webView.callAsyncJavaScript(Checker.probe, arguments: ["expect": p.expect], in: nil, in: .page) { outcome in
            guard case .success(let value) = outcome, let s = value as? String, let d = s.data(using: .utf8),
                  let o = try? JSONSerialization.jsonObject(with: d) as? [String: Any],
                  let x = (o["x"] as? NSNumber)?.doubleValue, let y = (o["y"] as? NSNumber)?.doubleValue else {
                var why = "?"
                if case .failure(let e) = outcome { why = e.localizedDescription }
                self.record(Result(name: p.name, differing: 100, misses: [], note: "probe failed: \(why)"))
                return
            }
            let misses = (o["miss"] as? [String]) ?? []
            self.snap(p, x, y, misses, attempt: 0)
        }
    }

    /// WebKit may take the snapshot before the picture is painted; a page over the limit is taken again a few times.
    func snap(_ p: Page, _ x: Double, _ y: Double, _ misses: [String], attempt: Int) {
        let cfg = WKSnapshotConfiguration()
        cfg.rect = CGRect(x: x, y: y, width: Double(p.w), height: Double(p.h))
        cfg.snapshotWidth = NSNumber(value: Double(p.w * p.scale) / Double(window.backingScaleFactor))
        cfg.afterScreenUpdates = true
        web.takeSnapshot(with: cfg) { image, _ in
            var diff = 100.0
            var note = "no snapshot"
            if let image, let d = self.compare(image, self.dir.appendingPathComponent(p.picture), p.w * p.scale, p.h * p.scale) { diff = d.0; note = d.1 }
            if diff > self.maxPercent && attempt < 4 {
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.25 * Double(attempt + 1)) { self.snap(p, x, y, misses, attempt: attempt + 1) }
                return
            }
            if let image, let dump = ProcessInfo.processInfo.environment["CHECK_DUMP"], let tiff = image.tiffRepresentation,
               let rep = NSBitmapImageRep(data: tiff), let png = rep.representation(using: .png, properties: [:]) {
                try? png.write(to: URL(fileURLWithPath: dump).appendingPathComponent("\(p.name).png"))
            }
            self.record(Result(name: p.name, differing: diff, misses: misses, note: note))
        }
    }

    /// Percent of pixels that differ by more than `channel` in any colour channel, at the picture's own size.
    func compare(_ snapshot: NSImage, _ pngURL: URL, _ w: Int, _ h: Int) -> (Double, String)? {
        let space = CGColorSpace(name: CGColorSpace.sRGB)!
        func pixels(_ cg: CGImage?) -> [UInt8]? {
            guard let cg else { return nil }
            var buf = [UInt8](repeating: 0, count: w * h * 4)
            let ok = buf.withUnsafeMutableBytes { raw -> Bool in
                guard let ctx = CGContext(data: raw.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: space,
                                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return false }
                ctx.interpolationQuality = .none
                ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))
                return true
            }
            return ok ? buf : nil
        }
        var best: CGImage?
        for rep in snapshot.representations {
            if let b = rep as? NSBitmapImageRep, let cg = b.cgImage, cg.width > (best?.width ?? 0) { best = cg }
        }
        if best == nil { var r = CGRect(x: 0, y: 0, width: w, height: h); best = snapshot.cgImage(forProposedRect: &r, context: nil, hints: nil) }
        var pic: CGImage?
        if let src = CGImageSourceCreateWithURL(pngURL as CFURL, nil) { pic = CGImageSourceCreateImageAtIndex(src, 0, nil) }
        let note = best.map { $0.width == w ? "" : "render was \($0.width) px wide, picture \(w)" } ?? ""
        guard let a = pixels(best), let b = pixels(pic) else { return nil }
        var differing = 0
        var x0 = w, y0 = h, x1 = -1, y1 = -1
        let lim = channel
        a.withUnsafeBufferPointer { pa in
            b.withUnsafeBufferPointer { pb in
                var i = 0
                let n = w * h * 4
                while i < n {
                    if pb[i + 3] == 255 {
                        let d0 = abs(Int(pa[i]) - Int(pb[i])), d1 = abs(Int(pa[i + 1]) - Int(pb[i + 1])), d2 = abs(Int(pa[i + 2]) - Int(pb[i + 2]))
                        if d0 > lim || d1 > lim || d2 > lim {
                            differing += 1
                            let px = (i / 4) % w, py = (i / 4) / w
                            x0 = min(x0, px); x1 = max(x1, px); y0 = min(y0, py); y1 = max(y1, py)
                        }
                    }
                    i += 4
                }
            }
        }
        let s = w / max(1, Int(snapshot.size.width.rounded()))
        let box = differing == 0 ? "" : "differences between \(x0 / max(s, 1)),\(y0 / max(s, 1)) and \(x1 / max(s, 1)),\(y1 / max(s, 1)) points"
        return (Double(differing) / Double(w * h) * 100, [note, box].filter { !$0.isEmpty }.joined(separator: "; "))
    }

    func finish() {
        var out = "| Page | Differing pixels | HTML misses | Result |\n|---|---|---|---|\n"
        var failed = 0, missTotal = 0
        for r in results {
            let bad = r.differing > maxPercent || !r.misses.isEmpty
            if bad { failed += 1 }
            missTotal += r.misses.count
            var miss = "\(r.misses.count)"
            if !r.misses.isEmpty { miss += " (" + r.misses.prefix(3).map { $0.replacingOccurrences(of: "|", with: "/") }.joined(separator: "; ") + ")" }
            out += String(format: "| %@ | %.3f%% | %@ | %@%@ |\n", r.name, r.differing, miss, bad ? "fail" : "pass", r.note.isEmpty ? "" : " (\(r.note))")
        }
        let passed = results.count - failed
        out += String(format: "\n%d of %d pages pass (a page fails above %.2f%% differing pixels, channel difference over %d, or with any HTML miss); %d HTML misses in all.\n",
                      passed, results.count, maxPercent, channel, missTotal)
        print(out)
        try? out.write(to: dir.appendingPathComponent("check-results.md"), atomically: true, encoding: .utf8)
        exit(failed == 0 ? 0 : 1)
    }
}

let args = CommandLine.arguments
guard args.count >= 2 else { fputs("usage: check-pages.swift <MacHTML mode folder> [percent] [channel]\n", stderr); exit(2) }
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let checker = Checker(dir: URL(fileURLWithPath: args[1]).standardizedFileURL,
                      maxPercent: args.count > 2 ? Double(args[2]) ?? 0.5 : 0.5,
                      channel: args.count > 3 ? Int(args[3]) ?? 24 : 24)
DispatchQueue.main.async { checker.start() }
app.run()
