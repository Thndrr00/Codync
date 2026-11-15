import CodyncKit
import Foundation
import Observation

/// One bot's memory, read-only: the bot changes it. The brains it can work in are changed from here.
@MainActor @Observable
final class MemoryStore {
    let botId: String
    private let client: () async throws -> HostClient
    var view: MemoryView?
    var error: String?
    /// Every brain with the bots in it; nil until read.
    var brains: [Brain]?
    private var request = 0

    init(botId: String, client: @escaping () async throws -> HostClient) {
        self.botId = botId
        self.client = client
    }

    func load() async {
        request += 1
        let asked = request
        do {
            let value = try await client().memory(botId: botId)
            if asked == request {
                view = value
                error = nil
            }
        } catch {
            // A host before 2.18.0 answers with its old memory listing, which has no layers or dreams.
            if asked == request {
                self.error = error is DecodingError ? "Update Codync on your computer to see this bot's memory." : error.localizedDescription
            }
        }
    }

    /// A long-form file as it is now.
    func file(_ name: String, shared: Bool) async -> MemoryFileDetail? {
        do {
            return try await client().memoryFile(botId: botId, file: name, shared: shared)
        } catch {
            self.error = error.localizedDescription
            return nil
        }
    }

    /// Every brain, and the bot's memory after the change that opened this.
    func loadBrains() async {
        do {
            brains = try await client().brains()
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    /// Makes a brain; with `id`, renames that one.
    func saveBrain(_ name: String, id: String?) async {
        await change { try await $0.saveBrain(name: name, id: id) }
    }

    /// Moves the bot into a brain (the Fleet one is the shared memory).
    func joinBrain(_ brain: Brain) async {
        await change { try await $0.joinBrain(botId: botId, brainId: brain.id) }
    }

    /// Takes the bot out of its brain; it keeps only its own memory (picking one again brings sharing back).
    func leaveBrain() async {
        await change { try await $0.leaveBrain(botId: botId) }
    }

    /// Deletes a brain; its bots go back to the Fleet brain and what it held is gone.
    func deleteBrain(_ brain: Brain) async {
        await change { try await $0.deleteBrain(id: brain.id) }
    }

    /// A brain as a file to share, or the brain made from one of those files with how many of its files were turned
    /// away.
    func exportBrain(_ brain: Brain) async -> URL? {
        do {
            let bundle = try await client().exportBrain(id: brain.id)
            let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(brain.name).brain.json")
            try JSONEncoder().encode(bundle).write(to: url)
            return url
        } catch {
            self.error = error.localizedDescription
            return nil
        }
    }

    func importBrain(_ data: Data, name: String?) async -> Int? {
        do {
            let reply = try await client().importBrain(try JSONDecoder().decode(BrainBundle.self, from: data), name: name)
            brains = reply.brains
            await load()
            return reply.skipped
        } catch {
            self.error = error is DecodingError ? "That file isn't a Codync brain file." : error.localizedDescription
            return nil
        }
    }

    private func change(_ work: (HostClient) async throws -> [Brain]) async {
        do {
            brains = try await work(try await client())
            error = nil
            await load()
        } catch {
            self.error = error.localizedDescription
        }
    }
}
