import Foundation

/// What a server's Library shows (`serverLibrary`): everything, only images, or only other files.
public enum ServerLibraryFilter: String, CaseIterable, Sendable {
    case all, images, files

    /// The filter's name in the picker.
    public var title: String {
        switch self {
        case .all: "All"
        case .images: "Images"
        case .files: "Files"
        }
    }

    /// Whether an item shows under this filter; the host applies the same rule to `image`.
    public func matches(_ item: ServerLibraryItem) -> Bool {
        switch self {
        case .all: true
        case .images: item.image
        case .files: !item.image
        }
    }

    /// The items this filter keeps, in their order (newest first).
    public func apply(to items: [ServerLibraryItem]) -> [ServerLibraryItem] {
        filter { matches($0) }
    }
}

/// One thing in a server's Library: a chat's item, with the bot that published or shared it, newest first.
public struct ServerLibraryItem: Hashable, Sendable, Identifiable {
    public var botId: String
    public var botName: String
    /// A picture the apps preview (png, jpg, jpeg, heic, gif, webp, tiff, bmp), as the host marks it.
    public var image: Bool
    public var item: ChatLibraryItem

    public init(botId: String, botName: String, image: Bool, item: ChatLibraryItem) {
        self.botId = botId
        self.botName = botName
        self.image = image
        self.item = item
    }

    public var id: String { "\(botId)/\(item.id)" }
    public var at: Int64 { item.at }
    public var date: Date { item.date }

    /// What `serverLibrary` sends: the chat item's shape plus `botId`, `botName` and `image`.
    public struct Wire: Decodable {
        var item: ChatLibraryItem.Wire
        var botId: String
        var botName: String
        var image: Bool

        private enum CodingKeys: String, CodingKey { case botId, botName, image }

        public init(from decoder: any Decoder) throws {
            item = try ChatLibraryItem.Wire(from: decoder)
            let c = try decoder.container(keyedBy: CodingKeys.self)
            botId = try c.decode(String.self, forKey: .botId)
            botName = (try? c.decode(String.self, forKey: .botName)) ?? ""
            image = (try? c.decode(Bool.self, forKey: .image)) ?? false
        }

        /// The item, or nil for a kind this app can't read.
        var serverItem: ServerLibraryItem? {
            item.item.map { ServerLibraryItem(botId: botId, botName: botName, image: image, item: $0) }
        }
    }
}
