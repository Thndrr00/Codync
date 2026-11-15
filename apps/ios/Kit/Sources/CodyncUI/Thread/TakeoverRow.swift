import CodyncKit
import SwiftUI

/// A step a bot stopped at for the person (a CAPTCHA, a login, a payment). They take it over, which holds the
/// agent, and hand it back when done.
struct TakeoverRow: View {
    let entry: Entry
    let request: TakeoverRequest
    @Environment(BotStore.self) private var model

    private var answering: String? { model.answering[entry.id] }
    private var live: Bool { request.isLive }
    private var taken: Bool { request.status == "taken" }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Label(
                    L10n.tr(taken ? "Takeover taken" : "Human Takeover Requested"),
                    systemImage: "hand.raised.fill"
                )
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(Palette.warning)
                Spacer(minLength: 0)
                if answering != nil { Spinner(size: 14) }
            }
            if let reason = request.reason, !reason.isEmpty {
                Text(reason).font(.subheadline).foregroundStyle(Palette.secondary)
            }
            if let url = request.url, let link = URL(string: url), !url.isEmpty {
                Link(url, destination: link).font(.caption)
            }
            if taken, let by = request.takenBy, !by.isEmpty {
                Text(L10n.tr("Taken by {who}").replacingOccurrences(of: "{who}", with: by))
                    .font(.caption)
                    .foregroundStyle(Palette.tertiary)
            }
            if live {
                HStack(spacing: 8) {
                    if taken {
                        Button(L10n.tr("Done, hand back")) { model.resolveTakeover(entry) }
                            .buttonStyle(PrimaryButtonStyle())
                    } else if request.status == "waiting" {
                        Button(L10n.tr("Take over")) { model.takeTakeover(entry) }
                            .buttonStyle(PrimaryButtonStyle())
                    }
                    Button(L10n.tr("Cancel")) { model.stop(entry.botId) }
                        .buttonStyle(.secondary)
                }
                .disabled(answering != nil)
            } else {
                Text(L10n.tr(outcome))
                    .font(.caption.weight(.medium))
                    .foregroundStyle(Palette.secondary)
            }
        }
        .padding(16)
        .frame(maxWidth: 420, alignment: .leading)
        .background(Palette.bubbleAgent, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
        .padding(.trailing, 40)
    }

    private var outcome: String {
        switch request.status {
        case "resolved": return "Completed"
        case "expired": return "Expired"
        default: return "Cancelled"
        }
    }
}
