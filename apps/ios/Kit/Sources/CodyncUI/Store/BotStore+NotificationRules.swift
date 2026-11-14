import CodyncKit
import Foundation

/// Which alerts reach this phone, and the quiet hours around them
/// (host/src/store/notification_rules.rs). An approval always arrives.
public struct AlertRules: Codable, Equatable, Sendable {
    public var done: Bool
    public var needsInput: Bool
    public var failed: Bool
    public var paused: Bool
    /// The quiet window as local hours; equal bounds mean no quiet hours.
    public var quietStart: Int
    public var quietEnd: Int
    /// The zone the quiet hours are read in; nil uses the computer's zone.
    public var timezone: String?

    public init(done: Bool = true, needsInput: Bool = true, failed: Bool = true, paused: Bool = true,
                quietStart: Int = 0, quietEnd: Int = 0, timezone: String? = nil) {
        self.done = done
        self.needsInput = needsInput
        self.failed = failed
        self.paused = paused
        self.quietStart = quietStart
        self.quietEnd = quietEnd
        self.timezone = timezone
    }

    /// Equal bounds: no quiet hours.
    public var hasQuietHours: Bool { quietStart != quietEnd }
}

extension BotStore {
    private struct RulesWrap: Decodable { var rules: AlertRules }

    public func alertRules() async throws -> AlertRules {
        let client = try await ready()
        let wrap: RulesWrap = try await client.call("notificationRules")
        return wrap.rules
    }

    /// The rules as saved; the computer keeps the old ones when it refuses them.
    @discardableResult
    public func setAlertRules(_ rules: AlertRules) async throws -> AlertRules {
        let client = try await ready()
        let wrap: RulesWrap = try await client.call("setNotificationRules", ["rules": rules])
        return wrap.rules
    }
}
