// Beat Board layout (Windows commit 0f30921e, ui.ts, BoardView.tsx and dnd.tsx): acts and the
// Parking Lot as side-by-side columns (Vertical, the default) or as full-width rows with cards
// left to right (Horizontal). A view preference kept per person on this Mac
// (`of.story.boardLayout`), never project data. Data and order are the same in both layouts;
// the arrow keys, the drop positions and the act menu follow the layout.

import CoreGraphics
import Foundation

enum StoryBoardLayout: String, CaseIterable, Identifiable {
    case vertical, horizontal

    static let key = "of.story.boardLayout"

    var id: String { rawValue }
    var title: String { self == .vertical ? "Vertical" : "Horizontal" }
    var symbol: String { self == .vertical ? "rectangle.split.3x1" : "rectangle.split.1x2" }

    /// Any stored value other than "horizontal" (old or hand-edited) falls back to the columns.
    init(stored: String?) { self = stored == "horizontal" ? .horizontal : .vertical }

    static func load(_ defaults: UserDefaults = .standard) -> StoryBoardLayout {
        StoryBoardLayout(stored: defaults.string(forKey: key))
    }

    func save(_ defaults: UserDefaults = .standard) { defaults.set(rawValue, forKey: Self.key) }

    /// In rows the reading order runs left to right, so Left and Right step through an act and Up
    /// and Down jump between acts: the arrow keys are turned a quarter (Windows ROW_KEYS).
    func arrow(_ d: StoryNavigation) -> StoryNavigation {
        guard self == .horizontal else { return d }
        switch d {
        case .left: return .up
        case .right: return .down
        case .up: return .left
        case .down: return .right
        }
    }

    /// Acts are compared along x in columns and along y in rows.
    var actsAlongY: Bool { self == .horizontal }
    /// Cards stack along y in columns and flow along x in rows.
    var itemsAlongY: Bool { self == .vertical }

    /// The act menu names the move in the direction the acts are laid out.
    var moveEarlier: String { self == .horizontal ? "Move Up" : "Move Left" }
    var moveLater: String { self == .horizontal ? "Move Down" : "Move Right" }

    /// True when `location` is in the first half of a zone of `size` along the axis: the top half
    /// when `alongY`, else the left half (Windows inFirstHalf).
    static func inFirstHalf(alongY: Bool, location: CGPoint, size: CGSize) -> Bool {
        alongY ? location.y < size.height / 2 : location.x < size.width / 2
    }
}
