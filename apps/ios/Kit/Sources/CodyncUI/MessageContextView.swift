import CodyncKit
import SwiftUI

/// One message and its neighbours, read through the host's `messageContext` (the call the desktop
/// uses for search jumps). Tapping a quote or a search result shows the same span with the target
/// highlighted and a way back into the chat.
/// Ported from OpenBot 0.70.0's message jump.
struct MessageContextRequest: Identifiable {
    let id = UUID()
    let botId: String
    let entryId: String
    let threadId: String?
}

struct MessageContextView: View {
    let request: MessageContextRequest
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var entries: [Entry]?
    @State private var error: String?

    var body: some View {
        VStack(spacing: 8) {
            ModalHeader(L10n.tr("Message context"))
            Button(L10n.tr("Continue in chat")) { dismiss() }
                .buttonStyle(.primary)
                .padding(.horizontal, 16)
            if let error {
                Text(error).font(.caption).foregroundStyle(Palette.danger)
                    .frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 16)
            }
            if entries == nil && error == nil {
                HStack(spacing: 8) {
                    Spinner(size: 16)
                    Text(L10n.tr("Loading…")).foregroundStyle(Palette.secondary)
                }
            }
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        ForEach(entries ?? []) { entry in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(entry.id == request.entryId ? L10n.tr("Matching message")
                                     : entry.kind == "user" ? L10n.tr("You") : L10n.tr("Agent"))
                                    .font(.caption).foregroundStyle(Palette.secondary)
                                Text(MessagePreview.text(entry.data.text ?? "")).textSelection(.enabled)
                            }
                            .id(entry.id)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(12)
                            .background(entry.id == request.entryId ? Palette.accent.opacity(0.12) : Palette.surface,
                                        in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                            .accessibilityElement(children: .combine)
                        }
                    }
                    .padding(16)
                }
                .onChange(of: entries) { _, loaded in
                    guard loaded != nil else { return }
                    withAnimation(Motion.reduced(Motion.layout, reduceMotion)) {
                        proxy.scrollTo(request.entryId, anchor: .center)
                    }
                }
            }
        }
        .background(Palette.background)
        .task(id: request.entryId) {
            entries = nil
            error = nil
            do {
                entries = try await model.messageContext(botId: request.botId, entryId: request.entryId)
            } catch is CancellationError {
            } catch {
                self.error = error.localizedDescription
            }
        }
    }
}
