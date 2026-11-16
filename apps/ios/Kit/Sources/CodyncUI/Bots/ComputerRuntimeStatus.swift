import CodyncKit
import SwiftUI

/// What the computer panel says about where the bot runs (`computerPanel`'s `runtime`), for the Details page.
struct ComputerRuntimeDetail: View {
    let botId: String
    /// Where the bot is set to run; a change fetches the status again.
    let kind: String
    @Environment(BotStore.self) private var model
    @State private var status: ComputerRuntimeStatus?

    var body: some View {
        Group {
            if let status {
                Text(status.detail)
                    .font(.caption)
                    .foregroundStyle(Palette.secondary)
                if status.kind == "container" {
                    Text(status.gvisorActive ? "gVisor · \(status.image)" : status.image)
                        .font(.caption2)
                        .foregroundStyle(Palette.tertiary)
                }
            }
        }
        .task(id: kind) { await load() }
    }

    private func load() async {
        guard let client = model.client else { return }
        do {
            let reply: ComputerPanelRuntime = try await client.call("computerPanel", ["botId": botId])
            status = reply.runtime
        } catch {
            status = nil
        }
    }
}

private struct ComputerPanelRuntime: Decodable {
    let runtime: ComputerRuntimeStatus?
}

struct ComputerRuntimeStatus: Decodable {
    let kind: String
    let active: String
    let available: Bool
    let gvisor: Bool
    let gvisorActive: Bool
    let container: String?
    let running: Bool
    let image: String
    let workspace: String
    let browserProfile: String
    let detail: String
}
