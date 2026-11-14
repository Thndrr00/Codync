import CodyncKit

/// A command in the composer's `/` list.
struct SlashCommand: Identifiable, Equatable {
    enum Action: Hashable { case fast, skill(String), media(MediaKind) }

    /// What follows the `/`.
    let name: String
    let icon: String
    let title: String
    let detail: String
    let warning: Bool
    let action: Action

    var id: Action { action }
}

/// The tools every bot can be asked to work with: `/image …` and `/video …` open the picker and the
/// host writes the request (host/src/media); `/n8n …` and `/comfy …` only add a note
/// (host/src/chat/tool_commands.rs).
private let TOOL_COMMANDS: [(name: String, icon: String, title: String, detail: String, media: MediaKind?)] = [
    (name: "image", icon: "sparkles", title: "Image", detail: "Make an image with your agent's subscription", media: MediaKind.image),
    (name: "video", icon: "film", title: "Video", detail: "Make a short video with your agent's subscription", media: MediaKind.video),
    (name: "n8n", icon: "point.3.connected.trianglepath.dotted", title: "n8n", detail: "Do it with the computer's n8n workflows", media: nil),
    (name: "comfy", icon: "photo", title: "ComfyUI", detail: "Make an image with ComfyUI", media: nil),
]

/// The composer's `/` commands, listed while the draft is just `/partial`, the same in a group: Fast mode runs here
/// through the host's `updateBot` (a group's on each member; a new session is in the bot's menu); a skill fills in `/<id> `, and the host turns that message into "read this skill first"
/// (`host/src/market/skills.rs` `expand_command`); `/image` and `/video` open the picker. docs/features/marketplace.md,
/// "Commands", docs/features/media.md.
enum SlashCommands {
    /// The command name being typed after a leading `/`, before any space.
    static func query(_ draft: String) -> String? {
        guard draft.hasPrefix("/"), !draft.contains(where: \.isWhitespace) else { return nil }
        return String(draft.dropFirst())
    }

    /// Built-in commands, then the skills the bot has on (a group's: any member's), then the tool commands,
    /// whose name or title holds `query`. `members` are a group's bots.
    static func list(for bot: Bot?, members: [Bot] = [], query: String, skills: [InstalledSkill]) -> [SlashCommand] {
        guard let bot else { return [] }
        let owners = bot.isGroup ? members : [bot]
        let own = skills.filter { skill in owners.contains { $0.skills.contains(skill.id) } }.map {
            SlashCommand(name: $0.id, icon: "book.pages", title: $0.name, detail: $0.description, warning: false, action: .skill($0.id))
        }
        // A skill with the same id keeps the name: the host expands the skill first.
        let tools = TOOL_COMMANDS.filter { tool in
            !own.contains { $0.name == tool.name } && (tool.media == nil || bot.mediaOffered)
        }.map { tool in
            SlashCommand(name: tool.name, icon: tool.icon, title: tool.title, detail: tool.detail, warning: false,
                         action: tool.media.map(Action.media) ?? .skill(tool.name))
        }
        return (builtIn(bot, members: members) + own + tools).filter {
            query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) || $0.title.localizedCaseInsensitiveContains(query)
        }
    }

    /// A draft that is just a built-in command (`/fast`): sending it runs the command. A bare `/image` or
    /// `/video` opens the picker, unless a skill of that name is on (the host expands the skill first).
    static func builtIn(for bot: Bot?, members: [Bot] = [], draft: String) -> SlashCommand? {
        guard let bot, let name = query(draft.trimmingCharacters(in: .whitespacesAndNewlines)) else { return nil }
        if let picked = builtIn(bot, members: members).first(where: { $0.name == name }) { return picked }
        let owners = bot.isGroup ? members : [bot]
        guard !owners.contains(where: { $0.skills.contains(name) }), bot.mediaOffered,
              let tool = TOOL_COMMANDS.first(where: { $0.name == name && $0.media != nil }), let kind = tool.media
        else { return nil }
        return SlashCommand(name: tool.name, icon: tool.icon, title: tool.title, detail: tool.detail, warning: false, action: .media(kind))
    }

    /// Fast mode, while the bot (or a member of the group) shows it.
    private static func builtIn(_ bot: Bot, members: [Bot]) -> [SlashCommand] {
        let fast: (detail: String, state: Bot.FastModeState)
        if bot.isGroup {
            let group = GroupFastMode(members)
            guard !group.members.isEmpty else { return [] }
            fast = (group.detail, group.state)
        } else {
            guard bot.fastModeShown else { return [] }
            fast = (bot.fastModeDetail, bot.fastModeState)
        }
        return [SlashCommand(name: "fast", icon: "bolt", title: "Fast mode", detail: fast.detail,
                             warning: fast.state == .refused, action: .fast)]
    }
}

/// The `@partial` being typed at the end of a group's draft, if any. A member's full name and its space end it, so the
/// list doesn't stay up for a longer name that holds this one.
func mentionQuery(_ draft: String, members: [Bot]) -> String? {
    guard let at = draft.lastIndex(of: "@") else { return nil }
    if at > draft.startIndex, !draft[draft.index(before: at)].isWhitespace { return nil }
    let query = String(draft[draft.index(after: at)...])
    if query.first?.isWhitespace == true || query.contains(where: \.isNewline) || query.count > 24 { return nil }
    return members.contains { query.lowercased() == "\($0.name.lowercased()) " } ? nil : query
}
