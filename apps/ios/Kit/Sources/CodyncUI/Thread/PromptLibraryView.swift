import SwiftUI

/// A starting point for a message: a short, complete instruction with the blanks marked in square
/// brackets. Picking one fills the message box, where it can be edited before sending; nothing here
/// runs on its own. Same list as the desktop's prompt library.
public struct PromptLibraryItem: Identifiable, Sendable {
    public let title: String
    public let detail: String
    public let icon: String
    public let text: String
    public var id: String { title }

    public static let all: [PromptLibraryItem] = [
        item("Summarize this", "A short summary, the key points, and what to do next.", "doc.plaintext",
             "Summarize the following, in this order: one sentence saying what it is, the key points as a short list, and any decision or next step it implies. Say what is missing if something needed is not there.\n\n[paste the text or attach the file]"),
        item("Plan a project", "Goal, steps, order, risks and what done means.", "list.bullet.rectangle",
             "Plan this project: [what it is]. Give me the goal in one sentence, the steps in the order they should happen with the result of each, the risks with how I would notice them, and a clear test for what \"done\" means. Ask me one question first only if the answer would change the plan."),
        item("Review a change", "Find what breaks, with the smallest fix for each.", "checklist",
             "Review this change as a careful reviewer. For each problem, give the trigger (an input or state), the wrong result, and the smallest fix. Separate blockers from nits. If you cannot name a trigger, leave it out.\n\n[paste the diff or point me at the files]"),
        item("Find the risks", "What can go wrong, how I would see it, and what to do.", "exclamationmark.triangle",
             "List the risks in [the plan, change or situation]. For each one: what would trigger it, what the consequence is, how I would notice it early, and the cheapest response. Rank them by how much they would hurt, not by how likely they feel."),
        item("Notes into actions", "Decisions, owners, dates and open questions.", "checkmark.circle",
             "Turn these notes into decisions, actions and open questions. Each action gets an owner, a date if the notes give one, and the line it came from. Do not invent an owner or a date; write \"no owner\" or \"no date\" instead.\n\n[paste the notes]"),
        item("Draft an email", "One message with the reason, the ask and the next step.", "square.and.pencil",
             "Draft an email to [who] about [what]. Put the reason for writing in the first sentence, then what I need from them as one clear ask, then what happens next. Keep it under 150 words and match a plain, direct voice. Do not invent facts, prices or promises."),
        item("Compare two options", "Criteria from my decision, not from the products.", "arrow.left.arrow.right",
             "Compare [option A] and [option B] for [the decision I am making]. First list the criteria that matter for this decision and mark which are must-haves. Then fill a small table with a source or a reason per cell. End with which one fits and what would change the answer."),
        item("Debug this", "Reproduce, find the first wrong step, then the smallest fix.", "wrench.and.screwdriver",
             "Help me debug this. Here is what I expected, what happened, and the exact error:\n\n[expected]\n[actual]\n[error text]\n\nReproduce it first, then find the first point where the real state differs from the expected one. Give me at most three hypotheses with the check that would rule each out, and the smallest fix once we know which one it is."),
        item("Explain it simply", "The idea in plain words, then the detail that matters.", "sparkles",
             "Explain [the idea] in plain words first, in two or three sentences, as if I know the field but not this part. Then give the detail that changes how I would use it, and one example. Name anything you are unsure about instead of guessing."),
        item("Extract a table", "Rows and columns from text or a file, with the blanks marked.", "square.grid.3x3",
             "Extract this into a table with the columns [columns]. One row per [what a row is]. Keep values exactly as written, and write \"missing\" where a value is not there. Tell me which rows you were unsure about and why.\n\n[paste the text or attach the file]"),
        item("Ask me questions first", "The few questions that would change the answer.", "info.circle",
             "Before you start on [the task], ask me the few questions whose answers would change what you do. At most five, one line each, with the options you expect. If none would change it, say so and begin."),
        item("Check my writing", "Meaning kept, filler gone, unsupported claims flagged.", "pencil",
             "Edit this text without changing what it means. Cut filler and stock phrases, shorten long sentences, and make the actor clear. List every fact, number and claim before and after so none disappear, and flag any claim that needs a source instead of inventing one.\n\n[paste the text]"),
    ]

    private static func item(_ title: String, _ detail: String, _ icon: String, _ text: String) -> Self {
        Self(title: title, detail: detail, icon: icon, text: text)
    }
}

/// The prompt library: pick one to fill the message box, or save what the box holds now for later.
public struct PromptLibraryView: View {
    private let draft: String
    private let pick: (String) -> Void
    @Environment(\.dismissModal) private var dismiss
    @State private var query = ""
    @AppStorage("codyncPromptLibrary") private var saved = "[]"

    private struct Saved: Codable, Identifiable {
        var title: String
        var text: String
        var id: String { text }
    }

    public init(draft: String, pick: @escaping (String) -> Void) {
        self.draft = draft
        self.pick = pick
    }

    private var savedPrompts: [Saved] {
        (try? JSONDecoder().decode([Saved].self, from: Data(saved.utf8))) ?? []
    }

    private var queryText: String { query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() }

    private var shownSaved: [Saved] {
        savedPrompts.filter { queryText.isEmpty || $0.title.lowercased().contains(queryText) || $0.text.lowercased().contains(queryText) }
    }

    private var shown: [PromptLibraryItem] {
        PromptLibraryItem.all.filter {
            queryText.isEmpty || $0.title.lowercased().contains(queryText) || $0.detail.lowercased().contains(queryText)
        }
    }

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Prompt library")
            CardForm {
                SearchField("Search prompts", text: $query)
                CardSection(footer: "Saved on this iPhone. Picking a prompt fills the message box; edit it before sending.") {
                    Button("Save what the message box holds") { save() }
                        .buttonStyle(.secondary)
                        .disabled(draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
                if !shownSaved.isEmpty {
                    CardSection("Your prompts") {
                        ForEach(shownSaved) { prompt in
                            HStack {
                                promptRow(prompt.title, detail: prompt.text.split(separator: "\n").first.map(String.init) ?? "", icon: "pencil") {
                                    choose(prompt.text)
                                }
                                IconButton("Remove", systemImage: "trash") { remove(prompt) }
                            }
                        }
                    }
                }
                if shown.isEmpty && shownSaved.isEmpty {
                    CardSection { Text("No prompt matches that.").foregroundStyle(Palette.secondary) }
                } else {
                    CardSection("Prompts") {
                        ForEach(shown) { prompt in
                            promptRow(prompt.title, detail: prompt.detail, icon: prompt.icon) { choose(prompt.text) }
                        }
                    }
                }
            }
        }
    }

    private func promptRow(_ title: String, detail: String, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: icon).frame(width: 22).foregroundStyle(Palette.secondary)
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).foregroundStyle(Palette.text)
                    Text(detail).font(.caption).foregroundStyle(Palette.secondary).lineLimit(2)
                }
                Spacer(minLength: 0)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func choose(_ text: String) {
        pick(text)
        dismiss()
    }

    private func save() {
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        let title = text.split(separator: "\n").first.map { String($0.prefix(60)) } ?? "My prompt"
        var prompts = savedPrompts.filter { $0.text != text }
        prompts.insert(Saved(title: title, text: text), at: 0)
        if let data = try? JSONEncoder().encode(Array(prompts.prefix(50))), let json = String(data: data, encoding: .utf8) {
            saved = json
        }
    }

    private func remove(_ prompt: Saved) {
        if let data = try? JSONEncoder().encode(savedPrompts.filter { $0.text != prompt.text }),
           let json = String(data: data, encoding: .utf8) {
            saved = json
        }
    }
}
