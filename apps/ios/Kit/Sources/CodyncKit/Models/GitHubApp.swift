import Foundation

/// The computer's GitHub App (`githubAppStatus`): read-only on phones, since it is set up at the
/// computer. The key and the webhook secret never leave it. Hosts before the App answer "unknown method".
public struct GitHubAppStatus: Codable, Hashable, Sendable {
    public var configured: Bool = false
    public var appId: String?
    public var slug: String?
    public var installations: [Installation] = []
    public var repos: [RepoMap] = []

    /// One installation of the App and the repositories it may reach.
    public struct Installation: Codable, Hashable, Sendable, Identifiable {
        public var id: String
        public var account: String
        public var repos: [String] = []
    }

    /// A repository that reaches `bots`, on `branches` (none means all of them).
    public struct RepoMap: Codable, Hashable, Sendable {
        public var repo: String
        public var bots: [String] = []
        public var branches: [String] = []
    }
}
