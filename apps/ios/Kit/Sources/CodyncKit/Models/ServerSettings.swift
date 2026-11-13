import Foundation

/// Settings that belong to a computer rather than to one bot (`serverSettings`): how its agents post
/// updates between steps, whether Claude agents load the person's own Claude Code setup, and who may
/// control it over remote desktop.
public struct ServerSettings: Codable, Sendable, Equatable {
    public enum ResponseStyle: String, Codable, Sendable, CaseIterable {
        /// A short note only when a finding changes the plan or before a long step, then a verdict-first answer.
        case natural
        /// The agent's own text as it comes.
        case classic
    }

    public enum RemoteAccess: String, Codable, Sendable, CaseIterable {
        /// Every device in the account, which is every paired phone today.
        case everyone
        /// Owners and admins; Codync has one owner and no member roles yet, so this reads as `everyone`.
        case admins
        /// The owner only: this computer's own screen, never a paired phone.
        case owner
    }

    public struct RemoteControl: Codable, Sendable, Equatable {
        public var access: RemoteAccess
        public var askOnHost: Bool

        public init(access: RemoteAccess = .everyone, askOnHost: Bool = false) {
            self.access = access
            self.askOnHost = askOnHost
        }
    }

    public var responseStyle: ResponseStyle
    public var claudeUserSetup: Bool
    public var remoteControl: RemoteControl

    public init(responseStyle: ResponseStyle = .natural, claudeUserSetup: Bool = false,
                remoteControl: RemoteControl = RemoteControl()) {
        self.responseStyle = responseStyle
        self.claudeUserSetup = claudeUserSetup
        self.remoteControl = remoteControl
    }
}
