import CodyncKit
import Foundation
import Observation

/// One bot's missions, read and moved through the host: the list, the create call, and every change a
/// person can make (a status, a checkpoint, a step).
@MainActor @Observable
final class MissionsStore {
    let botId: String
    private let client: () async throws -> HostClient
    var missions: [Mission] = []
    var showFinished = false
    var loading = false
    var busy = false
    var error: String?

    init(botId: String, client: @escaping () async throws -> HostClient) {
        self.botId = botId
        self.client = client
    }

    var open: [Mission] { missions.filter { !$0.status.isReturned } }
    var finished: [Mission] { missions.filter { $0.status.isReturned } }

    private struct Bot: Encodable { let botId: String }
    private struct List: Encodable { let botId: String; let open: Bool }
    private struct Listing: Decodable { let missions: [Mission] }
    private struct Created: Encodable { let botId: String; let title: String; let goal: String; let steps: [String] }
    private struct Move: Encodable {
        let id: String
        var status: MissionStatus?
        var stopReason: String?
        var checkpoint: String?
    }
    private struct StepMove: Encodable { let id: String; var stepId: String?; var status: MissionStepStatus?; var text: String? }
    private struct Answer: Decodable { let mission: Mission }

    /// Reads the list: open ones, and the finished ones when the page asks for them.
    func load() async {
        loading = true
        error = nil
        defer { loading = false }
        do {
            let open: Listing = try await client().call("missions", List(botId: botId, open: true))
            let all: Listing? = showFinished ? try await client().call("missions", Bot(botId: botId)) : nil
            var merged: [String: Mission] = [:]
            for mission in open.missions + (all?.missions ?? []) { merged[mission.id] = mission }
            missions = merged.values.sorted { $0.updatedAt > $1.updatedAt }
        } catch is CancellationError {} catch {
            self.error = error.localizedDescription
        }
    }

    /// Runs one change and reads the list again; a refusal shows as the page's error.
    private func run(_ work: () async throws -> Void) async {
        busy = true
        error = nil
        do {
            try await work()
            await load()
        } catch is CancellationError {} catch {
            self.error = error.localizedDescription
        }
        busy = false
    }

    func create(title: String, goal: String, steps: [String]) async {
        let title = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty else { return }
        await run {
            _ = try await client().call("createMission", Created(botId: botId, title: title, goal: goal, steps: steps)) as Answer
        }
    }

    func move(_ mission: Mission, to status: MissionStatus, stopReason: String? = nil) async {
        await run {
            _ = try await client().call("updateMission", Move(id: mission.id, status: status, stopReason: stopReason)) as Answer
        }
    }

    func checkpoint(_ mission: Mission, _ text: String) async {
        await run {
            _ = try await client().call("updateMission", Move(id: mission.id, checkpoint: text)) as Answer
        }
    }

    func addStep(_ mission: Mission, _ text: String) async {
        let text = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        await run {
            _ = try await client().call("missionStep", StepMove(id: mission.id, text: text)) as Answer
        }
    }

    func setStep(_ mission: Mission, _ step: MissionStep, to status: MissionStepStatus) async {
        await run {
            _ = try await client().call("missionStep", StepMove(id: mission.id, stepId: step.id, status: status)) as Answer
        }
    }
}
