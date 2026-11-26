import Foundation

/// Chat replies turned into what a call reads aloud.
enum SpokenText {
    /// One utterance the provider already posted, and when (OpenBot triage-15781).
    struct Utterance {
        let text: String
        let at: Date
    }

    /// Where a new turn came from, for whether it may cut off a reply being read aloud.
    enum Source: String, Sendable {
        case user, channel, thread, selfTurn, notice
    }

    /**
     * Markdown a voice never says (OpenBot 0.78.0): headings, quotes, `-`/`+`/`*`/`•`/`1.` list
     * markers, `[ ]` task boxes and setext `===` underlines. A marker is dropped only at the start
     * of a line and before whitespace, so punctuation inside words (`C++`, `x*2`, `#tag`) stays;
     * nested markers are peeled one at a time.
     */
    static func cleanSpokenText(_ text: String) -> String {
        let quote = #"^\s{0,3}>\s*"#
        let marker = #"^\s{0,3}(?:#{1,6}|[•+*-]|\d+[.)]|\[[ xX]\])(?:\s+|$)"#
        let underline = #"^\s{0,3}(?:={3,}|-{3,}|_{3,})\s*$"#
        return text.split(separator: "\n", omittingEmptySubsequences: false).map { line in
            var rest = String(line)
            guard rest.range(of: underline, options: .regularExpression) == nil else { return "" }
            while true {
                let next = rest.replacingOccurrences(of: quote, with: "", options: .regularExpression)
                    .replacingOccurrences(of: marker, with: "", options: .regularExpression)
                if next == rest { break }
                rest = next
            }
            return rest
        }.joined(separator: "\n")
    }

    /// A live-voice request never shows internal harness instructions as words the person spoke.
    static func spokenRequest(_ text: String) -> String {
        let harnessLine = #"^\s*#\s*AGENTS\.md"#
        return text.split(separator: "\n", omittingEmptySubsequences: false)
            .filter { String($0).range(of: harnessLine, options: [.regularExpression, .caseInsensitive]) == nil }
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// A reply as it should sound: markdown syntax and code blocks dropped.
    static func from(_ markdown: String) -> String {
        var text = cleanSpokenText(markdown)
        let rules: [(String, String)] = [
            (#"```[\s\S]*?```"#, ""),                  // code blocks
            (#"!?\[([^\]]*)\]\([^)]*\)"#, "$1"),       // links and images → their text
            (#"`([^`]*)`"#, "$1"),                     // inline code
            (#"(\*\*|\*|~~)(\S[^\n]*?)\1"#, "$2"),   // emphasis (not `_`: it's in identifiers)
            (#"(?m)^[ \t]*\|?[ \t:|-]{3,}\|?[ \t]*$"#, ""), // table rules
            (#"(?m)^[ \t]*\|[ \t]*|[ \t]*\|[ \t]*$"#, ""), // table edges
            (#"[ \t]*\|[ \t]*"#, ", "),                      // table cells
            (#"\n{2,}"#, "\n"),
        ]
        for (pattern, template) in rules {
            text = text.replacingOccurrences(of: pattern, with: template, options: .regularExpression)
        }
        return text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// How soon the same words are one utterance the provider posted twice (triage-15781).
    static let utteranceRepeatSeconds = 2.0

    /// True when the provider re-sent exactly these words within the window, not a new turn.
    static func isRepeatUtterance(_ text: String, _ last: Utterance?, windowSeconds: Double, now: Date) -> Bool {
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, let last else { return false }
        let interval = now.timeIntervalSince(last.at)
        return text.trimmingCharacters(in: .whitespacesAndNewlines) == last.text.trimmingCharacters(in: .whitespacesAndNewlines)
            && interval >= 0 && interval <= windowSeconds
    }

    /// Only a new explicit user message in the main chat interrupts a reply; the same agent's
    /// channel, thread or self-started work keeps reading (triage-15567).
    static func shouldInterruptSpeech(source: Source, isUserMainMessage: Bool) -> Bool {
        isUserMainMessage && source == .user
    }
}
