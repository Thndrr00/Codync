import CodyncKit
import SwiftUI

/// The "Waiting for replies" card above a chat's (or thread's) composer: one row per teammate the
/// agent asked, with a Stop that drops the request from its queue. Stopped rows stay visible.
struct WaitingRepliesCard: View {
    let botId: String
    var threadId: String? = nil
    @Environment(BotStore.self) private var model
    @State private var hidden = false

    private var rows: [WaitingReply] {
        let inChat = threadId.map { model.threadChat(botId, $0) } ?? model.chat(botId)
        let entries = inChat + Array(model.stoppedRequests.values)
        return WaitingReplies.rows(entries, botId: botId, threadId: threadId,
                                   stopped: Set(model.stoppedRequests.keys),
                                   name: { model.authorName($0) })
    }

    var body: some View {
        let shown = rows
        if !hidden && !shown.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(L10n.tr("Waiting for replies"))
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(Palette.text)
                    Spacer(minLength: 8)
                    if shown.allSatisfy({ $0.state == .stopped }) {
                        Button(L10n.tr("Hide this list")) { withAnimation(Motion.hover) { hidden = true } }
                            .buttonStyle(.plain)
                            .font(.caption)
                            .foregroundStyle(Palette.accent)
                    }
                }
                ForEach(shown) { row in
                    HStack(spacing: 10) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(row.name).font(.footnote.weight(.semibold)).foregroundStyle(Palette.text)
                            Text(row.request).font(.caption).foregroundStyle(Palette.secondary).lineLimit(1)
                        }
                        Spacer(minLength: 8)
                        Text(L10n.tr(row.state == .stopped ? "Stopped" : "Asked"))
                            .font(.caption2)
                            .foregroundStyle(row.state == .stopped ? Palette.tertiary : Palette.accent)
                        if row.state == .asked {
                            Button {
                                Motion.animate { model.stopRequest(row.entry) }
                            } label: {
                                Image(systemName: "xmark").font(.caption.weight(.semibold))
                                    .frame(width: 28, height: 28)
                                    .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel("\(L10n.tr("Stop")) \(row.name)")
                        }
                    }
                }
            }
            .padding(12)
            .background(Palette.bubbleAgent, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .padding(.horizontal, 16)
            .padding(.bottom, 4)
        }
    }
}
