import Foundation

/// Model identifiers and display names advertised by the selected computer's ACP agent.
public struct AgentModels: Decodable, Sendable {
    public struct Model: Decodable, Identifiable, Sendable {
        public let id: String
        public let name: String
        public let description: String?
        /// The model's own settings (effort, fast…); absent from hosts that don't list them.
        public let options: [Option]?
        public let optionsError: String?
    }

    /// A model setting and the values the agent accepts.
    public struct Option: Decodable, Identifiable, Sendable {
        public struct Value: Decodable, Sendable {
            public let id: String
            public let name: String
        }

        public let id: String
        public let name: String
        public let values: [Value]
        public let `default`: String?
        public let kind: String?
        public var isReasoning: Bool {
            kind == "reasoning" || ["effort", "reasoning_effort", "reasoning-effort", "thought_level"].contains(id)
        }
    }

    public let models: [Model]
    public let currentModelId: String?
    /// The agent offers Fast mode (absent on older hosts).
    public let fastMode: Bool?
    public let defaultModelOptions: Bool?
    /// The agent's modes, its Auto mode first and listed as `auto` (nil from older hosts).
    public let modes: [AgentMode]?
    /// Models turned off in the provider's Models settings (missing on older hosts).
    public let hidden: [String]

    enum CodingKeys: String, CodingKey {
        case models, currentModelId, fastMode, defaultModelOptions, modes, hidden
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        models = try c.decode([Model].self, forKey: .models)
        currentModelId = try c.decodeIfPresent(String.self, forKey: .currentModelId)
        fastMode = try c.decodeIfPresent(Bool.self, forKey: .fastMode)
        defaultModelOptions = try c.decodeIfPresent(Bool.self, forKey: .defaultModelOptions)
        modes = try c.decodeIfPresent([AgentMode].self, forKey: .modes)
        hidden = try c.decodeIfPresent([String].self, forKey: .hidden) ?? []
    }

    /// A model turned off in the provider's settings stays in this agent's picker
    /// only if this agent still uses it; a new agent (no `used`) never starts on it
    /// (OpenBot nightly triage-12582).
    public func visibleModels(used: String?) -> [Model] {
        models.filter { model in
            !hidden.contains(model.id) || (used.map { $0 == model.id } ?? false)
        }
    }
}
