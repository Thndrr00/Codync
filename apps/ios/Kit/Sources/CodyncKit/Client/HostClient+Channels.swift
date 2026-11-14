import Foundation

private extension String {
    var trimmed: String { trimmingCharacters(in: .whitespacesAndNewlines) }
}

// The messaging channels of a computer and its linked WhatsApp number. The calls and their
// answers live in the host (`telegramStatus`, `slackStatus`, `whatsappState`, …); these methods
// only carry them.

public extension HostClient {
    /// A channel sub-task: stop it, let it go on, give it another member, or cancel its branch.
    func groupTask(_ botId: String, taskId: String, action: String, ownerId: String? = nil) async throws {
        struct Body: Encodable {
            var botId: String
            var taskId: String
            var action: String
            var ownerId: String?
        }
        let _: Empty = try await call("groupTask", Body(botId: botId, taskId: taskId, action: action, ownerId: ownerId))
    }

    struct TelegramBody: Encodable {
        var name: String
        var userId: Int64?
        var allowed: Bool?
        var token: String?
        var botId: String?
    }

    /// The computer's Telegram bots. A host older than them answers without `bots`.
    func telegramBots() async throws -> [TelegramBot] {
        let status: TelegramStatus = try await call("telegramStatus", Empty())
        return status.bots
    }

    /// A one-time code to send to the Telegram bot `name`; nil from an older host.
    func pairTelegram(name: String) async throws -> String? {
        let status: TelegramStatus = try await call("pairTelegram", TelegramBody(name: name))
        return status.result?.code
    }

    /// Lets a Telegram user write to the bot `name`, or takes them out; the bots as they are then.
    func setTelegramAllowed(name: String, userId: Int64, allowed: Bool) async throws -> [TelegramBot] {
        let status: TelegramStatus = try await call("setTelegramAllowed",
                                                    TelegramBody(name: name, userId: userId, allowed: allowed))
        return status.bots
    }

    func slackStatus() async throws -> SlackStatus { try await call("slackStatus", Empty()) }

    /// Forgets the Slack connection, workspace and bot token included.
    func disconnectSlack() async throws -> SlackStatus { try await call("disconnectSlack", Empty()) }

    func discordStatus() async throws -> DiscordStatus { try await call("discordStatus", Empty()) }

    func disconnectDiscord() async throws -> DiscordStatus { try await call("disconnectDiscord", Empty()) }

    func matrixStatus() async throws -> MatrixStatus { try await call("matrixStatus", Empty()) }

    func disconnectMatrix() async throws -> MatrixStatus { try await call("disconnectMatrix", Empty()) }

    /// Sets the Matrix session and its bound rooms up; the host signs in and checks every room first.
    func setMatrix(homeserver: String, user: String, accessToken: String, password: String,
                   bindings: [MatrixBinding]) async throws -> MatrixStatus {
        struct Body: Encodable {
            var homeserver: String
            var user: String
            var accessToken: String
            var password: String
            var bindings: [Binding]
        }
        struct Binding: Encodable {
            var roomId: String
            var botId: String
            var allowedUsers: [String]
        }
        let body = Body(homeserver: homeserver.trimmed, user: user.trimmed, accessToken: accessToken.trimmed,
                        password: password,
                        bindings: bindings.map { Binding(roomId: $0.roomId.trimmed, botId: $0.botId,
                                                        allowedUsers: $0.allowedUsers.map(\.trimmed)) })
        return try await call("setMatrixChannel", body, timeout: 60)
    }

    /// Sets the Discord bot and its bound channels up; the host checks the token before saving anything.
    func setDiscord(guildId: String, botUserId: String, botToken: String,
                    bindings: [DiscordBinding]) async throws -> DiscordStatus {
        struct Body: Encodable {
            var guildId: String
            var botUserId: String
            var botToken: String
            var bindings: [Binding]
        }
        struct Binding: Encodable {
            var channelId: String
            var botId: String
            var allowedUsers: [String]
        }
        let body = Body(guildId: guildId.trimmed, botUserId: botUserId.trimmed, botToken: botToken.trimmed,
                        bindings: bindings.map { Binding(channelId: $0.channelId.trimmed, botId: $0.botId,
                                                        allowedUsers: $0.allowedUsers.map(\.trimmed)) })
        return try await call("setDiscordChannel", body)
    }

    /// Adds a Telegram bot the host will talk through, answered by `botId`; its token stays on the computer.
    func addTelegramBot(name: String, token: String, botId: String) async throws -> [TelegramBot] {
        let status: TelegramStatus = try await call("addTelegramBot",
                                                    TelegramBody(name: name.trimmed, token: token.trimmed, botId: botId))
        return status.bots
    }

    /// Forgets a Telegram bot and its token.
    func removeTelegramBot(name: String) async throws -> [TelegramBot] {
        let status: TelegramStatus = try await call("removeTelegramBot", TelegramBody(name: name))
        return status.bots
    }

    func githubStatus() async throws -> GitHubStatus { try await call("githubStatus", Empty()) }

    func disconnectGitHub() async throws -> GitHubStatus { try await call("disconnectGitHub", Empty()) }

    /// Gives the host a GitHub token; it answers with the account that token belongs to.
    func setGitHubToken(_ token: String) async throws -> GitHubStatus {
        struct Body: Encodable { var token: String }
        return try await call("setGitHubToken", Body(token: token.trimmed))
    }

    /// Creates a repository on that account; the answer carries `created` plus the fresh status.
    func githubCreateRepo(name: String, isPrivate: Bool, description: String) async throws -> GitHubStatus {
        struct Body: Encodable { var name: String; var `private`: Bool; var description: String }
        return try await call("githubCreateRepo", Body(name: name.trimmed, private: isPrivate, description: description.trimmed))
    }

    func whatsappState() async throws -> WhatsAppLink { try await call("whatsappState", Empty()) }

    /// Starts the link; the host reports the QR code through `whatsappState`.
    func whatsappConnect() async throws {
        let _: [String: Bool] = try await call("whatsappConnect", Empty())
    }

    /// Links `phone` with a pairing code to type in WhatsApp instead of scanning a QR code.
    func whatsappPair(phone: String) async throws -> WhatsAppLink {
        struct Body: Encodable { var phone: String }
        return try await call("whatsappPair", Body(phone: phone))
    }

    /// Forgets the number on this computer.
    func whatsappLogout() async throws {
        let _: [String: Bool] = try await call("whatsappLogout", Empty())
    }

    /// Sends the number's messages to `botId`, or to none when nil.
    func whatsappSetBot(botId: String?) async throws {
        struct Body: Encodable {
            var botId: String?
            // `JSONEncoder` leaves a nil optional out; the host wants the key, empty for no bot.
            private enum CodingKeys: String, CodingKey { case botId }
            func encode(to encoder: Encoder) throws {
                var c = encoder.container(keyedBy: CodingKeys.self)
                try c.encode(botId ?? "", forKey: .botId)
            }
        }
        let _: [String: Bool] = try await call("whatsappSetBot", Body(botId: botId))
    }

    /// Lets a chat or person write, or takes them out. Returns the list as it is then.
    func whatsappSetAllowed(jid: String, allowed: Bool) async throws -> [String] {
        struct Body: Encodable { var jid: String; var allowed: Bool }
        struct Answer: Decodable { var allowed: [String] }
        let answer: Answer = try await call("whatsappSetAllowed", Body(jid: jid, allowed: allowed))
        return answer.allowed
    }

    /// Approves or denies a person who asked to write; approved, they join the allowed chats.
    func whatsappAnswerRequest(code: String, approve: Bool) async throws -> [WhatsAppRequest] {
        struct Body: Encodable { var code: String; var approve: Bool }
        struct Answer: Decodable { var requests: [WhatsAppRequest] }
        let answer: Answer = try await call("whatsappAnswerRequest", Body(code: code, approve: approve))
        return answer.requests
    }

    // MARK: conversation links (card K13)

    struct ConversationLinkBody: Encodable {
        var botId: String
        var provider: String
        var channel: String
        var thread: String?
        var threadId: String?
    }

    /// The conversations of a bot linked to a Codync thread.
    func conversationLinks(botId: String) async throws -> [ConversationLink] {
        struct Body: Encodable { var botId: String }
        struct Answer: Decodable { var links: [ConversationLink] }
        let answer: Answer = try await call("conversationLinks", Body(botId: botId))
        return answer.links
    }

    /// Links a channel conversation to `threadId`; every link of the bot comes back.
    func linkConversation(botId: String, provider: String, channel: String, thread: String, threadId: String) async throws -> [ConversationLink] {
        struct Answer: Decodable { var links: [ConversationLink] }
        let body = ConversationLinkBody(botId: botId, provider: provider, channel: channel, thread: thread, threadId: threadId)
        let answer: Answer = try await call("linkConversation", body)
        return answer.links
    }

    /// Takes a channel conversation's link away; its next message starts a conversation of its own.
    func unlinkConversation(botId: String, provider: String, channel: String, thread: String) async throws -> [ConversationLink] {
        struct Answer: Decodable { var links: [ConversationLink] }
        let body = ConversationLinkBody(botId: botId, provider: provider, channel: channel, thread: thread)
        let answer: Answer = try await call("unlinkConversation", body)
        return answer.links
    }
}
