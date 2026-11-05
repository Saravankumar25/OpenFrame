// Toolbar items sit straight on the window's background. macOS 26 draws a glass capsule behind each
// item or group of items by default, which the owner does not want (2026-11-04).

import SwiftUI

extension ToolbarContent {
    /// This item without the shared capsule macOS 26 puts behind it. Earlier systems draw none.
    @ToolbarContentBuilder func plainToolbarBackground() -> some ToolbarContent {
        if #available(macOS 26.0, *) {
            sharedBackgroundVisibility(.hidden)
        } else {
            self
        }
    }
}
