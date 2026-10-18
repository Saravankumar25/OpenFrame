// Mac phase M4 editor spike (ADR-0027 open item 1): the same 120-page script in the native text
// engine (TextKit 2) and in a web view, measured on this Mac. The numbers go to
// docs/release/evidence/mac/m4-editor-spike.md. Typing latency here is the engine's own cost for
// one keystroke in the middle of the script plus the layout of what is on screen; the screen
// refresh adds the same amount to both.

import AppKit
import Foundation
import Testing
import WebKit
@testable import OpenFrameCore
@testable import OpenFrameUI

enum SpikeScript {
    /// About 120 pages: 110 scenes of heading, action, and dialogue exchanges.
    static func paragraphs() -> [EditorParagraph] {
        var out: [EditorParagraph] = []
        let names = ["ANIKA", "RAVI", "MEERA", "JOHN", "LI WEI"]
        for s in 0..<110 {
            out.append(EditorParagraph(kind: .sceneHeading, text: "INT. RAILWAY STATION PLATFORM \(s) - NIGHT", elementId: "h\(s)", sceneId: "s\(s)"))
            out.append(EditorParagraph(kind: .action, text: "Rain hammers the tin roof. A train idles at the far end, its windows fogged. People wait under the clock, coats pulled tight against the cold, watching the empty track.", elementId: "a\(s)"))
            for d in 0..<8 {
                out.append(EditorParagraph(kind: .character, text: names[(s + d) % names.count], elementId: "c\(s)-\(d)"))
                if d % 3 == 0 { out.append(EditorParagraph(kind: .parenthetical, text: "(quietly)", elementId: "p\(s)-\(d)")) }
                out.append(EditorParagraph(kind: .dialogue, text: "We should have left an hour ago. You know the last train never waits, not for anyone, and certainly not for us.", elementId: "d\(s)-\(d)"))
            }
            out.append(EditorParagraph(kind: .transition, text: "CUT TO:", elementId: "t\(s)"))
        }
        return out
    }

    static func percentile(_ values: [Double], _ p: Double) -> Double {
        let sorted = values.sorted()
        return sorted[min(sorted.count - 1, Int(Double(sorted.count - 1) * p))]
    }
}

@Suite("Editor spike", .serialized)
@MainActor
struct EditorSpikeTests {
    @Test func nativeTextEngineTypingLatency() throws {
        _ = NSApplication.shared
        let view = ScreenplayTextView.make()
        let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 900, height: 800))
        scroll.documentView = view
        view.frame = NSRect(x: 0, y: 0, width: 900, height: 800)
        let window = NSWindow(contentRect: scroll.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = scroll
        let script = SpikeScript.paragraphs()

        let loadStart = CFAbsoluteTimeGetCurrent()
        view.load(script)
        view.textLayoutManager?.ensureLayout(for: view.textLayoutManager!.documentRange)
        let loadMs = (CFAbsoluteTimeGetCurrent() - loadStart) * 1000

        // Caret in the middle of the script.
        view.reveal(paragraph: script.count / 2)
        var samples: [Double] = []
        for i in 0..<300 {
            let t0 = CFAbsoluteTimeGetCurrent()
            view.insertText(i % 9 == 8 ? " " : "a", replacementRange: view.selectedRange())
            if let tlm = view.textLayoutManager, let viewport = tlm.textViewportLayoutController.viewportRange {
                tlm.ensureLayout(for: viewport)
            } else {
                view.layoutSubtreeIfNeeded()
            }
            samples.append((CFAbsoluteTimeGetCurrent() - t0) * 1000)
        }
        let p50 = SpikeScript.percentile(samples, 0.5)
        let p95 = SpikeScript.percentile(samples, 0.95)
        let maxMs = samples.max() ?? 0
        // Pages: 55 lines a page, as the Windows pagination counts.
        let report = String(format: "native paragraphs=%d load_ms=%.1f type_p50_ms=%.3f type_p95_ms=%.3f type_max_ms=%.3f", script.count, loadMs, p50, p95, maxMs)
        print("SPIKE " + report)
        try? report.write(to: Fixture.snapshots.appendingPathComponent("spike-native.txt"), atomically: true, encoding: .utf8)
        // The Windows typing budget is 16 ms a keystroke (typingBudget.test.ts).
        #expect(p95 < 16)
        // Every paragraph keeps its element after the edits.
        #expect(view.paragraphs().count == script.count)
    }

    @Test func webViewTypingLatency() async throws {
        _ = NSApplication.shared
        let web = WKWebView(frame: NSRect(x: 0, y: 0, width: 900, height: 800))
        let window = NSWindow(contentRect: web.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = web
        let body = SpikeScript.paragraphs().map { p in
            "<p class=\"\(p.kind.rawValue)\">\(p.text.replacingOccurrences(of: "<", with: "&lt;"))</p>"
        }.joined()
        let html = """
        <!doctype html><meta charset=utf-8><style>
        body{font:12pt Courier;width:6in;margin:0 auto} p{margin:12pt 0 0} .character{margin-left:2.2in;text-transform:uppercase}
        .dialogue{margin:0 1.5in 0 1in} .parenthetical{margin:0 2in 0 1.6in} .scene_heading{text-transform:uppercase;font-weight:bold}
        .transition{text-align:right}</style><div id=e contenteditable=true>\(body)</div>
        """
        let loaded = AsyncStream<Void> { continuation in
            let delegate = LoadDelegate { continuation.yield(); continuation.finish() }
            web.navigationDelegate = delegate
            objc_setAssociatedObject(web, "d", delegate, .OBJC_ASSOCIATION_RETAIN)
            web.loadHTMLString(html, baseURL: nil)
        }
        let loadStart = CFAbsoluteTimeGetCurrent()
        for await _ in loaded {}
        let loadMs = (CFAbsoluteTimeGetCurrent() - loadStart) * 1000
        let js = """
        (() => { const e=document.getElementById('e'); e.focus();
          const ps=e.querySelectorAll('p'); const mid=ps[Math.floor(ps.length/2)];
          const r=document.createRange(); r.setStart(mid.firstChild, 5); r.collapse(true);
          const s=getSelection(); s.removeAllRanges(); s.addRange(r);
          const out=[]; for (let i=0;i<300;i++){ const t0=performance.now();
            document.execCommand('insertText', false, i%9==8?' ':'a'); void e.offsetHeight; out.push(performance.now()-t0); }
          out.sort((a,b)=>a-b); return JSON.stringify({p50: out[150], p95: out[284], max: out[299], n: ps.length}); })()
        """
        let result = try await web.evaluateJavaScript(js) as? String ?? "{}"
        let report = "web load_ms=\(String(format: "%.1f", loadMs)) \(result)"
        print("SPIKE " + report)
        try? report.write(to: Fixture.snapshots.appendingPathComponent("spike-web.txt"), atomically: true, encoding: .utf8)
        #expect(result.contains("p95"))
    }
}

final class LoadDelegate: NSObject, WKNavigationDelegate {
    let done: () -> Void
    init(done: @escaping () -> Void) { self.done = done }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) { done() }
}
