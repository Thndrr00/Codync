import CodyncKit
import SwiftUI

/// Who else is in this chat right now, over the message box: their initials in their colour, and
/// "Ana is typing…" while a key comes from them. Port of OpenBot's team presence line
/// (`TeamPresenceMember`, `TeamPersonAvatar`), with the devices paired to this computer as the
/// people. Nothing shows while this is the only one here.
struct PresenceRow: View {
    let botId: String
    @Environment(BotStore.self) private var model

    /// How often the view asks who else is in the chat; the computer forgets a device thirty
    /// seconds after it last looked, so this also keeps this person's own place alive while reading.
    private static let poll: Duration = .seconds(3)

    var body: some View {
        let people = model.people[botId] ?? []
        VStack(spacing: 0) {
            if !people.isEmpty {
                let typing = people.filter(\.typing)
                HStack(spacing: 6) {
                    HStack(spacing: -5) {
                        ForEach(people.prefix(3)) { Face(person: $0) }
                    }
                    Text(typing.isEmpty ? hereLine(people) : typingLine(typing))
                        .font(.caption2)
                        .foregroundStyle(Palette.secondary)
                        .lineLimit(1)
                        .truncationMode(.tail)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 18)
            }
        }
        .animation(Motion.layout, value: people.isEmpty)
        // Asks now and then while the row is on screen.
        .task(id: botId) { await poll() }
    }

    /// Asks now and then while the row is on screen.
    private func poll() async {
        while !Task.isCancelled {
            model.look(botId: botId, typing: model.isTyping(botId) ? true : nil)
            try? await Task.sleep(for: Self.poll)
        }
    }
}

/// One person's small face: their initials on their colour.
private struct Face: View {
    let person: Person

    var body: some View {
        Text(person.initials)
            .font(.system(size: 8, weight: .bold))
            .foregroundStyle(Palette.onAccent)
            .frame(width: 17, height: 17)
            .background(AvatarPalette.color(person.color), in: .circle)
            .overlay(Circle().stroke(Palette.background, lineWidth: 2).padding(-2))
            .scaleEffect(person.typing ? 1.1 : 1)
            .animation(.easeInOut(duration: 1.3).repeatForever(autoreverses: true), value: person.typing)
    }
}

/// "Ana is typing…" / "Ana and Bo are typing…" / "Ana, Bo and Cy are typing…".
private func typingLine(_ people: [Person]) -> String {
    guard people.count > 2 else {
        if let b = people.dropFirst().first {
            return L10n.tr("{a} and {b} are typing…")
                .replacingOccurrences(of: "{a}", with: people[0].name)
                .replacingOccurrences(of: "{b}", with: b.name)
        }
        return L10n.tr("{name} is typing…").replacingOccurrences(of: "{name}", with: people[0].name)
    }
    return L10n.tr("{names} are typing…").replacingOccurrences(of: "{names}", with: people.map(\.name).joined(separator: ", "))
}

/// "Ana is here" / "Ana and Bo are here" / "Ana, Bo and 2 more are here".
private func hereLine(_ people: [Person]) -> String {
    guard people.count > 2 else {
        if let b = people.dropFirst().first {
            return L10n.tr("{a} and {b} are here")
                .replacingOccurrences(of: "{a}", with: people[0].name)
                .replacingOccurrences(of: "{b}", with: b.name)
        }
        return L10n.tr("{name} is here").replacingOccurrences(of: "{name}", with: people[0].name)
    }
    return L10n.tr("{names} and {count} more are here")
        .replacingOccurrences(of: "{names}", with: people.prefix(2).map(\.name).joined(separator: ", "))
        .replacingOccurrences(of: "{count}", with: "\(people.count - 2)")
}
