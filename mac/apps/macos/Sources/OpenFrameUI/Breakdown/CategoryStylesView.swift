// Breakdown category colours and icons (depth spec section 23; Windows: Settings, Advanced).
// Normal people never need this page; the defaults are chosen for contrast and clarity. The label
// and tooltip of a category never change, so colour is never the only signal. The core stores the
// choice in the project and sends the colour as a hex value.

import SwiftUI
import OpenFrameCore

struct CategoryStylesView: View {
    @Bindable var model: CategoryStylesModel
    @Environment(AppModel.self) private var app
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        BreakdownCategorySettingsView(model: model)
            .task(id: app.changes.key(BDTables.categoryStyle)) { await model.reload() }
    }
}

/// The category look editor. It stands alone so the Settings window can show the same page.
struct BreakdownCategorySettingsView: View {
    @Bindable var model: CategoryStylesModel
    @Environment(BreakdownHub.self) private var hub
    @Environment(\.ofTheme) private var theme

    var body: some View {
        ScrollView {
            VStack(spacing: PageSpacing.medium) {
                BDPageHeader(title: "Categories", sub: "Breakdown category colours and icons. Every category also keeps its name and description, so colour is never the only signal.") {
                    if model.canEdit {
                        Button { Task { await model.reset() } } label: { Label("Restore Defaults", systemImage: "arrow.counterclockwise") }
                            .disabled(!model.anyCustomized)
                            .accessibilityIdentifier("restore-category-defaults")
                    }
                }
                if let styles = model.styles {
                    VStack(spacing: 0) {
                        ForEach(styles.categories, id: \.category) { c in
                            row(c, styles)
                            Divider()
                        }
                    }
                    .background(theme["panel"], in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(theme["line"]))
                } else {
                    ProgressView().frame(maxWidth: .infinity, minHeight: 120)
                }
            }
            .pageColumn(maxWidth: 900)
            .padding(.bottom, PageSpacing.large)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("categories-tab")
    }

    private func row(_ c: BreakdownCategoryStyleDto, _ styles: BreakdownCategoryStyles) -> some View {
        HStack(alignment: .top, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                CategoryMark(look: hub.look(c.category)).font(.callout.weight(.semibold))
                Text(c.tooltip).font(.caption).foregroundStyle(theme["muted"]).fixedSize(horizontal: false, vertical: true)
                if c.customized { BDChip(text: "Changed", tone: .accent) }
            }
            .frame(width: 210, alignment: .leading)
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 6) {
                    ForEach(styles.palette, id: \.token) { p in
                        let on = c.color == p.token
                        Button {
                            if !on { Task { await model.setColor(c.category, token: p.token) } }
                        } label: {
                            Circle().fill(BDColor.color(hex: p.hex) ?? theme["muted"]).frame(width: 22, height: 22)
                                .overlay(Circle().strokeBorder(on ? theme["ink"] : theme["line2"], lineWidth: on ? 2.5 : 1))
                        }
                        .buttonStyle(.plain)
                        .disabled(!model.canEdit)
                        .help(p.label)
                        .accessibilityLabel(p.label)
                        .accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
                        .accessibilityValue(on ? "Selected" : "")
                        .accessibilityIdentifier("color-\(c.category.rawValue)-\(p.token)")
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("\(c.label) colour")
                HStack(spacing: 4) {
                    ForEach(styles.icons, id: \.self) { key in
                        let on = c.icon == key
                        Button {
                            if !on { Task { await model.setIcon(c.category, key: key) } }
                        } label: {
                            Image(systemName: CategoryLook.symbol(forKey: key))
                                .frame(width: 26, height: 24)
                                .foregroundStyle(on ? (BDColor.color(hex: c.colorHex) ?? theme["ink"]) : theme["ink2"])
                                .background(on ? theme["accent-soft"] : Color.clear, in: RoundedRectangle(cornerRadius: 5, style: .continuous))
                        }
                        .buttonStyle(.plain)
                        .disabled(!model.canEdit)
                        .help(key.replacingOccurrences(of: "-", with: " "))
                        .accessibilityLabel("Icon \(key.replacingOccurrences(of: "-", with: " "))")
                        .accessibilityAddTraits(on ? [.isButton, .isSelected] : .isButton)
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel("\(c.label) icon")
            }
        }
        .padding(14)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("\(c.label) look")
        .accessibilityIdentifier("category-style-\(c.category.rawValue)")
    }
}
