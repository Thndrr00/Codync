import Foundation

/// Another person in a chat: one of your devices, named and coloured by the computer
/// (`presence`), so two people on one computer are told apart.
public struct Person: Codable, Hashable, Sendable, Identifiable {
    /// The device's key.
    public var key: String
    public var name: String
    /// One of the avatar colours.
    public var color: String
    /// Whether they are typing now.
    public var typing: Bool

    public var id: String { key }

    public init(key: String, name: String, color: String, typing: Bool = false) {
        self.key = key
        self.name = name
        self.color = color
        self.typing = typing
    }

    /// A message's author carries no typing flag; presence always does.
    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        key = try container.decode(String.self, forKey: .key)
        name = try container.decode(String.self, forKey: .name)
        color = try container.decodeIfPresent(String.self, forKey: .color) ?? "blue"
        typing = try container.decodeIfPresent(Bool.self, forKey: .typing) ?? false
    }

    /// Up to two letters of the name, for its small face.
    public var initials: String {
        name.split(separator: " ").prefix(2).map { String($0.prefix(1)).uppercased() }.joined()
    }
}
