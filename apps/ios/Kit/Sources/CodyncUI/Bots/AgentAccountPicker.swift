import CodyncKit
import SwiftUI

/// Selects host-owned agent accounts; this view only keeps its presentation state.
struct AgentAccountPicker: View {
    let backend: String
    @Binding var selection: String?
    @Environment(BotStore.self) private var store
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var accounts: [AgentAccount] = []
    @State private var loading = false
    @State private var signingIn = false
    @State private var failure: String?
    @State private var retry = 0
    @State private var signInTask: Task<Void, Never>?

    private var scope: String {
        "\(ObjectIdentifier(store))/\(store.storage.id)/\(store.computer.id)/\(backend)"
    }

    private var requestKey: String { "\(scope)/\(store.connection == .online)/\(retry)" }

    private var choices: [(String, String)] {
        var result = [("", "Default (computer)")] + accounts.map { ($0.id, $0.name) }
        if let selection, !result.contains(where: { $0.0 == selection }) {
            result.append((selection, "Saved account (unavailable)"))
        }
        return result
    }

    private var value: Binding<String> {
        Binding(get: { selection ?? "" }, set: { newValue in
            withAnimation(Motion.reduced(Motion.fade, reduceMotion)) {
                selection = newValue.isEmpty ? nil : newValue
            }
        })
    }

    var body: some View {
        if backend == "antigravity" {
            VStack(alignment: .leading, spacing: 8) {
                ValueRow("Google account", detail: selection == nil
                         ? "Uses this computer's default Antigravity account."
                         : "Uses a dedicated account. Changing it starts a new agent session; chat history is kept.") {
                    ChoicePicker(selection: value, options: choices, fitsAvailableWidth: true)
                        .disabled(signingIn)
                }
                HStack(spacing: 8) {
                    Button("Sign in with another account") { startSignIn() }
                        .buttonStyle(SecondaryButtonStyle())
                        .disabled(loading || signingIn || store.connection != .online)
                    IconButton("Refresh Google accounts", systemImage: "arrow.clockwise") { retry += 1 }
                        .disabled(loading || signingIn)
                    if loading || signingIn {
                        Spinner(size: 12)
                            .accessibilityLabel(signingIn ? "Waiting for sign-in on the computer" : "Loading Google accounts")
                    }
                }
                if signingIn {
                    Text("Finish Google sign-in in the paired computer's browser. Keep this screen open while it completes.")
                        .font(.caption).foregroundStyle(Palette.secondary)
                }
                if let failure {
                    Text(failure).font(.caption).foregroundStyle(Palette.warning)
                    Button("Retry loading accounts") { retry += 1 }
                        .buttonStyle(SecondaryButtonStyle())
                        .disabled(loading || signingIn)
                } else if !loading && !signingIn && accounts.isEmpty {
                    Text("No dedicated accounts on this computer. Use Default or sign in with another account.")
                        .font(.caption).foregroundStyle(Palette.secondary)
                }
            }
            .animation(Motion.reduced(Motion.fade, reduceMotion), value: loading)
            .animation(Motion.reduced(Motion.fade, reduceMotion), value: signingIn)
            .animation(Motion.reduced(Motion.fade, reduceMotion), value: failure)
            .task(id: requestKey) { await load() }
            .onChange(of: scope) { _, _ in
                signInTask?.cancel()
                signingIn = false
                accounts = []
            }
            .onDisappear {
                signInTask?.cancel()
                signingIn = false
            }
        }
    }

    @MainActor private func load() async {
        let requestedScope = scope
        loading = true
        failure = nil
        do {
            let response = try await store.loadAgentAccounts(backend: backend)
            try Task.checkCancellation()
            guard requestedScope == scope else { return }
            accounts = response
        } catch {
            guard !Task.isCancelled, requestedScope == scope else { return }
            failure = "Couldn't load Google accounts: \(error.localizedDescription)"
        }
        guard requestedScope == scope else { return }
        loading = false
    }

    @MainActor private func startSignIn() {
        guard !signingIn else { return }
        let requestedScope = scope
        signingIn = true
        failure = nil
        signInTask = Task {
            do {
                let account = try await store.authenticateAgentAccount(backend: backend)
                try Task.checkCancellation()
                guard requestedScope == scope else { return }
                withAnimation(Motion.reduced(Motion.fade, reduceMotion)) {
                    accounts.removeAll { $0.id == account.id }
                    accounts.append(account)
                    selection = account.id
                }
            } catch {
                guard !Task.isCancelled, requestedScope == scope else { return }
                failure = "Couldn't sign in: \(error.localizedDescription)"
            }
            guard requestedScope == scope else { return }
            signingIn = false
        }
    }
}
