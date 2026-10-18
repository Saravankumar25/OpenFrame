// The internal build's team password screen (ADR-0024, ADR-0030). The core checks the password
// and the waits after wrong tries; this screen only shows what the core says.

import SwiftUI
import OpenFrameCore

struct TeamGateView: View {
    let status: TeamGateStatus
    @Environment(AppModel.self) private var model
    @Environment(\.ofTheme) private var theme
    @State private var password = ""
    @State private var message: String?
    @State private var checking = false
    @FocusState private var focused: Bool

    private var waiting: Bool { status.waitSeconds > 0 }

    var body: some View {
        VStack(spacing: 20) {
            BrandMark(size: 56)
            VStack(spacing: 6) {
                Text("OpenFrame Studio")
                    .font(.system(size: 26, weight: .semibold))
                    .tracking(-0.4)
                Text("This is an internal build for the team. Enter the team password to continue.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 340)
            }
            SecureField("Team password", text: $password)
                .textFieldStyle(.roundedBorder)
                .frame(width: 280)
                .focused($focused)
                .disabled(checking || waiting)
                .onSubmit(submit)
                .accessibilityIdentifier("team-password")
            if let message {
                Text(message)
                    .font(.callout)
                    .foregroundStyle(theme["red-t"])
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 340)
                    .accessibilityIdentifier("team-gate-message")
            } else if waiting {
                Text("Too many wrong tries. Try again in \(Int(status.waitSeconds)) seconds.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            Button(action: submit) {
                if checking { ProgressView().controlSize(.small) } else { Text("Continue") }
            }
            .keyboardShortcut(.defaultAction)
            .controlSize(.large)
            .disabled(password.isEmpty || checking || waiting)
            .accessibilityIdentifier("team-gate-continue")
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(theme["bg"])
        .onAppear { focused = true }
        .task(id: status.waitSeconds) {
            guard waiting else { return }
            try? await Task.sleep(nanoseconds: UInt64(status.waitSeconds) * 1_000_000_000)
            await model.refreshGate()
        }
    }

    private func submit() {
        guard !password.isEmpty, !checking, !waiting else { return }
        checking = true
        let typed = password
        Task {
            let answer = await model.unlock(password: typed)
            checking = false
            if let answer {
                message = answer
                password = ""
                focused = true
            }
        }
    }
}

/// The OpenFrame mark in the theme's logo colours (the logo follows the theme).
struct BrandMark: View {
    var size: CGFloat = 32
    @Environment(\.ofTheme) private var theme

    var body: some View {
        RoundedRectangle(cornerRadius: size * 0.24, style: .continuous)
            .fill(theme.logoGradient)
            .frame(width: size, height: size)
            .overlay {
                Image(systemName: "film")
                    .font(.system(size: size * 0.46, weight: .semibold))
                    .foregroundStyle(.white)
            }
            .accessibilityHidden(true)
    }
}
