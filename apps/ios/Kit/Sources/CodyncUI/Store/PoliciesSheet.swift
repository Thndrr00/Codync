import CodyncKit
import SwiftUI

/// A policy rule on this computer (`docs/features/policy-engine.md`): a named CEL expression with an
/// effect (`allow` or `deny`) and a message, evaluated before any bot's approval rules.
struct PolicyRule: Codable, Identifiable, Sendable, Equatable {
    let id: String
    let name: String
    let expression: String
    /// `allow` or `deny`.
    let effect: String
    let message: String
    let enabled: Bool
}

/// A rule being written: no id yet adds it.
struct PolicyDraft: Encodable, Sendable {
    var id: String?
    var name: String
    var expression: String
    var effect: String
    var message: String
    var enabled: Bool

    init(_ rule: PolicyRule) {
        id = rule.id
        name = rule.name
        expression = rule.expression
        effect = rule.effect
        message = rule.message
        enabled = rule.enabled
    }

    init() {
        name = ""
        expression = ""
        effect = "deny"
        message = ""
        enabled = true
    }
}

/// A starting point the host offers; it is a rule only once it is saved.
struct PolicyPreset: Codable, Identifiable, Sendable {
    let id: String
    let name: String
    let about: String
    let expression: String
    let effect: String
    let message: String
}

struct PolicyVariableInfo: Codable, Sendable {
    let name: String
    let about: String
}

struct PolicyLimits: Codable, Sendable {
    let name: Int
    let expression: Int
    let message: Int
}

struct PoliciesReply: Codable, Sendable {
    let policies: [PolicyRule]
    let presets: [PolicyPreset]
    let variables: [PolicyVariableInfo]
    let limits: PolicyLimits
    /// Any rule is switched on.
    let enforcing: Bool
    /// An allow rule is saved: only what an allow rule names goes through.
    let allowlist: Bool
}

/// The rule that decided, as the host names it.
struct PolicyDecidingRule: Codable, Sendable {
    let name: String
    let effect: String
    let message: String?
}

/// What the policy list says about a proposed call (`previewPolicy`), and the variables it read.
struct PolicyPreviewReply: Codable, Sendable {
    let decision: String
    let source: String
    let rule: PolicyDecidingRule?
    let allowlist: Bool
    let refusal: String?
    let variables: [String: JSONValue]
    let explanation: String
}

extension BotStore {
    func policyList() async throws -> PoliciesReply {
        try await ready().call("policies")
    }

    @discardableResult
    func savePolicy(_ draft: PolicyDraft) async throws -> PolicyRule {
        struct Body: Encodable { var policy: PolicyDraft }
        struct Reply: Decodable, Sendable { var policy: PolicyRule }
        let reply: Reply = try await ready().call("savePolicy", Body(policy: draft))
        return reply.policy
    }

    func setPolicyEnabled(_ id: String, _ enabled: Bool) async throws {
        struct Body: Encodable { var id: String; var enabled: Bool }
        let _: JSONValue = try await ready().call("setPolicyEnabled", Body(id: id, enabled: enabled))
    }

    func removePolicy(_ id: String) async throws {
        struct Body: Encodable { var id: String }
        let _: JSONValue = try await ready().call("removePolicy", Body(id: id))
    }

    func previewPolicy(tool: String, command: String, initiator: String) async throws -> PolicyPreviewReply {
        struct Body: Encodable { var tool: String; var command: String?; var initiator: String }
        struct Reply: Decodable, Sendable { var preview: PolicyPreviewReply }
        let body = Body(tool: tool, command: command.isEmpty ? nil : command, initiator: initiator)
        let reply: Reply = try await ready().call("previewPolicy", body)
        return reply.preview
    }
}

/// This computer's policies: the list, its presets, and a what-if that never acts. The host compiles and
/// decides; this only words it.
public struct PoliciesSheet: View {
    @Environment(BotStore.self) private var store
    @State private var reply: PoliciesReply?
    @State private var rows: [PolicyRule] = []
    @State private var editing: PolicyEditorTarget?
    @State private var error: String?
    @State private var busy = false
    @State private var tool = "shell"
    @State private var command = ""
    @State private var initiator = "person"
    @State private var preview: PolicyPreviewReply?

    public init() {}

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Policies")
            CardForm {
                CardSection(
                    "Policies",
                    footer: "A deny rule refuses first. An allow rule permits, and once one is saved only what an allow rule names goes through; without policies the bots' approval rules and Approvals setting decide as before. A rule that cannot run refuses and says why."
                ) {
                    if rows.isEmpty {
                        Text(L10n.tr("No policies: start from a preset, or add a rule of your own."))
                            .font(.footnote)
                            .foregroundStyle(Palette.secondary)
                    }
                    ForEach(rows) { rule in
                        HStack(spacing: 10) {
                            Toggle(isOn: Binding(
                                get: { rule.enabled },
                                set: { on in Task { await toggle(rule, on) } }
                            )) { EmptyView() }
                                .toggleStyle(.codync)
                                .accessibilityLabel(L10n.tr("On"))
                            VStack(alignment: .leading, spacing: 2) {
                                Text(rule.name).lineLimit(1)
                                Text(rule.expression)
                                    .font(.caption.monospaced())
                                    .foregroundStyle(Palette.secondary)
                                    .lineLimit(2)
                            }
                            Spacer(minLength: 4)
                            Text(L10n.tr(rule.effect == "allow" ? "Allow" : "Deny"))
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(rule.effect == "allow" ? Palette.text : Palette.danger)
                            IconButton("Edit rule", systemImage: "pencil") {
                                editing = PolicyEditorTarget(draft: PolicyDraft(rule))
                            }
                            IconButton("Delete rule", systemImage: "trash") {
                                Task { await remove(rule) }
                            }
                        }
                    }
                    Button(L10n.tr("New rule")) {
                        editing = PolicyEditorTarget(draft: PolicyDraft())
                    }
                    .buttonStyle(.secondary)
                    ForEach(reply?.presets ?? []) { preset in
                        Button {
                            var draft = PolicyDraft()
                            draft.name = preset.name
                            draft.expression = preset.expression
                            draft.effect = preset.effect
                            draft.message = preset.message
                            editing = PolicyEditorTarget(draft: draft)
                        } label: {
                            Text(L10n.fill(L10n.tr("Start from “{name}”"), ["name": preset.name]))
                        }
                        .buttonStyle(.secondary)
                    }
                    Text(stateSentence).font(.footnote).foregroundStyle(Palette.secondary)
                }

                if reply != nil {
                    CardSection(
                        "What would happen?",
                        footer: "Answered by the same engine a real request goes through, with credentials masked. Nothing runs."
                    ) {
                        TextField(L10n.tr("Tool"), text: $tool)
                            .textFieldStyle(.plain)
                            .textInputAutocapitalization(.never)
                        TextField(L10n.tr("Command or input"), text: $command, axis: .vertical)
                            .textFieldStyle(.plain)
                            .lineLimit(1...3)
                            .textInputAutocapitalization(.never)
                        ChoicePicker(selection: $initiator, options: [
                            ("person", L10n.tr("A person")),
                            ("routine", L10n.tr("A routine")),
                            ("channel", L10n.tr("A channel")),
                            ("network", L10n.tr("Another computer")),
                        ])
                        Button(L10n.tr("Ask")) { Task { await ask() } }
                            .buttonStyle(.secondary)
                            .disabled(busy || tool.trimmingCharacters(in: .whitespaces).isEmpty)
                        if let preview {
                            Text(decisionSentence(preview)).font(.subheadline.weight(.semibold)).foregroundStyle(Palette.text)
                            Text(preview.explanation)
                                .font(.caption)
                                .foregroundStyle(Palette.secondary)
                                .textSelection(.enabled)
                            ForEach(variableRows(preview.variables)) { row in
                                HStack(alignment: .firstTextBaseline, spacing: 8) {
                                    Text(row.id).font(.caption.monospaced()).foregroundStyle(Palette.secondary)
                                    Text(row.value.isEmpty ? "—" : row.value).font(.caption).foregroundStyle(Palette.text).lineLimit(1)
                                }
                            }
                        }
                    }
                }

                if let reply {
                    CardSection("Variables a rule can use", footer: "Every value is a string; an empty one means the call has none.") {
                        ForEach(reply.variables, id: \.name) { variable in
                            ValueRow(variable.name, detail: L10n.tr(variable.about)) {
                                Text(variable.name).font(.caption.monospaced()).foregroundStyle(Palette.secondary)
                            }
                        }
                    }
                }

                if let error {
                    Text(error).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
                }
            }
        }
        .background(Palette.background)
        .task { await load() }
        .codyncSheet(item: $editing) { target in
            PolicyEditorSheet(draft: target.draft) { draft in
                try await save(draft)
                editing = nil
            }
        }
    }

    private var stateSentence: String {
        guard let reply else { return L10n.tr("Reading the policies…") }
        if !reply.enforcing { return L10n.tr("No policy is on: the bots' approval rules and Approvals setting decide everything, as before.") }
        return reply.allowlist
            ? L10n.tr("Policies are on. An allow rule is saved, so only what an allow rule names goes through.")
            : L10n.tr("Policies are on. What they do not refuse goes on to the bot's approval rules.")
    }

    private func decisionSentence(_ preview: PolicyPreviewReply) -> String {
        switch preview.decision {
        case "allow":
            return L10n.tr("Allowed")
        case "deny":
            return preview.rule.map { L10n.fill(L10n.tr("Refused by “{name}”"), ["name": $0.name]) } ?? L10n.tr("Refused")
        default:
            return L10n.tr("No policy decides: the bot's approval rules do")
        }
    }

    private func variableRows(_ snapshot: [String: JSONValue]) -> [PolicyVariableRow] {
        var rows: [PolicyVariableRow] = []
        for (group, value) in snapshot.sorted(by: { $0.key < $1.key }) {
            if case let .object(fields) = value {
                for (field, inner) in fields.sorted(by: { $0.key < $1.key }) {
                    rows.append(PolicyVariableRow(id: "\(group).\(field)", value: Self.text(inner)))
                }
            } else {
                rows.append(PolicyVariableRow(id: group, value: Self.text(value)))
            }
        }
        return rows
    }

    private static func text(_ value: JSONValue) -> String {
        switch value {
        case let .string(text): return text
        case let .number(number): return String(number)
        case let .bool(flag): return flag ? "true" : "false"
        case .null: return ""
        case let .array(items): return items.map(Self.text).joined(separator: ", ")
        case let .object(fields): return fields.map { "\($0.key): \(Self.text($0.value))" }.sorted().joined(separator: ", ")
        }
    }

    private func load() async {
        do {
            let answer = try await store.policyList()
            reply = answer
            rows = answer.policies
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func save(_ draft: PolicyDraft) async throws {
        let saved = try await store.savePolicy(draft)
        rows = (rows.filter { $0.id != saved.id } + [saved]).sorted { $0.name < $1.name }
        await load()
    }

    private func toggle(_ rule: PolicyRule, _ enabled: Bool) async {
        do {
            try await store.setPolicyEnabled(rule.id, enabled)
            await load()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func remove(_ rule: PolicyRule) async {
        do {
            try await store.removePolicy(rule.id)
            await load()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func ask() async {
        busy = true
        defer { busy = false }
        do {
            preview = try await store.previewPolicy(tool: tool.trimmingCharacters(in: .whitespaces), command: command, initiator: initiator)
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }
}

/// One rule's target in the editor sheet, so a new one and an existing one look the same.
struct PolicyEditorTarget: Identifiable {
    let id = UUID()
    let draft: PolicyDraft
}

/// One row of the what-if's variable snapshot.
struct PolicyVariableRow: Identifiable {
    let id: String
    let value: String
}

/// One rule: its name, its expression, its effect and what a person reads when it decides. Saving asks the
/// host, which compiles it and refuses what it can't run.
struct PolicyEditorSheet: View {
    @State var draft: PolicyDraft
    let save: (PolicyDraft) async throws -> Void
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(draft.id == nil ? "New policy" : "Edit policy")
            CardForm {
                CardSection(footer: "The expression is CEL and must be true or false. It reads tool.name, tool.kind, intent, command, file.path, bot.name, actor.role, initiator.kind, key and more.") {
                    TextField(L10n.tr("Name"), text: $draft.name)
                        .textFieldStyle(.plain)
                    TextField(L10n.tr("Expression"), text: $draft.expression, axis: .vertical)
                        .textFieldStyle(.plain)
                        .font(.body.monospaced())
                        .lineLimit(1...6)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    ChoicePicker(selection: $draft.effect, options: [
                        ("deny", L10n.tr("Deny")),
                        ("allow", L10n.tr("Allow")),
                    ])
                    TextField(L10n.tr("What a person reads when this rule decides"), text: $draft.message, axis: .vertical)
                        .textFieldStyle(.plain)
                        .lineLimit(1...3)
                    Button(busy ? L10n.tr("Saving…") : L10n.tr("Save")) { submit() }
                        .buttonStyle(.primary)
                        .disabled(busy || draft.expression.trimmingCharacters(in: .whitespaces).isEmpty)
                    if let error {
                        Text(error).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
                    }
                }
            }
        }
        .background(Palette.background)
    }

    private func submit() {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                try await save(draft)
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
