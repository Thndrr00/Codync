import CodyncKit
import Foundation

/// One encrypted backup already in the person's WebDAV folder.
public struct CloudBackupFile: Decodable, Identifiable, Equatable, Sendable {
    public var name: String
    public var at: Int

    public var id: String { name }
}

/// Where this computer puts its encrypted backups and how the last one went.
public struct CloudBackupStatus: Decodable, Equatable, Sendable {
    public var url: String
    public var username: String
    public var backend: String
    public var rclone: Bool
    public var enabled: Bool
    public var keep: Int
    public var lastAt: Int?
    public var lastError: String?
    public var configured: Bool
    public var hasPassword: Bool
    public var files: [CloudBackupFile]
}

public struct CloudBackupUpload: Decodable, Sendable {
    public var name: String
    public var bytes: Int
    public var at: Int
}

extension BotStore {
    private struct SaveBody: Encodable {
        var url: String
        var username: String
        var password: String
        var backend: String
        var enabled: Bool
        var keep: Int
    }

    public func cloudBackupStatus() async throws -> CloudBackupStatus {
        let client = try await ready()
        return try await client.call("cloudBackupStatus")
    }

    @discardableResult
    public func saveCloudBackup(url: String, username: String, password: String, backend: String, enabled: Bool, keep: Int) async throws -> CloudBackupStatus {
        let client = try await ready()
        return try await client.call(
            "saveCloudBackup",
            SaveBody(url: url, username: username, password: password, backend: backend, enabled: enabled, keep: keep),
            timeout: 120
        )
    }

    @discardableResult
    public func runCloudBackup() async throws -> CloudBackupUpload {
        let client = try await ready()
        return try await client.call("runCloudBackup", Empty(), timeout: 300)
    }
}
