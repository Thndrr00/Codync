import SwiftUI

/// A device preference; it does not change the host or another client's appearance.
public enum Appearance: String, CaseIterable, Sendable {
    case system, light, dark

    public static let storageKey = "appearance"

    public var label: String {
        switch self {
        case .system: "System"
        case .light: "Light"
        case .dark: "Dark"
        }
    }
}

/// How a bot's character is drawn: plush (shaded dots under a light) or flat (even dots, no light).
/// A device preference like the theme; the App Group store carries it to the widgets too.
public enum AvatarStyle: String, CaseIterable, Sendable {
    case plush, flat

    public static let storageKey = "avatarStyle"

    public var label: String {
        switch self {
        case .plush: L10n.tr("Plush")
        case .flat: L10n.tr("Flat")
        }
    }
}

public extension SharedStore {
    /// The widgets draw the same style as the app, so both read the App Group store.
    static var avatarStyle: AvatarStyle {
        get { UserDefaults(suiteName: appGroup)?.string(forKey: AvatarStyle.storageKey).flatMap(AvatarStyle.init(rawValue:)) ?? .plush }
        set { UserDefaults(suiteName: appGroup)?.set(newValue.rawValue, forKey: AvatarStyle.storageKey) }
    }
}

/// How the person's own bubbles take an agent's avatar colour (Settings › Appearance › Your bubbles):
/// Solid fills them with it, Tint mixes a little into the chat's bubble, Classic keeps today's look.
/// A device preference like the theme: only this device sees it.
public enum YourBubbleStyle: String, CaseIterable, Sendable {
    case solid, tint, classic

    public static let storageKey = "yourBubbles"

    public var label: String {
        switch self {
        case .solid: L10n.tr("Solid")
        case .tint: L10n.tr("Tint")
        case .classic: L10n.tr("Classic")
        }
    }
}

/// The fill and inks the person's bubble takes in one chat.
public struct YourBubbleLook: Sendable {
    public let fill: Color
    public let ink: Color
    /// The quoted line's text and icon.
    public let secondaryInk: Color
}

public extension YourBubbleStyle {
    /// This chat's bubble colours in light or dark, or nil for the Classic look: a group and a chat with no
    /// avatar colour of its own keep it.
    func look(for bot: Bot?, dark: Bool) -> YourBubbleLook? {
        guard let bot, !bot.isGroup, self != .classic else { return nil }
        let avatar = AvatarPalette.colors.first { $0.id == bot.avatarColor }?.hex ?? AvatarPalette.colors[7].hex
        switch self {
        case .tint:
            // A little of the colour mixed into the bubble the chat already has.
            let base: UInt32 = dark ? 0x363636 : 0xE2E2E2
            return YourBubbleLook(
                fill: Color(hex: Self.mix(base, avatar, dark ? 0.22 : 0.14)),
                ink: Palette.text,
                secondaryInk: Palette.secondary
            )
        case .solid:
            let fill = Self.readableFill(avatar)
            let ink = Self.ink(on: fill)
            // The quote and its icon keep the bubble's ink: a faded one would drop below 4.5:1 on some colours.
            return YourBubbleLook(fill: Color(hex: fill), ink: Color(hex: ink), secondaryInk: Color(hex: ink))
        case .classic:
            return nil
        }
    }

    /// `a` mixed toward `b` by `amount` (0 … 1).
    private static func mix(_ a: UInt32, _ b: UInt32, _ amount: Double) -> UInt32 {
        func channel(_ shift: UInt32) -> UInt32 {
            let from = Double((a >> shift) & 0xFF)
            let to = Double((b >> shift) & 0xFF)
            return UInt32((from * (1 - amount) + to * amount).rounded())
        }
        return (channel(16) << 16) | (channel(8) << 8) | channel(0)
    }

    /// WCAG relative luminance (0 black … 1 white).
    private static func luminance(_ hex: UInt32) -> Double {
        func channel(_ shift: UInt32) -> Double {
            let c = Double((hex >> shift) & 0xFF) / 255
            return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }

    /// WCAG contrast between two colours (1 … 21).
    private static func contrast(_ first: UInt32, _ second: UInt32) -> Double {
        let a = luminance(first)
        let b = luminance(second)
        return (max(a, b) + 0.05) / (min(a, b) + 0.05)
    }

    /// The ink that reads best on `fill`: the theme's near-black, or white.
    private static func ink(on fill: UInt32) -> UInt32 {
        contrast(0xFFFFFF, fill) >= contrast(0x141414, fill) ? 0xFFFFFF : 0x141414
    }

    /// `fill` nudged away from its ink until that ink reads at 4.5:1, so a Solid bubble stays legible on
    /// every avatar colour.
    private static func readableFill(_ fill: UInt32) -> UInt32 {
        let ink = ink(on: fill)
        // Away from the ink (toward white for dark ink, black for white ink): the fill keeps its hue.
        let away: UInt32 = ink == 0x141414 ? 0xFFFFFF : 0x000000
        for step in 0...20 {
            let mixed = mix(fill, away, Double(step) / 20)
            if contrast(ink, mixed) >= 4.5 { return mixed }
        }
        return away
    }
}
