import Foundation

/// A header name is an HTTP token: letters, digits and a small set of separators,
/// never a space, colon, slash or bracket (OpenBot nightly triage-13174). The form,
/// the host and an agent all refuse the same names.
enum HeaderNames {
    static let message =
        "Header names cannot contain spaces or symbols such as : ( ) /. Use a name such as Authorization, and put the key or token in Value."

    private static let token = try? NSRegularExpression(
        pattern: "^[!#$%&'*+.^_`|~0-9A-Za-z-]+$",
        options: []
    )

    static func valid(_ name: String) -> Bool {
        guard !name.isEmpty, name.count <= 64, let token else { return false }
        let range = NSRange(name.startIndex..., in: name)
        return token.numberOfMatches(in: name, range: range) == 1
    }

    /// The first unusable key in a "Name: value" text box, or nil when every row is fine.
    static func invalidKey(in text: String) -> String? {
        for line in text.split(separator: "\n", omittingEmptySubsequences: false) {
            guard let at = line.firstIndex(of: ":") else { continue }
            let key = line[..<at].trimmingCharacters(in: .whitespaces)
            if !key.isEmpty && !valid(String(key)) { return String(key) }
        }
        return nil
    }
}
