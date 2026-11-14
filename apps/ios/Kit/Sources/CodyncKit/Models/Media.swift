import Foundation

/// What the picker behind `/image` and `/video` makes (host/src/media).
/// Ported from OpenBot 0.70.0 `packages/ui/src/features/workflows/WorkflowMediaModelPicker.tsx`.
public enum MediaKind: String, Codable, Sendable, CaseIterable, Identifiable {
    case image, video

    public var id: String { rawValue }

    /// The sheet's title and the noun of its sentences.
    public var title: String { self == .image ? "Image" : "Video" }
}

/// A model a bot's agent makes files with: the host's `mediaModels` lists what the agent can do,
/// the picker turns a choice into the `/image …` or `/video …` message that carries it, and the
/// host writes the request.
public struct MediaModel: Decodable, Hashable, Sendable, Identifiable {
    public let id: String
    public let name: String
    /// The company that makes the model.
    public let maker: String
    /// The subscription that pays for the file.
    public let subscription: String
    /// What the model makes.
    public let kinds: [MediaKind]
    public let aspects: [String]
    /// An image of this model can have a clear background.
    public let transparent: Bool
    /// A video of this model can speak in a preset voice.
    public let voices: Bool
    public let capabilities: MediaCapabilities
}

public struct MediaCapabilities: Decodable, Hashable, Sendable {
    public let aspects: [String]
    public let durations: [Int]
    public let resolutions: [String]
    public let transparent: Bool
    /// Nil for a model whose videos don't speak.
    public let voices: [MediaVoice]?
}

public struct MediaVoice: Decodable, Hashable, Sendable, Identifiable {
    public let id: String
    public let name: String
}

/// The host's answer to `mediaModels`: a nil `agent` when the bot's agent has no image tool.
public struct MediaAnswer: Decodable, Hashable, Sendable {
    public let agent: String?
    public let installed: Bool
    public let models: [MediaModel]
}

/// What the picker chose; the flags of the message follow it.
public struct MediaChoice: Hashable, Sendable {
    public var kind: MediaKind
    public var model: String
    public var aspect: String
    /// Seconds; only a video has one.
    public var duration: Int
    /// `480p`, `720p`…; only a video has one.
    public var resolution: String
    public var transparent: Bool
    public var voice: String?
}

/// The flags and their values, in the order the host reads them.
private let VALUED = ["--model", "--aspect", "--duration", "--resolution", "--voice", "--first", "--last", "--ref"]

public extension Optional where Wrapped == MediaAnswer {
    /// The models of a kind, the first one chosen by default.
    func models(of kind: MediaKind) -> [MediaModel] {
        self?.models.filter { $0.kinds.contains(kind) } ?? []
    }

    /// The choice the picker starts on: the kind's first model at its first frame, length and resolution.
    func firstChoice(_ kind: MediaKind) -> MediaChoice? {
        guard let model = models(of: kind).first else { return nil }
        return MediaChoice(kind: kind, model: model.id, aspect: model.capabilities.aspects.first ?? "1:1",
                           duration: model.capabilities.durations.first ?? 0,
                           resolution: model.capabilities.resolutions.first ?? "", transparent: false, voice: nil)
    }
}

public extension MediaChoice {
    /// The same choice on another model, keeping only the settings the new one makes.
    func on(_ model: MediaModel) -> MediaChoice {
        var next = self
        let caps = model.capabilities
        next.model = model.id
        next.aspect = caps.aspects.contains(aspect) ? aspect : caps.aspects.first ?? "1:1"
        next.duration = caps.durations.contains(duration) ? duration : caps.durations.first ?? 0
        next.resolution = caps.resolutions.contains(resolution) ? resolution : caps.resolutions.first ?? ""
        next.voice = voice.flatMap { id in caps.voices?.contains { $0.id == id } == true ? id : nil }
        next.transparent = transparent && caps.transparent
        return next
    }
}

/// The words of `draft` that are the description: after the command and its flags.
public func mediaPrompt(_ draft: String) -> String {
    var words = draft.split(whereSeparator: \.isWhitespace).map(String.init)
    if words.first == "/image" || words.first == "/video" { words.removeFirst() }
    var index = 0
    while index < words.count {
        if words[index] == "--transparent" {
            index += 1
        } else if VALUED.contains(words[index]) {
            index += 2
        } else {
            break
        }
    }
    return words.dropFirst(index).joined(separator: " ")
}

/// The chat message that asks for `choice`, keeping the description already typed in `draft`.
public func mediaDraft(_ choice: MediaChoice, draft: String) -> String {
    var flags = ["--model \(choice.model)", "--aspect \(choice.aspect)"]
    if choice.kind == .video {
        flags.append("--duration \(choice.duration)")
        flags.append("--resolution \(choice.resolution)")
        if let voice = choice.voice { flags.append("--voice \(voice)") }
    } else if choice.transparent {
        flags.append("--transparent")
    }
    let prompt = mediaPrompt(draft)
    let command = "/\(choice.kind.rawValue) \(flags.joined(separator: " "))"
    return prompt.isEmpty ? "\(command) " : "\(command) \(prompt)"
}

/// The line under a model: who makes it and what pays for it, and when its agent isn't installed.
public func mediaDetail(_ model: MediaModel, installed: Bool) -> String {
    let base = "\(model.maker) · \(model.subscription)"
    return installed ? base : "\(base) · \(L10n.tr("not installed"))"
}

public extension Bot {
    /// Whether the picker is offered: a group has no agent of its own.
    var mediaOffered: Bool { !isGroup }
}
