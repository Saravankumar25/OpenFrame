// Which screenplay command a key press runs, read from the shortcuts now in force
// (ShortcutStore: the defaults, or what the person changed in Keyboard Shortcuts).

import OpenFrameCore

enum ScreenplayKeyCommand: Equatable {
    case element(ElementType)
    case pageBreak
    case find

    private static let elementIds: [(String, ElementType)] = [
        ("screenplay.element_scene_heading", .sceneHeading),
        ("screenplay.element_action", .action),
        ("screenplay.element_character", .character),
        ("screenplay.element_parenthetical", .parenthetical),
        ("screenplay.element_dialogue", .dialogue),
        ("screenplay.element_transition", .transition),
        ("screenplay.element_shot", .shot),
        ("screenplay.element_note", .note),
    ]

    /// The command that owns this chord, or nil when no screenplay command uses it.
    @MainActor
    static func command(forChord chord: String, store: ShortcutStore = .shared) -> ScreenplayKeyCommand? {
        if store.chords("screenplay.page_break").contains(chord) { return .pageBreak }
        if store.chords("screenplay.find").contains(chord) { return .find }
        for (id, kind) in elementIds where store.chords(id).contains(chord) { return .element(kind) }
        return nil
    }
}
