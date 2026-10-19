// The first-launch welcome page, shown in the window before Application Home. It asks once, in
// plain words, for the name that goes on notes, tasks and activity.

import OpenFrameCore
import SwiftUI

struct WelcomeNameView: View {
    @Environment(\.ofTheme) private var theme
    @State var model: WelcomeNameModel
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: PageSpacing.medium) {
            VStack(spacing: PageSpacing.tight) {
                Text("Welcome to OpenFrame Studio").font(.title2.weight(.semibold))
                    .multilineTextAlignment(.center)
                    .accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier("welcome-title")
                Text("What should we call you?").font(.callout).foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity)
            VStack(alignment: .leading, spacing: 4) {
                Text("Your name (required)").font(.callout.weight(.medium))
                TextField("Your name", text: $model.name)
                    .textFieldStyle(.roundedBorder)
                    .focused($focused)
                    .onSubmit { Task { await model.continueTapped() } }
                    .onChange(of: model.name) { _, _ in model.edited() }
                    .accessibilityLabel("Your name")
                    .accessibilityIdentifier("welcome-name")
                if model.showsEmptyError {
                    Text("Enter your name.").font(.caption).foregroundStyle(theme["red-t"])
                        .accessibilityIdentifier("welcome-name-error")
                } else if let error = model.error {
                    Text(error).font(.caption).foregroundStyle(theme["red-t"])
                        .accessibilityIdentifier("welcome-error")
                }
            }
            Text("Your name appears on notes, tasks and project activity. There is no account. It stays on this computer and you can change it any time in Settings.")
                .font(.caption).foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: .infinity)
                .fixedSize(horizontal: false, vertical: true)
            HStack {
                Spacer()
                Button("Continue") { Task { await model.continueTapped() } }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .disabled(model.invalid || model.busy)
                    .accessibilityIdentifier("welcome-continue")
                Spacer()
            }
        }
        .padding(PageSpacing.large)
        .frame(width: 420)
        .onAppear { focused = true }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("welcome-sheet")
    }
}
