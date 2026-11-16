import Foundation

// Wire types for the bdots-inspired features the host already serves (mini-apps, capability links,
// open items, proactive settings, action-class policies and connector health). They live in the
// shared kit so the iPhone app can decode the same payloads as Desktop and Android.
// Field names match the host's camelCase JSON; older hosts omit newer fields, so the decoders fill defaults.

// MARK: - Action policies (Idea 43)

public enum ActionClass: String, Codable, Hashable, Sendable, CaseIterable {
    case files, shell, browse, browser, apps, bots, routines
}

public enum RulePolicy: String, Codable, Hashable, Sendable {
    case allow, ask, block
}

public struct ActionClassInfo: Codable, Hashable, Sendable {
    public var id: ActionClass
    public var label: String
    public var hint: String

    public init(id: ActionClass, label: String, hint: String) {
        self.id = id
        self.label = label
        self.hint = hint
    }
}

extension ActionClass {
    /// The seven classes in the order the host and other clients show them.
    public static let catalog: [ActionClassInfo] = [
        ActionClassInfo(id: .files, label: "Files", hint: "Read, write or edit workspace files"),
        ActionClassInfo(id: .shell, label: "Shell", hint: "Run commands and background jobs"),
        ActionClassInfo(id: .browse, label: "Web fetch", hint: "Fetch public URLs and search"),
        ActionClassInfo(id: .browser, label: "Browser control", hint: "Drive pages and click controls"),
        ActionClassInfo(id: .apps, label: "Connected apps", hint: "Call MCP connectors and secrets"),
        ActionClassInfo(id: .bots, label: "Team bots", hint: "Delegate to or message other bots"),
        ActionClassInfo(id: .routines, label: "Routines", hint: "Create or modify scheduled routines"),
    ]
}

// MARK: - Avatars (Idea 21)

public enum RobotLook: String, Codable, Hashable, Sendable, CaseIterable {
    case round, visor, dome, boxy, cyclops, ears, scout, owl, pixel, astro, cat, hex
}

public struct RobotLookInfo: Codable, Hashable, Sendable {
    public var id: RobotLook
    public var label: String

    public init(id: RobotLook, label: String) {
        self.id = id
        self.label = label
    }
}

extension RobotLook {
    /// The twelve chassis the host assigns in order to new bots.
    public static let catalog: [RobotLookInfo] = [
        RobotLookInfo(id: .round, label: "Round"),
        RobotLookInfo(id: .visor, label: "Visor"),
        RobotLookInfo(id: .dome, label: "Dome"),
        RobotLookInfo(id: .boxy, label: "Boxy"),
        RobotLookInfo(id: .cyclops, label: "Cyclops"),
        RobotLookInfo(id: .ears, label: "Ears"),
        RobotLookInfo(id: .scout, label: "Scout"),
        RobotLookInfo(id: .owl, label: "Owl"),
        RobotLookInfo(id: .pixel, label: "Pixel"),
        RobotLookInfo(id: .astro, label: "Astro"),
        RobotLookInfo(id: .cat, label: "Cat"),
        RobotLookInfo(id: .hex, label: "Hex"),
    ]
}

// MARK: - Proactive check-ins and background apps (Ideas 41, 44)

public struct ProactiveSettings: Codable, Hashable, Sendable {
    public var enabled: Bool
    public var everyHours: Int
    public var quietStart: Int
    public var quietEnd: Int
    public var timezone: String?
    public var maxPerDay: Int
    public var appsInBackground: Bool

    public init(
        enabled: Bool = false,
        everyHours: Int = 4,
        quietStart: Int = 22,
        quietEnd: Int = 8,
        timezone: String? = nil,
        maxPerDay: Int = 3,
        appsInBackground: Bool = true
    ) {
        self.enabled = enabled
        self.everyHours = everyHours
        self.quietStart = quietStart
        self.quietEnd = quietEnd
        self.timezone = timezone
        self.maxPerDay = maxPerDay
        self.appsInBackground = appsInBackground
    }

    private enum CodingKeys: String, CodingKey {
        case enabled, everyHours, quietStart, quietEnd, timezone, maxPerDay, appsInBackground
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        enabled = try c.decodeIfPresent(Bool.self, forKey: .enabled) ?? false
        everyHours = try c.decodeIfPresent(Int.self, forKey: .everyHours) ?? 4
        quietStart = try c.decodeIfPresent(Int.self, forKey: .quietStart) ?? 22
        quietEnd = try c.decodeIfPresent(Int.self, forKey: .quietEnd) ?? 8
        timezone = try c.decodeIfPresent(String.self, forKey: .timezone)
        maxPerDay = try c.decodeIfPresent(Int.self, forKey: .maxPerDay) ?? 3
        appsInBackground = try c.decodeIfPresent(Bool.self, forKey: .appsInBackground) ?? true
    }
}

// MARK: - Mini-apps and capability links (Ideas 1, 7, 47, 48)

public enum AppLinkMode: String, Codable, Hashable, Sendable {
    case view, edit
}

public struct AppLink: Codable, Identifiable, Hashable, Sendable {
    public var token: String
    public var botId: String
    public var slug: String
    public var label: String
    public var mode: AppLinkMode
    public var url: String?
    public var createdAt: Int64
    public var lastOpenedAt: Int64?
    public var opens: Int

    public var id: String { token }

    private enum CodingKeys: String, CodingKey {
        case token, botId, slug, label, mode, url, createdAt, lastOpenedAt, opens
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        token = try c.decode(String.self, forKey: .token)
        botId = try c.decode(String.self, forKey: .botId)
        slug = try c.decode(String.self, forKey: .slug)
        label = try c.decodeIfPresent(String.self, forKey: .label) ?? "Shared link"
        mode = try c.decodeIfPresent(AppLinkMode.self, forKey: .mode) ?? .view
        url = try c.decodeIfPresent(String.self, forKey: .url)
        createdAt = try c.decodeIfPresent(Int64.self, forKey: .createdAt) ?? 0
        lastOpenedAt = try c.decodeIfPresent(Int64.self, forKey: .lastOpenedAt)
        opens = try c.decodeIfPresent(Int.self, forKey: .opens) ?? 0
    }
}

public struct BotAppCollectionSummary: Codable, Hashable, Sendable {
    public var collection: String
    public var count: Int
    public var updatedAt: Int64

    public init(collection: String, count: Int = 0, updatedAt: Int64 = 0) {
        self.collection = collection
        self.count = count
        self.updatedAt = updatedAt
    }
}

public struct BotApp: Codable, Identifiable, Hashable, Sendable {
    public var slug: String
    public var botId: String
    public var title: String
    public var description: String?
    public var entry: String
    public var version: Int
    public var createdAt: Int64
    public var updatedAt: Int64
    public var ownerToken: String?
    public var url: String
    public var links: [AppLink]
    public var collections: [BotAppCollectionSummary]

    public var id: String { slug }

    private enum CodingKeys: String, CodingKey {
        case slug, botId, title, description, entry, version, createdAt, updatedAt, ownerToken, url, links, collections
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        slug = try c.decode(String.self, forKey: .slug)
        botId = try c.decode(String.self, forKey: .botId)
        title = try c.decode(String.self, forKey: .title)
        description = try c.decodeIfPresent(String.self, forKey: .description)
        entry = try c.decodeIfPresent(String.self, forKey: .entry) ?? "index.html"
        version = try c.decodeIfPresent(Int.self, forKey: .version) ?? 1
        createdAt = try c.decodeIfPresent(Int64.self, forKey: .createdAt) ?? 0
        updatedAt = try c.decodeIfPresent(Int64.self, forKey: .updatedAt) ?? 0
        ownerToken = try c.decodeIfPresent(String.self, forKey: .ownerToken)
        url = try c.decodeIfPresent(String.self, forKey: .url) ?? ""
        links = try c.decodeIfPresent([AppLink].self, forKey: .links) ?? []
        collections = try c.decodeIfPresent([BotAppCollectionSummary].self, forKey: .collections) ?? []
    }
}

// MARK: - Tracked work and connector health (Ideas 8, 50)

public struct OpenItem: Codable, Identifiable, Hashable, Sendable {
    public var id: String
    public var botId: String
    public var title: String
    public var note: String
    public var sourceUrl: String?
    public var createdAt: Int64
    public var updatedAt: Int64
    public var checkAfter: Int64?
    public var status: String

    private enum CodingKeys: String, CodingKey {
        case id, botId, title, note, sourceUrl, createdAt, updatedAt, checkAfter, status
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        botId = try c.decode(String.self, forKey: .botId)
        title = try c.decode(String.self, forKey: .title)
        note = try c.decodeIfPresent(String.self, forKey: .note) ?? ""
        sourceUrl = try c.decodeIfPresent(String.self, forKey: .sourceUrl)
        createdAt = try c.decodeIfPresent(Int64.self, forKey: .createdAt) ?? 0
        updatedAt = try c.decodeIfPresent(Int64.self, forKey: .updatedAt) ?? 0
        checkAfter = try c.decodeIfPresent(Int64.self, forKey: .checkAfter)
        status = try c.decodeIfPresent(String.self, forKey: .status) ?? "open"
    }
}

public struct ConnectorFailure: Codable, Hashable, Sendable {
    public var kind: String
    public var message: String
    public var at: Int64

    private enum CodingKeys: String, CodingKey { case kind, message, at }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        kind = try c.decode(String.self, forKey: .kind)
        message = try c.decode(String.self, forKey: .message)
        at = try c.decodeIfPresent(Int64.self, forKey: .at) ?? 0
    }
}

public struct MessageBookmark: Codable, Identifiable, Hashable, Sendable {
    public var id: String
    public var botId: String
    public var entryId: String
    public var threadId: String?
    public var author: String
    public var preview: String
    public var note: String
    public var createdAt: Int64
    public var updatedAt: Int64

    private enum CodingKeys: String, CodingKey {
        case id, botId, entryId, threadId, author, preview, note, createdAt, updatedAt
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        botId = try c.decode(String.self, forKey: .botId)
        entryId = try c.decode(String.self, forKey: .entryId)
        threadId = try c.decodeIfPresent(String.self, forKey: .threadId)
        author = try c.decodeIfPresent(String.self, forKey: .author) ?? ""
        preview = try c.decodeIfPresent(String.self, forKey: .preview) ?? ""
        note = try c.decodeIfPresent(String.self, forKey: .note) ?? ""
        createdAt = try c.decodeIfPresent(Int64.self, forKey: .createdAt) ?? 0
        updatedAt = try c.decodeIfPresent(Int64.self, forKey: .updatedAt) ?? 0
    }
}
