/// A place the bot can answer in. The linked number's own "Message yourself"
/// chat is always one of them, even on a number used only for OpenBot with a
/// non-empty allowlist (OpenBot nightly triage-13865).
struct WhatsAppPlace: Equatable {
    let id: String
    let own: Bool
}

enum WhatsAppPlaces {
    static func list(me: String?, allowed: [String]) -> [WhatsAppPlace] {
        var own: [WhatsAppPlace] = []
        if let me, !me.trimmingCharacters(in: .whitespaces).isEmpty {
            own.append(WhatsAppPlace(id: me, own: true))
        }
        return own + allowed.filter { $0 != me }.map { WhatsAppPlace(id: $0, own: false) }
    }
}
