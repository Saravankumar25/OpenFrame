// "Draw / Sketch panel": basic sketching only. A pen, an eraser, three inks, undo and clear on a
// white 960 by 540 sheet, drawn by an AppKit view (no PencilKit). The picture is saved as a PNG
// through the core (storyboard.add_panel and storyboard.set_panel_visual with `dataBase64`), the
// same bytes the Windows sketch dialog writes: strokes painted with round caps on white paper,
// the pen 4 wide and the eraser 28 wide painting paper. Mirrors
// apps/desktop/src/workspaces/production/visual/SketchDialog.tsx.

import AppKit
import Observation
import SwiftUI

enum SketchInk: String, CaseIterable, Identifiable {
    case black, red, blue
    var id: String { rawValue }
    var label: String { rawValue.capitalized }
    /// The same document colours as Windows.
    var hex: String {
        switch self {
        case .black: "#111111"
        case .red: "#c8443a"
        case .blue: "#3b6fd4"
        }
    }
    var cgColor: CGColor {
        let v = UInt32(hex.dropFirst(), radix: 16) ?? 0
        return CGColor(red: CGFloat((v >> 16) & 0xff) / 255, green: CGFloat((v >> 8) & 0xff) / 255, blue: CGFloat(v & 0xff) / 255, alpha: 1)
    }
}

enum SketchTool: String, CaseIterable, Identifiable {
    case pen, eraser
    var id: String { rawValue }
    var label: String { self == .pen ? "Pen" : "Eraser" }
    var width: Double { self == .pen ? 4 : 28 }
}

struct SketchStroke: Equatable {
    var ink: SketchInk
    var width: Double
    var erase: Bool
    var points: [CGPoint]
}

/// The sheet and what is drawn on it.
struct SketchSheetSize {
    static let width = 960
    static let height = 540
}

enum SketchPainter {
    /// Paint strokes onto a context whose origin is the top left corner (white paper; the eraser paints paper).
    static func paint(_ ctx: CGContext, strokes: [SketchStroke]) {
        ctx.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
        ctx.fill(CGRect(x: 0, y: 0, width: SketchSheetSize.width, height: SketchSheetSize.height))
        ctx.setLineCap(.round)
        ctx.setLineJoin(.round)
        for s in strokes {
            ctx.setStrokeColor(s.erase ? CGColor(red: 1, green: 1, blue: 1, alpha: 1) : s.ink.cgColor)
            ctx.setLineWidth(s.width)
            ctx.beginPath()
            for (i, p) in s.points.enumerated() {
                if i == 0 { ctx.move(to: p) } else { ctx.addLine(to: p) }
            }
            if s.points.count == 1, let p = s.points.first { ctx.addLine(to: CGPoint(x: p.x + 0.1, y: p.y)) }
            ctx.strokePath()
        }
    }

    /// The sheet as a 960 by 540 PNG.
    static func png(strokes: [SketchStroke]) -> Data? {
        guard let rep = NSBitmapImageRep(
            bitmapDataPlanes: nil, pixelsWide: SketchSheetSize.width, pixelsHigh: SketchSheetSize.height,
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
            let gc = NSGraphicsContext(bitmapImageRep: rep)
        else { return nil }
        let ctx = gc.cgContext
        // Strokes use a top left origin; the bitmap has a bottom left one.
        ctx.translateBy(x: 0, y: CGFloat(SketchSheetSize.height))
        ctx.scaleBy(x: 1, y: -1)
        paint(ctx, strokes: strokes)
        return rep.representation(using: .png, properties: [:])
    }
}

/// The state of one sketch dialog. Strokes are only kept here until Save Panel.
@MainActor
@Observable
final class SketchModel {
    private(set) var strokes: [SketchStroke] = []
    private(set) var live: SketchStroke?
    var tool: SketchTool = .pen
    var ink: SketchInk = .black
    /// Bumped on every change so the drawing view repaints.
    private(set) var revision = 0

    var canUndo: Bool { !strokes.isEmpty }
    var canSave: Bool { !strokes.isEmpty }
    var allStrokes: [SketchStroke] { live.map { strokes + [$0] } ?? strokes }

    func chooseInk(_ i: SketchInk) {
        ink = i
        tool = .pen
    }

    func begin(at p: CGPoint) {
        live = SketchStroke(ink: ink, width: tool.width, erase: tool == .eraser, points: [clamped(p)])
        revision += 1
    }

    func extend(to p: CGPoint) {
        guard live != nil else { return }
        live?.points.append(clamped(p))
        revision += 1
    }

    func end() {
        guard let s = live else { return }
        live = nil
        strokes.append(s)
        revision += 1
    }

    /// Takes the last stroke back.
    func undo() {
        guard !strokes.isEmpty else { return }
        strokes.removeLast()
        revision += 1
    }

    func clear() {
        strokes = []
        live = nil
        revision += 1
    }

    func png() -> Data? { SketchPainter.png(strokes: strokes) }

    /// Keeps the points on the sheet, as the browser canvas does when the pointer leaves it.
    private func clamped(_ p: CGPoint) -> CGPoint {
        CGPoint(x: min(max(p.x, 0), CGFloat(SketchSheetSize.width)), y: min(max(p.y, 0), CGFloat(SketchSheetSize.height)))
    }
}

// MARK: Drawing view

final class SketchDrawingNSView: NSView {
    weak var model: SketchModel?

    override var isFlipped: Bool { true }
    override var acceptsFirstResponder: Bool { true }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func resetCursorRects() {
        addCursorRect(bounds, cursor: model?.tool == .eraser ? .crosshair : .crosshair)
    }

    private func sheetPoint(_ event: NSEvent) -> CGPoint {
        let p = convert(event.locationInWindow, from: nil)
        guard bounds.width > 0, bounds.height > 0 else { return .zero }
        return CGPoint(x: p.x / bounds.width * CGFloat(SketchSheetSize.width), y: p.y / bounds.height * CGFloat(SketchSheetSize.height))
    }

    override func mouseDown(with event: NSEvent) {
        MainActor.assumeIsolated {
            window?.makeFirstResponder(self)
            model?.begin(at: sheetPoint(event))
        }
    }

    override func mouseDragged(with event: NSEvent) {
        MainActor.assumeIsolated { model?.extend(to: sheetPoint(event)) }
    }

    override func mouseUp(with event: NSEvent) {
        MainActor.assumeIsolated { model?.end() }
    }

    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext, let model else { return }
        let strokes = MainActor.assumeIsolated { model.allStrokes }
        ctx.saveGState()
        ctx.scaleBy(x: bounds.width / CGFloat(SketchSheetSize.width), y: bounds.height / CGFloat(SketchSheetSize.height))
        SketchPainter.paint(ctx, strokes: strokes)
        ctx.restoreGState()
    }

    override func accessibilityRole() -> NSAccessibility.Role? { .image }
    override func accessibilityLabel() -> String? { "Sketch area" }
    override func isAccessibilityElement() -> Bool { true }
}

struct SketchCanvas: NSViewRepresentable {
    let model: SketchModel

    func makeNSView(context: Context) -> SketchDrawingNSView {
        let v = SketchDrawingNSView()
        v.model = model
        v.setAccessibilityIdentifier("sketch-area")
        return v
    }

    func updateNSView(_ view: SketchDrawingNSView, context: Context) {
        view.model = model
        _ = model.revision
        view.needsDisplay = true
    }
}

// MARK: Dialog

struct SketchSheet: View {
    var title = "Draw / Sketch panel"
    var saving = false
    var onSave: (Data) -> Void
    var onCancel: () -> Void

    @State private var model = SketchModel()
    @Environment(\.ofTheme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.title3.weight(.semibold))
            HStack(spacing: 12) {
                Picker("Drawing tool", selection: $model.tool) {
                    ForEach(SketchTool.allCases) { Text($0.label).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .fixedSize()
                .accessibilityLabel("Drawing tool")
                .accessibilityIdentifier("sketch-tool")
                HStack(spacing: 6) {
                    ForEach(SketchInk.allCases) { ink in
                        Button { model.chooseInk(ink) } label: {
                            Circle().fill(Color(cgColor: ink.cgColor)).frame(width: 18, height: 18)
                                .overlay(Circle().strokeBorder(theme["accent"], lineWidth: model.ink == ink && model.tool == .pen ? 2 : 0).padding(-3))
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("\(ink.label) ink")
                        .accessibilityAddTraits(model.ink == ink ? .isSelected : [])
                        .accessibilityIdentifier("sketch-ink-\(ink.rawValue)")
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("Ink colour")
                Button("Undo") { model.undo() }
                    .disabled(!model.canUndo)
                    .keyboardShortcut("z", modifiers: .command)
                    .accessibilityIdentifier("sketch-undo")
                Button("Clear") { model.clear() }
                    .disabled(!model.canUndo)
                    .accessibilityIdentifier("sketch-clear")
                Spacer()
            }
            SketchCanvas(model: model)
                .aspectRatio(16.0 / 9.0, contentMode: .fit)
                .frame(minWidth: 640, minHeight: 360)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(theme["line2"]))
            Text("Basic sketching only. For polished art, import an image made elsewhere.")
                .font(.caption).foregroundStyle(theme["muted"])
            HStack {
                Spacer()
                Button("Cancel", role: .cancel, action: onCancel)
                    .keyboardShortcut(.cancelAction)
                    .accessibilityIdentifier("sketch-cancel")
                Button("Save Panel") {
                    if let png = model.png() { onSave(png) }
                }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.defaultAction)
                .disabled(!model.canSave || saving)
                .accessibilityIdentifier("sketch-save")
            }
        }
        .padding(20)
        .accessibilityIdentifier("sketch-sheet")
    }
}
