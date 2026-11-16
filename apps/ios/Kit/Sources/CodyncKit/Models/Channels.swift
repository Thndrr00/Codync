import Foundation

// The messaging channels of a computer (host/src/telegram, host/src/slack, host/src/discord,
// host/src/github) and its linked WhatsApp number (host/src/whatsapp.rs). The host owns the
// meaning and keeps the tokens; these shapes only carry what the screens show, and their words
// come from `Channels` and `WhatsApp`.

/// A Telegram bot of the computer, as `telegramStatus` lists it. Never carries the token.
public struct TelegramBot: Decodable, Identifiable, Equatable, Sendable {
    public var name: String
    public var username: String
    public var botId: String
    public var botName: String
    /// The Telegram user ids that may write to it.
    public var allowed: [Int64]
    /// `ok`, `starting`, or `error: …`.
    public var state: String
    public var lastReceivedAt: Int64?
    public var pairingExpiresAt: Int64?

    public var id: String { name }

    private enum CodingKeys: String, CodingKey {
        case name, username, botId, botName, allowed, state, lastReceivedAt, pairingExpiresAt
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        name = try c.decode(String.self, forKey: .name)
        username = try c.decodeIfPresent(String.self, forKey: .username) ?? ""
        botId = try c.decodeIfPresent(String.self, forKey: .botId) ?? ""
        botName = try c.decodeIfPresent(String.self, forKey: .botName) ?? ""
        allowed = try c.decodeIfPresent([Int64].self, forKey: .allowed) ?? []
        state = try c.decodeIfPresent(String.self, forKey: .state) ?? "starting"
        lastReceivedAt = try c.decodeIfPresent(Int64.self, forKey: .lastReceivedAt)
        pairingExpiresAt = try c.decodeIfPresent(Int64.self, forKey: .pairingExpiresAt)
    }
}

/// The answer to a Telegram call: the bots as they are then, and what the call made.
public struct TelegramStatus: Decodable, Sendable {
    public var configured: Bool
    public var bots: [TelegramBot]
    /// The one-time code a `pairTelegram` answer carries.
    public var result: TelegramPairing?

    private enum CodingKeys: String, CodingKey { case configured, bots, result }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        configured = try c.decodeIfPresent(Bool.self, forKey: .configured) ?? false
        bots = try c.decodeIfPresent([TelegramBot].self, forKey: .bots) ?? []
        result = try c.decodeIfPresent(TelegramPairing.self, forKey: .result)
    }
}

public struct TelegramPairing: Decodable, Sendable {
    public var code: String
    public var expiresAt: Int64?

    private enum CodingKeys: String, CodingKey { case code, expiresAt }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        code = try c.decode(String.self, forKey: .code)
        expiresAt = try c.decodeIfPresent(Int64.self, forKey: .expiresAt)
    }
}

/// One bound Matrix room: the host bot that answers it and the people allowed in it.
public struct MatrixBinding: Decodable, Identifiable, Equatable, Sendable {
    public var roomId: String
    public var botId: String
    public var allowedUsers: [String]
    public var memory: String
    public var icon: String

    public var id: String { roomId }

    public init(roomId: String, botId: String, allowedUsers: [String], memory: String = "", icon: String = "") {
        self.roomId = roomId
        self.botId = botId
        self.allowedUsers = allowedUsers
        self.memory = memory
        self.icon = icon
    }

    private enum CodingKeys: String, CodingKey { case roomId, botId, allowedUsers, memory, icon }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        roomId = try c.decode(String.self, forKey: .roomId)
        botId = try c.decodeIfPresent(String.self, forKey: .botId) ?? ""
        allowedUsers = try c.decodeIfPresent([String].self, forKey: .allowedUsers) ?? []
        memory = try c.decodeIfPresent(String.self, forKey: .memory) ?? ""
        icon = try c.decodeIfPresent(String.self, forKey: .icon) ?? ""
    }
}

/// The Matrix connection, as `matrixStatus` reports it.
public struct MatrixStatus: Decodable, Equatable, Sendable {
    public var configured: Bool
    public var homeserver: String?
    public var userId: String?
    public var deviceId: String?
    public var bindings: [MatrixBinding]
    /// `ok`, `starting`, or `error: …`.
    public var state: String

    private enum CodingKeys: String, CodingKey { case configured, homeserver, userId, deviceId, bindings, state }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        configured = try c.decodeIfPresent(Bool.self, forKey: .configured) ?? false
        homeserver = try c.decodeIfPresent(String.self, forKey: .homeserver)
        userId = try c.decodeIfPresent(String.self, forKey: .userId)
        deviceId = try c.decodeIfPresent(String.self, forKey: .deviceId)
        bindings = try c.decodeIfPresent([MatrixBinding].self, forKey: .bindings) ?? []
        state = try c.decodeIfPresent(String.self, forKey: .state) ?? "starting"
    }
}

/// The Slack connection, as `slackStatus` reports it.
public struct SlackStatus: Decodable, Equatable, Sendable {
    public var configured: Bool
    public var socketMode: Bool
    public var workspaceId: String?
    public var bindings: [SlackBinding]

    private enum CodingKeys: String, CodingKey { case configured, socketMode, workspaceId, bindings }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        configured = try c.decodeIfPresent(Bool.self, forKey: .configured) ?? false
        socketMode = try c.decodeIfPresent(Bool.self, forKey: .socketMode) ?? false
        workspaceId = try c.decodeIfPresent(String.self, forKey: .workspaceId)
        bindings = try c.decodeIfPresent([SlackBinding].self, forKey: .bindings) ?? []
    }
}

/// One bound Slack channel: the host bot that answers it and the people allowed in it.
public struct SlackBinding: Decodable, Identifiable, Equatable, Sendable {
    public var channelId: String
    public var botId: String
    public var allowedUsers: [String]
    public var memory: String
    public var icon: String

    public var id: String { channelId }

    private enum CodingKeys: String, CodingKey { case channelId, botId, allowedUsers, memory, icon }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        channelId = try c.decode(String.self, forKey: .channelId)
        botId = try c.decodeIfPresent(String.self, forKey: .botId) ?? ""
        allowedUsers = try c.decodeIfPresent([String].self, forKey: .allowedUsers) ?? []
        memory = try c.decodeIfPresent(String.self, forKey: .memory) ?? ""
        icon = try c.decodeIfPresent(String.self, forKey: .icon) ?? ""
    }
}

/// One messaging conversation linked to a Codync thread (card K13): what `conversationLinks` lists and
/// `linkConversation` and `unlinkConversation` change, so the app, a call and the channel share one conversation.
public struct ConversationLink: Decodable, Identifiable, Equatable, Sendable {
    public var botId: String
    /// `telegram`, `slack`, `discord` or `matrix`.
    public var source: String
    public var channel: String
    /// The provider's thread inside the channel: a Slack thread's timestamp, empty for a Telegram chat or Discord channel.
    public var thread: String
    /// The Codync thread the conversation writes into.
    public var rootId: String
    public var createdAt: Int64
    public var updatedAt: Int64

    public var id: String { "\(channel)/\(thread)" }

    private enum CodingKeys: String, CodingKey { case botId, source, channel, thread, rootId, createdAt, updatedAt }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        botId = try c.decode(String.self, forKey: .botId)
        source = try c.decode(String.self, forKey: .source)
        channel = try c.decode(String.self, forKey: .channel)
        thread = try c.decodeIfPresent(String.self, forKey: .thread) ?? ""
        rootId = try c.decode(String.self, forKey: .rootId)
        createdAt = try c.decodeIfPresent(Int64.self, forKey: .createdAt) ?? 0
        updatedAt = try c.decodeIfPresent(Int64.self, forKey: .updatedAt) ?? 0
    }
}

/// One bound Discord channel: the host bot that answers it and the people allowed in it.
public struct DiscordBinding: Decodable, Identifiable, Equatable, Sendable {
    public var channelId: String
    public var botId: String
    public var allowedUsers: [String]

    public var id: String { channelId }

    public init(channelId: String, botId: String, allowedUsers: [String]) {
        self.channelId = channelId
        self.botId = botId
        self.allowedUsers = allowedUsers
    }

    private enum CodingKeys: String, CodingKey { case channelId, botId, allowedUsers }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        channelId = try c.decode(String.self, forKey: .channelId)
        botId = try c.decodeIfPresent(String.self, forKey: .botId) ?? ""
        allowedUsers = try c.decodeIfPresent([String].self, forKey: .allowedUsers) ?? []
    }
}

/// The Discord connection, as `discordStatus` reports it.
public struct DiscordStatus: Decodable, Equatable, Sendable {
    public var configured: Bool
    public var guildId: String?
    public var botUserId: String?
    public var bindings: [DiscordBinding]
    /// `ok`, `starting`, or `error: …`.
    public var state: String

    private enum CodingKeys: String, CodingKey { case configured, guildId, botUserId, bindings, state }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        configured = try c.decodeIfPresent(Bool.self, forKey: .configured) ?? false
        guildId = try c.decodeIfPresent(String.self, forKey: .guildId)
        botUserId = try c.decodeIfPresent(String.self, forKey: .botUserId)
        bindings = try c.decodeIfPresent([DiscordBinding].self, forKey: .bindings) ?? []
        state = try c.decodeIfPresent(String.self, forKey: .state) ?? "starting"
    }
}

/// One Signal chat bound to a bot: a phone number or `group:<id>`.
public struct SignalBinding: Decodable, Identifiable, Equatable, Sendable {
    public var chat: String
    public var botId: String
    public var allowedUsers: [String]
    public var memory: String
    public var icon: String

    public var id: String { chat }

    public init(chat: String, botId: String, allowedUsers: [String], memory: String = "", icon: String = "") {
        self.chat = chat
        self.botId = botId
        self.allowedUsers = allowedUsers
        self.memory = memory
        self.icon = icon
    }

    private enum CodingKeys: String, CodingKey { case chat, botId, allowedUsers, memory, icon }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        chat = try c.decode(String.self, forKey: .chat)
        botId = try c.decodeIfPresent(String.self, forKey: .botId) ?? ""
        allowedUsers = try c.decodeIfPresent([String].self, forKey: .allowedUsers) ?? []
        memory = try c.decodeIfPresent(String.self, forKey: .memory) ?? ""
        icon = try c.decodeIfPresent(String.self, forKey: .icon) ?? ""
    }
}

/// The signal-cli connection, as `signalStatus` reports it. A daemon that is not running is a
/// state, and `setup` names the exact commands; the number stays on the computer.
public struct SignalStatus: Decodable, Equatable, Sendable {
    public var configured: Bool
    public var number: String?
    public var daemon: String?
    public var bindings: [SignalBinding]
    /// `ok`, `starting`, `notRunning: ...` or `error: ...`.
    public var state: String
    public var setup: String?

    private enum CodingKeys: String, CodingKey { case configured, number, daemon, bindings, state, setup }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        configured = try c.decodeIfPresent(Bool.self, forKey: .configured) ?? false
        number = try c.decodeIfPresent(String.self, forKey: .number)
        daemon = try c.decodeIfPresent(String.self, forKey: .daemon)
        bindings = try c.decodeIfPresent([SignalBinding].self, forKey: .bindings) ?? []
        state = try c.decodeIfPresent(String.self, forKey: .state) ?? "starting"
        setup = try c.decodeIfPresent(String.self, forKey: .setup)
    }
}

/// The Microsoft Teams connection, as `teamsStatus` reports it.
public struct TeamsStatus: Decodable, Equatable, Sendable {
    public var configured: Bool
    public var appId: String?
    public var tenantId: String?
    public var botId: String?
    public var managedIdentity: Bool
    public var allowedUsers: [String]
    public var allowedTenants: [String]
    public var memory: String
    public var icon: String
    /// The path the Azure Bot resource's messaging endpoint points at.
    public var endpoint: String?
    /// The Azure steps; the host cannot create the registration.
    public var registration: String
    public var state: String

    private enum CodingKeys: String, CodingKey {
        case configured, appId, tenantId, botId, managedIdentity, allowedUsers, allowedTenants, memory, icon,
             endpoint, registration, state
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        configured = try c.decodeIfPresent(Bool.self, forKey: .configured) ?? false
        appId = try c.decodeIfPresent(String.self, forKey: .appId)
        tenantId = try c.decodeIfPresent(String.self, forKey: .tenantId)
        botId = try c.decodeIfPresent(String.self, forKey: .botId)
        managedIdentity = try c.decodeIfPresent(Bool.self, forKey: .managedIdentity) ?? false
        allowedUsers = try c.decodeIfPresent([String].self, forKey: .allowedUsers) ?? []
        allowedTenants = try c.decodeIfPresent([String].self, forKey: .allowedTenants) ?? []
        memory = try c.decodeIfPresent(String.self, forKey: .memory) ?? ""
        icon = try c.decodeIfPresent(String.self, forKey: .icon) ?? ""
        endpoint = try c.decodeIfPresent(String.self, forKey: .endpoint)
        registration = try c.decodeIfPresent(String.self, forKey: .registration) ?? ""
        state = try c.decodeIfPresent(String.self, forKey: .state) ?? "starting"
    }
}

/// The GitHub connection, as `githubStatus` reports it. Never carries the token.
public struct GitHubStatus: Decodable, Equatable, Sendable {
    public var configured: Bool
    public var login: String?
    public var scopes: [String]
    /// The token may create repositories (Administration, or a classic token's `repo` scope).
    public var canCreateRepos: Bool
    /// The repository the last `githubCreateRepo` made; only that answer carries it.
    public var created: CreatedRepo?

    private enum CodingKeys: String, CodingKey { case configured, login, scopes, canCreateRepos, created }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        configured = try c.decodeIfPresent(Bool.self, forKey: .configured) ?? false
        login = try c.decodeIfPresent(String.self, forKey: .login)
        scopes = try c.decodeIfPresent([String].self, forKey: .scopes) ?? []
        canCreateRepos = try c.decodeIfPresent(Bool.self, forKey: .canCreateRepos) ?? false
        created = try c.decodeIfPresent(CreatedRepo.self, forKey: .created)
    }
}

/// A repository `githubCreateRepo` just created.
public struct CreatedRepo: Decodable, Equatable, Sendable {
    public var fullName: String
    public var htmlUrl: String
    public var cloneUrl: String
    /// The host pointed the workspace at it, because the workspace had no origin.
    public var originSet: Bool
}

/// Someone waiting for the owner to let them write to a linked number.
public struct WhatsAppRequest: Decodable, Equatable, Sendable {
    public var code: String
    public var sender: String
    public var name: String?
    /// When the request arrived, in Unix milliseconds.
    public var at: Int64
}

/// A person a linked number knows: their name and the facts they gave.
public struct WhatsAppPerson: Decodable, Equatable, Sendable {
    public var sender: String
    public var name: String?
    public var facts: [String]
}

/// The computer's linked WhatsApp number, as `whatsappState` reports it.
public struct WhatsAppLink: Decodable, Equatable, Sendable {
    /// `disconnected`, `starting`, `qr`, `pairing`, `connected`, `loggedOut` or `error`.
    public var state: String
    public var qr: String?
    /// The code to type in `WhatsApp` → Linked devices instead of scanning a QR code.
    public var pairCode: String?
    public var me: String?
    public var name: String?
    public var error: String?
    /// The bot whose chat receives this number's messages.
    public var bot: String?
    /// Chats and people allowed to write; empty means anyone.
    public var allowed: [String]
    /// People waiting for the owner's answer on a restricted number.
    public var requests: [WhatsAppRequest]
    /// People this number knows, with the facts they gave.
    public var people: [WhatsAppPerson]

    private enum CodingKeys: String, CodingKey { case state, qr, pairCode, me, name, error, bot, allowed, requests, people }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        state = try c.decodeIfPresent(String.self, forKey: .state) ?? "disconnected"
        qr = try c.decodeIfPresent(String.self, forKey: .qr)
        pairCode = try c.decodeIfPresent(String.self, forKey: .pairCode)
        me = try c.decodeIfPresent(String.self, forKey: .me)
        name = try c.decodeIfPresent(String.self, forKey: .name)
        error = try c.decodeIfPresent(String.self, forKey: .error)
        bot = try c.decodeIfPresent(String.self, forKey: .bot)
        allowed = try c.decodeIfPresent([String].self, forKey: .allowed) ?? []
        requests = try c.decodeIfPresent([WhatsAppRequest].self, forKey: .requests) ?? []
        people = try c.decodeIfPresent([WhatsAppPerson].self, forKey: .people) ?? []
    }
}

/// What the Channels screen shows, and the words it uses.
public enum Channels {
    /// A short name the host accepts for a Telegram bot.
    public static func nameOk(_ name: String) -> Bool {
        (1...32).contains(name.count) && name.allSatisfy { $0.isASCII && ($0.isLowercase || $0.isNumber || $0 == "-") }
    }

    /// `123456:ABC…`, the shape BotFather gives.
    public static func tokenOk(_ token: String) -> Bool {
        token.count <= 128 && token.wholeMatch(of: /\d+:[A-Za-z0-9_-]{30,}/) != nil
    }

    /// A Discord snowflake.
    public static func idOk(_ id: String) -> Bool { (1...32).contains(id.count) && id.allSatisfy(\.isNumber) }

    /// The three dot-separated parts of a bot token.
    public static func discordTokenOk(_ token: String) -> Bool {
        token.count <= 200 && token.wholeMatch(of: /[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]{20,}/) != nil
    }

    /// `github_pat_…` or a classic token: 20–255 letters, digits, `-` or `_`.
    public static func githubTokenOk(_ token: String) -> Bool {
        let value = token.trimmingCharacters(in: .whitespacesAndNewlines)
        return (20...255).contains(value.count) && value.allSatisfy { $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "_" || $0 == "-") }
    }

    /// GitHub's own repository-name rule, checked before the host is asked.
    public static func repoNameOk(_ name: String) -> Bool {
        let value = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let first = value.first, first.isASCII, first.isLetter || first.isNumber || first == "_" || first == "-" else { return false }
        return value.count <= 100 && value.allSatisfy { $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "_" || $0 == "-" || $0 == ".") }
    }

    public static func stateText(_ state: String) -> String {
        switch state {
        case "ok": "Connected"
        case "starting": "Starting…"
        default: "Cannot reach Telegram"
        }
    }

    public static func discordStateText(_ state: String) -> String {
        switch state {
        case "ok": "Connected"
        case "starting": "Connecting…"
        default: "Cannot reach Discord"
        }
    }

    /// A homeserver address: https, or http to a server on the computer's own network.
    public static func matrixHomeserverOk(_ value: String) -> Bool {
        let url = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return url.count <= 300 && url.wholeMatch(of: /https?:\/\/[^\s\/]+(\/[^\s]*)?/) != nil
    }

    /// A full Matrix user id: `@name:server`.
    public static func matrixUserOk(_ value: String) -> Bool {
        let id = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return id.count <= 255 && id.wholeMatch(of: /@[^:\s]+:[^\s]+/) != nil
    }

    /// A room id or alias: `!opaque:server` or `#alias:server`.
    public static func matrixRoomOk(_ value: String) -> Bool {
        let id = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return id.count <= 255 && id.wholeMatch(of: /[!#][^:\s]+:[^\s]+/) != nil
    }

    public static func matrixStateText(_ state: String) -> String {
        switch state {
        case "ok": "Connected"
        case "starting": "Connecting…"
        default: "Cannot reach Matrix"
        }
    }

    /// A Signal chat: an E.164 number or `group:<id>`.
    public static func signalChatOk(_ value: String) -> Bool {
        let chat = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return chat.count <= 512 && (chat.wholeMatch(of: /\+[1-9]\d{6,14}/) != nil
            || chat.wholeMatch(of: /group:[A-Za-z0-9+\/=_-]{8,512}/) != nil)
    }

    public static func signalNumberOk(_ value: String) -> Bool {
        value.trimmingCharacters(in: .whitespacesAndNewlines).wholeMatch(of: /\+[1-9]\d{6,14}/) != nil
    }

    public static func signalPersonOk(_ value: String) -> Bool {
        signalNumberOk(value) || value.trimmingCharacters(in: .whitespacesAndNewlines).wholeMatch(of: /[0-9a-fA-F-]{32,64}/) != nil
    }

    public static func signalStateText(_ state: String) -> String {
        switch state {
        case "ok": "Connected"
        case "starting": "Connecting…"
        default: state.hasPrefix("notRunning") ? "signal-cli is not running" : "Cannot reach signal-cli"
        }
    }

    public static func teamsStateText(_ state: String) -> String {
        switch state {
        case "ok": "Connected"
        case "starting": "Connecting…"
        default: "Not set up"
        }
    }

    public static func lastReceivedText(at: Int64?, now: Int64 = Int64(Date().timeIntervalSince1970 * 1000)) -> String {
        guard let at else { return "Nothing received yet" }
        let minutes = max(0, (now - at) / 60_000)
        return switch minutes {
        case ..<1: "Last message just now"
        case ..<60: "Last message \(minutes) min ago"
        case ..<(60 * 24): "Last message \(minutes / 60) h ago"
        default: "Last message \(minutes / (60 * 24)) d ago"
        }
    }

    /// Where to send the code, which works once for ten minutes.
    public static func pairingInstruction(code: String, username: String) -> String {
        let destination = username.isEmpty ? "your Telegram bot" : "@\(username)"
        return "Send /start \(code) to \(destination) in Telegram. The code works once, for 10 minutes."
    }

    public static func allowedText(_ count: Int) -> String {
        count == 1 ? "1 person allowed" : "\(count) people allowed"
    }
}

/// What the WhatsApp screen shows, and the words it uses.
public enum WhatsApp {
    public static func stateText(_ state: String) -> String {
        switch state {
        case "connected": "Linked"
        case "starting": "Starting…"
        case "qr": "Scan the code"
        case "pairing": "Enter the code on your phone."
        case "error": "The link stopped"
        default: "Not linked"
        }
    }

    /// The code in two groups of four, as WhatsApp shows its field for it.
    public static func groupedCode(_ code: String) -> String {
        code.count == 8 ? "\(code.prefix(4))-\(code.suffix(4))" : code
    }
}
