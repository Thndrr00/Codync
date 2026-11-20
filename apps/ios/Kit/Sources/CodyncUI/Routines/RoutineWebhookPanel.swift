import CodyncKit
import SwiftUI

/// A saved webhook routine's address and key: copy either, reveal or replace the key.
struct RoutineWebhookPanel: View {
    let botId: String
    let routineId: String
    @Environment(BotStore.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var webhook: RoutineWebhook?
    @State private var error: String?
    @State private var showKey = false
    @State private var copied: String?
    @State private var confirmRotate = false
    @State private var rotating = false
    @State private var confirmGive = false
    @State private var giving = false
    @State private var givenName: String?

    private var agentName: String { model.bots[botId]?.name ?? botId }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if let webhook {
                Field(webhook.url == nil ? "Local URL" : "URL") {
                    value(webhook.url ?? webhook.localUrl, copy: "url")
                }
                Field("Key") {
                    value(showKey ? webhook.key : String(repeating: "•", count: 16), copy: "key") {
                        IconButton(showKey ? "Hide key" : "Show key", systemImage: showKey ? "eye.slash" : "eye") {
                            withAnimation(Motion.reduced(Motion.hover, reduceMotion)) { showKey.toggle() }
                        }
                        IconButton("Replace key", systemImage: "arrow.triangle.2.circlepath") { confirmRotate = true }
                            .disabled(rotating || giving || model.isOffline)
                        IconButton("Give the signing secret to \(agentName)", systemImage: "key") { confirmGive = true }
                            .disabled(giving || rotating || model.isOffline)
                    }
                }
                if let givenName {
                    Text("Saved as \(givenName)").font(.caption).foregroundStyle(Palette.secondary).padding(.leading, 4)
                }
                Text(note(webhook))
                    .font(.caption).foregroundStyle(Palette.secondary).padding(.leading, 4)
                    .fixedSize(horizontal: false, vertical: true)
            } else if let error {
                Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger).padding(.leading, 4)
            } else {
                Text(L10n.tr("Loading webhook…")).font(.caption).foregroundStyle(Palette.secondary).padding(.leading, 4)
            }
        }
        .task(id: routineId) { await load(rotate: false) }
        .codyncDialog("Replace the key?", isPresented: $confirmRotate,
                      message: "Senders using the current key stop working until they get the new one.") {
            [DialogAction("Replace key", destructive: true) { Task { await load(rotate: true) } }]
        }
        .codyncDialog("Give the signing secret to \(agentName)?", isPresented: $confirmGive,
                      message: "The old secret stops working. The agent gets the new one in its own vault; the value is never shown here.") {
            [DialogAction("Give secret", destructive: true) { Task { await give() } }]
        }
    }

    private func note(_ webhook: RoutineWebhook) -> String {
        guard webhook.url != nil else {
            return webhook.needsAccount == true
                ? "This computer isn't in an account, so only it can send to this address. Add it to your account for a public one."
                : "The Codync cloud is off, so only this computer can send to it."
        }
        return "POST with Authorization: Bearer <key>. For GitHub, use content type application/json and the key as the secret. "
            + "Deliveries wait up to 72 hours while this computer is off. They pass through the Codync cloud, which can read them."
    }

    /// A selectable monospaced value with a copy button, plus any extra buttons.
    private func value(_ text: String, copy: String, @ViewBuilder extra: () -> some View = { EmptyView() }) -> some View {
        HStack(alignment: .center, spacing: 8) {
            Text(text)
                .font(.callout.monospaced())
                .lineLimit(2)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
            extra()
            IconButton(copied == copy ? "Copied" : "Copy", systemImage: copied == copy ? "checkmark" : "doc.on.doc") {
                guard let webhook else { return }
                Pasteboard.copy(copy == "key" ? webhook.key : webhook.url ?? webhook.localUrl)
                withAnimation(Motion.reduced(Motion.hover, reduceMotion)) { copied = copy }
            }
        }
        .padding(.vertical, 8)
        .padding(.leading, 12)
        .padding(.trailing, 6)
        .background(Palette.bubbleUser, in: RoundedRectangle(cornerRadius: 12))
    }

    /// Rotates the key into the agent's own vault; the host returns only the saved key's name.
    private func give() async {
        guard let client = model.client else { error = "Connect to this computer to give the secret."; return }
        giving = true
        defer { giving = false }
        do {
            struct Given: Decodable { var name: String }
            struct Args: Encodable { var id: String }
            struct Body: Encodable { var botId: String; var name: String; var arguments: Args }
            let result: Given = try await client.call("routineCall",
                                                      Body(botId: botId, name: "give_webhook_secret", arguments: Args(id: routineId)))
            givenName = result.name
            showKey = false
            await load(rotate: false)
        } catch { self.error = error.localizedDescription }
    }

    private func load(rotate: Bool) async {
        guard let client = model.client else { error = "Connect to this computer to see the webhook."; return }
        rotating = rotate
        defer { rotating = false }
        do {
            let value = try await client.routineWebhook(botId: botId, id: routineId, rotate: rotate)
            withAnimation(Motion.reduced(Motion.hover, reduceMotion)) {
                webhook = value
                if rotate { copied = nil; showKey = true }
            }
            error = nil
        } catch { self.error = error.localizedDescription }
    }
}
