import Foundation

/// A computer's ceiling on local mini-apps (`localApps`): how many it holds now, the limit the person
/// picked, the values it offers and how many bots made them. It is a safeguard on that computer, not a
/// plan: only a new app takes a slot, and a bot may always update an app it already made.
/// Mirrors `host/src/local_apps.rs`; hosts before the cap answer "unknown method".
public struct LocalApps: Decodable, Hashable, Sendable {
    /// How many mini-apps the computer holds now.
    public var count = 0
    /// The ceiling the person picked.
    public var limit = LocalApps.defaultLimit
    /// The values the computer offers.
    public var choices = LocalApps.defaultChoices
    /// How many bots made them.
    public var botsWithApps = 0

    /// The ceiling until the person changes it, as the host has it.
    public static let defaultLimit = 100
    /// The values the host offers until it says otherwise.
    public static let defaultChoices = [25, 50, 100, 250, 1_000]

    private enum CodingKeys: String, CodingKey {
        case count, limit, choices, botsWithApps
    }

    /// Lenient: a field this app's host doesn't send (or one a newer host renames) falls back, so one
    /// value never costs the whole card.
    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        count = try container.decodeIfPresent(Int.self, forKey: .count) ?? 0
        limit = try container.decodeIfPresent(Int.self, forKey: .limit) ?? LocalApps.defaultLimit
        let offered = try container.decodeIfPresent([Int].self, forKey: .choices) ?? []
        choices = offered.isEmpty ? LocalApps.defaultChoices : offered
        botsWithApps = try container.decodeIfPresent(Int.self, forKey: .botsWithApps) ?? 0
    }
}
