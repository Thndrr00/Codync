import CodyncKit
import SwiftUI

/// The picker behind `/image` and `/video`: the models this bot's agent can make files with, the
/// frame, and for a video its length, resolution and voice. It writes the command into the box, so
/// the person only describes what to make (host/src/media).
/// Ported from OpenBot 0.70.0 `WorkflowMediaModelPicker.tsx` and `WorkflowImageNode.tsx`.
struct MediaSheet: View {
    let botId: String
    /// The command picked in the menu.
    let asked: MediaKind
    /// The box's text, whose description is kept.
    let draft: String
    let use: (String) -> Void
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @State private var answer: MediaAnswer?
    @State private var kind: MediaKind
    @State private var choice: MediaChoice?
    @State private var failed = false

    init(botId: String, kind: MediaKind, draft: String, use: @escaping (String) -> Void) {
        self.botId = botId
        self.asked = kind
        self.draft = draft
        self.use = use
        _kind = State(initialValue: kind)
    }

    private var models: [MediaModel] { answer.models(of: kind) }
    private var chosen: MediaModel? { models.first { $0.id == choice?.model } }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(kind.title) {
                IconButton("Use", systemImage: "checkmark", action: used)
                    .disabled(choice == nil)
            }
            CardForm {
                CardSection {
                    if let agent = answer?.agent {
                        Text(L10n.tr("Made by {agent}, paid with its subscription").replacingOccurrences(of: "{agent}", with: agent))
                            .font(.footnote)
                            .foregroundStyle(Palette.secondary)
                    }
                    SegmentedChoice(selection: $kind, options: MediaKind.allCases.map { (id: $0, label: $0.title) })
                        .onChange(of: kind) { _, next in choice = answer.firstChoice(next) }
                    content
                    if let choice, let chosen { MediaSettings(choice: choice, model: chosen) { choice = $0 } }
                    if let choice {
                        Text(mediaDraft(choice, draft: draft))
                            .font(.caption)
                            .monospaced()
                            .foregroundStyle(Palette.secondary)
                            .lineLimit(3)
                            .textSelection(.enabled)
                    }
                }
            }
        }
        .background(Palette.background)
        // Every opening asks again: another agent account may answer, and a bot's agent can change.
        .task(id: asked) {
            answer = nil
            failed = false
            choice = nil
            do {
                let loaded = try await model.mediaModels(botId)
                if Task.isCancelled { return }
                answer = loaded
                choice = loaded.firstChoice(asked)
            } catch is CancellationError {
            } catch {
                failed = true
            }
        }
    }

    @ViewBuilder private var content: some View {
        if failed {
            Text(L10n.tr("This computer's host is not answering.")).foregroundStyle(Palette.warning)
        } else if answer == nil {
            HStack(spacing: 8) {
                Spinner(size: 16)
                Text(L10n.tr("Checking what this bot can make…")).foregroundStyle(Palette.secondary)
            }
        } else if answer?.agent == nil {
            Text(L10n.tr("This bot’s agent has no image tool. Pick one that does, or use ComfyUI."))
                .foregroundStyle(Palette.secondary)
        } else if models.isEmpty {
            Text(L10n.tr("This bot’s agent makes no {kind}.").replacingOccurrences(of: "{kind}", with: L10n.tr(kind.title).lowercased()))
                .foregroundStyle(Palette.secondary)
        } else {
            if answer?.installed == false {
                Text(L10n.tr("This bot’s agent is not installed on this computer yet."))
                    .font(.footnote)
                    .foregroundStyle(Palette.warning)
            }
            if models.count > 1 {
                ChoiceList(selection: Binding(get: { choice?.model ?? "" }, set: { id in
                    guard let next = models.first(where: { $0.id == id }), let choice else { return }
                    self.choice = choice.on(next)
                }), options: models.map { (id: $0.id, label: $0.name, detail: mediaDetail($0, installed: answer?.installed != false)) })
            } else if let chosen {
                Text(mediaDetail(chosen, installed: answer?.installed != false))
                    .font(.footnote)
                    .foregroundStyle(Palette.secondary)
            }
        }
    }

    private func used() {
        guard let choice else { return }
        use(mediaDraft(choice, draft: draft))
        dismiss()
    }
}

/// A frame, and for a video its length, resolution and voice; only what the model makes.
private struct MediaSettings: View {
    let choice: MediaChoice
    let model: MediaModel
    let change: (MediaChoice) -> Void

    var body: some View {
        let caps = model.capabilities
        VStack(alignment: .leading, spacing: 8) {
            VStack(alignment: .leading, spacing: 6) {
                Text(L10n.tr("Aspect ratio")).font(.footnote).foregroundStyle(Palette.secondary)
                TabChoice(selection: Binding(get: { choice.aspect }, set: { aspect in
                    var next = choice
                    next.aspect = aspect
                    change(next)
                }), options: caps.aspects.map { (id: $0, label: $0) })
            }
            if choice.kind == .video, !caps.durations.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text(L10n.tr("Length")).font(.footnote).foregroundStyle(Palette.secondary)
                    TabChoice(selection: Binding(get: { choice.duration }, set: { duration in
                        var next = choice
                        next.duration = duration
                        change(next)
                    }), options: caps.durations.map { (id: $0, label: "\($0)s") })
                }
            }
            if choice.kind == .video, !caps.resolutions.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text(L10n.tr("Resolution")).font(.footnote).foregroundStyle(Palette.secondary)
                    TabChoice(selection: Binding(get: { choice.resolution }, set: { resolution in
                        var next = choice
                        next.resolution = resolution
                        change(next)
                    }), options: caps.resolutions.map { (id: $0, label: $0) })
                }
            }
            if choice.kind == .video, let voices = caps.voices {
                VStack(alignment: .leading, spacing: 6) {
                    Text(L10n.tr("Voice")).font(.footnote).foregroundStyle(Palette.secondary)
                    TabChoice(selection: Binding(get: { choice.voice ?? "" }, set: { voice in
                        var next = choice
                        next.voice = voice.isEmpty ? nil : voice
                        change(next)
                    }), options: [(id: "", label: L10n.tr("None"))] + voices.map { (id: $0.id, label: $0.name) })
                }
            }
            if choice.kind == .image, caps.transparent {
                Toggle(L10n.tr("Transparent background"), isOn: Binding(get: { choice.transparent }, set: { on in
                    var next = choice
                    next.transparent = on
                    change(next)
                })).toggleStyle(.codync)
            }
        }
    }
}
