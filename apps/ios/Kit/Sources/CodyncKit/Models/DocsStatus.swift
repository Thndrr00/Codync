import Foundation

/// What a bot's `codync-docs` index holds (`docsStatus`): the folder it reads, what is in it, and whether
/// a limit cut the walk short. Mirrors `host/src/docs/mod.rs`; a host before the feature answers
/// "unknown method" instead.
public struct DocsStatus: Decodable, Hashable, Sendable {
    /// Where the index reads on the computer: the folder its owner named, or the bot's working folder.
    public var folder: String
    /// Documentation and source files it holds.
    public var files: Int
    /// Their size together, in bytes.
    public var bytes: Int
    /// The walk reached its limit (4,000 files or 24 MiB, `host/src/docs/index.rs`), so some files are
    /// not searchable.
    public var truncated: Bool
}
