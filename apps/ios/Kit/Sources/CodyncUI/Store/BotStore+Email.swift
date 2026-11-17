import CodyncKit
import Foundation

/// An email account this computer sends and reads with; `configured` says its password is saved.
public struct EmailAccount: Decodable, Identifiable, Equatable, Sendable {
    public var id: String
    public var name: String
    public var address: String
    public var imapHost: String
    public var smtpHost: String
    public var configured: Bool
}

/// One message header from the inbox.
public struct EmailMessage: Decodable, Identifiable, Equatable, Sendable {
    public var uid: String
    public var from: String
    public var subject: String
    public var date: String

    public var id: String { uid }
}

extension BotStore {
    private struct InboxBody: Encodable {
        var account: String
        var limit: Int
    }
    private struct AccountsWrap: Decodable { var accounts: [EmailAccount] }
    private struct MessagesWrap: Decodable { var messages: [EmailMessage] }
    private struct AccountWrap: Decodable { var account: EmailAccount }
    private struct SentWrap: Decodable { var sent: Bool }

    public func emailAccounts() async throws -> [EmailAccount] {
        let client = try await ready()
        let wrap: AccountsWrap = try await client.call("emailAccounts")
        return wrap.accounts
    }

    @discardableResult
    public func saveEmailAccount(name: String, address: String, password: String, imapHost: String,
                                 smtpHost: String, username: String) async throws -> EmailAccount {
        let client = try await ready()
        let wrap: AccountWrap = try await client.call("saveEmailAccount", [
            "name": name,
            "address": address,
            "password": password,
            "imapHost": imapHost,
            "smtpHost": smtpHost,
            "username": username,
        ])
        return wrap.account
    }

    @discardableResult
    public func deleteEmailAccount(_ id: String) async throws -> Bool {
        struct Wrap: Decodable { var deleted: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("deleteEmailAccount", ["id": id])
        return wrap.deleted
    }

    public func emailInbox(account: String, limit: Int = 20) async throws -> [EmailMessage] {
        let client = try await ready()
        let wrap: MessagesWrap = try await client.call("emailInbox", InboxBody(account: account, limit: limit), timeout: 60)
        return wrap.messages
    }

    @discardableResult
    public func sendEmail(account: String, to: String, subject: String, body: String) async throws -> Bool {
        let client = try await ready()
        let wrap: SentWrap = try await client.call(
            "emailSend", ["account": account, "to": to, "subject": subject, "body": body], timeout: 60
        )
        return wrap.sent
    }
}
