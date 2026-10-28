// The corner chip and "Buy full version" button in the top bar (Windows: TrialCorner.tsx, owner
// mocks 183 and 188). Shown on every screen from trial day 1 until the purchase; amber in the last
// 3 days, always with the days in words; gone after the purchase. No price is ever shown. The
// internal disk image build shows nothing.
//
// After "Not now" on the "Checking your purchase" screen the bar shows "Check purchase" to open it
// again, so the way to check is never lost for the session.

import SwiftUI
import OpenFrameCore

struct TrialCorner: View {
    let licence: LicenceModel
    @Environment(\.ofTheme) private var theme

    var body: some View {
        if licence.showsCheckPurchase {
            Button { licence.reopenCheck() } label: { Label("Check purchase", systemImage: "arrow.clockwise") }
                .buttonStyle(.ofProminent).controlSize(.small)
                .accessibilityIdentifier("check-purchase")
        } else if let chip = licence.chip {
            HStack(spacing: 6) {
                Text(chip.label)
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 8).padding(.vertical, 3)
                    .background((chip.ending ? theme["yellow-t"] : Color.secondary).opacity(0.18), in: Capsule())
                    .foregroundStyle(chip.ending ? theme["yellow-t"] : Color.primary)
                    .help(chip.spoken)
                    .accessibilityElement()
                    .accessibilityLabel(chip.spoken)
                    .accessibilityIdentifier("trial-chip")
                Button { Task { await licence.buy() } } label: { Label("Buy full version", systemImage: "bolt.fill") }
                    .buttonStyle(.ofProminent).controlSize(.small).disabled(licence.busy)
                    .accessibilityIdentifier("trial-buy")
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Free trial")
        }
    }
}

struct TrialCornerToolbar: ViewModifier {
    let licence: LicenceModel
    func body(content: Content) -> some View {
        content.toolbar {
            // Nothing is added in the internal build, so its toolbar stays exactly as it is.
            if licence.isStore {
                ToolbarItem(placement: .primaryAction) { TrialCorner(licence: licence) }
            }
        }
    }
}

extension View {
    func trialCorner(_ licence: LicenceModel) -> some View { modifier(TrialCornerToolbar(licence: licence)) }
}
