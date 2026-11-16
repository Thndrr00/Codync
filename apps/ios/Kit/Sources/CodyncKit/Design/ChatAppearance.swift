import Foundation
import SwiftUI
import UIKit

/// One of the 11 chat colours (OpenBot 0.65), with the shade that reads on each theme. The list is
/// the same in the desktop app, Android and the terminal UI (`host/src/tui/appearance.rs`).
public struct ChatColour: Identifiable, Hashable, Sendable {
    public let id: String
    public let light: UInt32
    public let dark: UInt32

    public var label: String { L10n.tr(id.capitalized) }

    public func hex(dark: Bool) -> UInt32 { dark ? self.dark : light }
}

public enum ChatColours {
    public static let all: [ChatColour] = [
        .init(id: "red", light: 0xE35652, dark: 0xF8706D),
        .init(id: "orange", light: 0xD86706, dark: 0xED8125),
        .init(id: "yellow", light: 0xD39809, dark: 0xE8B027),
        .init(id: "green", light: 0x06A74E, dark: 0x25BF69),
        .init(id: "cyan", light: 0x0DA195, dark: 0x2BB9AD),
        .init(id: "sky", light: 0x069BC2, dark: 0x24B2D8),
        .init(id: "blue", light: 0x408DEA, dark: 0x5CA5FE),
        .init(id: "indigo", light: 0x7B7DEB, dark: 0x9596FF),
        .init(id: "violet", light: 0xA66BE0, dark: 0xBD85F5),
        .init(id: "magenta", light: 0xCA5BB6, dark: 0xE076CD),
        .init(id: "pink", light: 0xDD5487, dark: 0xF26FA0),
    ]

    public static func colour(_ id: String?) -> ChatColour? {
        guard let id else { return nil }
        return all.first { $0.id == id }
    }
}

/// A calm tint behind the chat for each theme, in the same order as `ChatColours`.
public struct ChatBackground: Identifiable, Hashable, Sendable {
    public let id: String
    public let light: UInt32
    public let dark: UInt32

    public var label: String { L10n.tr(id.capitalized) }

    public func hex(dark: Bool) -> UInt32 { dark ? self.dark : light }
}

public enum ChatBackgrounds {
    public static let all: [ChatBackground] = [
        .init(id: "red", light: 0xFFEFED, dark: 0x311B19),
        .init(id: "orange", light: 0xFEF0E9, dark: 0x2F1D12),
        .init(id: "yellow", light: 0xFAF2E6, dark: 0x2A200D),
        .init(id: "green", light: 0xEBF7ED, dark: 0x152619),
        .init(id: "cyan", light: 0xE7F8F5, dark: 0x0A2724),
        .init(id: "sky", light: 0xE7F6FD, dark: 0x0C252E),
        .init(id: "blue", light: 0xECF4FF, dark: 0x162232),
        .init(id: "indigo", light: 0xF1F2FF, dark: 0x1E2032),
        .init(id: "violet", light: 0xF6F1FE, dark: 0x251D2F),
        .init(id: "magenta", light: 0xFCEFF8, dark: 0x2C1B28),
        .init(id: "pink", light: 0xFFEFF3, dark: 0x2F1A21),
    ]

    public static func background(_ id: String?) -> ChatBackground? {
        guard let id else { return nil }
        return all.first { $0.id == id }
    }
}

/// A saved chat colour or background: one choice per appearance. `nil` means the theme's own.
public struct ChatChoice: Equatable, Sendable {
    public var light: String?
    public var dark: String?

    public init(light: String? = nil, dark: String? = nil) {
        self.light = light
        self.dark = dark
    }

    /// `"light/dark"`, with `-` for none. A single id saved by an older build means both themes,
    /// so a colour chosen before the split survives it.
    public static func decode(_ stored: String, known: [String]) -> ChatChoice {
        let value = stored.trimmingCharacters(in: .whitespacesAndNewlines)
        let parts = value.split(separator: "/", maxSplits: 1, omittingEmptySubsequences: false)
        let light = parts.first.map(String.init) ?? value
        let dark = parts.count > 1 ? String(parts[1]) : light
        let pick = { (id: String) -> String? in known.contains(id) ? id : nil }
        return ChatChoice(light: pick(light), dark: pick(dark))
    }

    public var encoded: String {
        "\(light ?? "-")/\(dark ?? "-")"
    }

    public func id(dark: Bool) -> String? {
        dark ? self.dark : light
    }

    /// The next choice in the list: none first, then each preset, then none again.
    public func cycling(dark: Bool, ids: [String]) -> ChatChoice {
        let current = id(dark: dark)
        let next: String?
        if let current, let at = ids.firstIndex(of: current), ids.indices.contains(at + 1) {
            next = ids[at + 1]
        } else {
            next = current == nil ? ids.first : nil
        }
        return dark ? ChatChoice(light: light, dark: next) : ChatChoice(light: next, dark: dark)
    }
}

/// Where the chat colour and chat background are saved on this device, and how they paint.
public enum ChatAppearance {
    public static let colourKey = "chatColour"
    public static let backgroundKey = "chatBackground"

    public static func choice(_ key: String, known: [String], defaults: UserDefaults = .standard) -> ChatChoice {
        ChatChoice.decode(defaults.string(forKey: key) ?? "", known: known)
    }

    public static func set(_ choice: ChatChoice, key: String, defaults: UserDefaults = .standard) {
        defaults.set(choice.encoded, forKey: key)
    }

    /// The person's own message bubbles: the theme's own fill, tinted with the chosen colour.
    public static func userBubble(dark: Bool, defaults: UserDefaults = .standard) -> Color {
        let base = UIColor(Palette.bubbleUser)
        guard let colour = ChatColours.colour(choice(colourKey, known: ChatColours.all.map(\.id), defaults: defaults).id(dark: dark))
        else { return Palette.bubbleUser }
        return Color(uiColor: base.mixed(with: UIColor(hex: colour.hex(dark: dark)), amount: dark ? 0.22 : 0.14))
    }

    /// The colour behind the chat area, or the theme's own background.
    public static func background(dark: Bool, defaults: UserDefaults = .standard) -> Color {
        guard let background = ChatBackgrounds.background(
            choice(backgroundKey, known: ChatBackgrounds.all.map(\.id), defaults: defaults).id(dark: dark)
        ) else { return Palette.background }
        return Color(hex: background.hex(dark: dark))
    }
}

extension UIColor {
    /// `self` moved `amount` of the way to `other`, for a bubble tint that keeps the theme's ink readable.
    func mixed(with other: UIColor, amount: CGFloat) -> UIColor {
        var (r1, g1, b1, a1): (CGFloat, CGFloat, CGFloat, CGFloat) = (0, 0, 0, 0)
        var (r2, g2, b2, a2): (CGFloat, CGFloat, CGFloat, CGFloat) = (0, 0, 0, 0)
        _ = getRed(&r1, green: &g1, blue: &b1, alpha: &a1)
        _ = other.getRed(&r2, green: &g2, blue: &b2, alpha: &a2)
        let t = min(max(amount, 0), 1)
        return UIColor(red: r1 + (r2 - r1) * t, green: g1 + (g2 - g1) * t, blue: b1 + (b2 - b1) * t, alpha: a1 + (a2 - a1) * t)
    }
}
