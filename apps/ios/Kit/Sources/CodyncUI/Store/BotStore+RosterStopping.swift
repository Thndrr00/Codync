import CodyncKit
import Foundation

extension BotStore {
    public func stopFromRoster(_ bot: Bot) {
        guard !rosterStopping.contains(bot.id), let current = bots[bot.id], current.isWorking else { return }
        rosterStopping.insert(bot.id)
        Task {
            defer { rosterStopping.remove(bot.id) }
            do {
                let client = try await ready()
                guard let latest = bots[bot.id], latest.isWorking, latest.startedAt == current.startedAt else { return }
                try await client.stop(bot.id)
                let deadline = ContinuousClock.now + .seconds(20)
                while let updated = bots[bot.id], updated.isWorking, updated.startedAt == latest.startedAt {
                    guard ContinuousClock.now < deadline else {
                        lastError = "Stop was requested; confirmation is unavailable. Check the connection."
                        return
                    }
                    try await Task.sleep(for: .milliseconds(150))
                }
            } catch is CancellationError { }
            catch { lastError = error.localizedDescription }
        }
    }
}
