/// A short what-to-do hint for a failed workflow step, from the error the host recorded.
enum StepErrorHint {
    static func kind(_ error: String) -> String {
        let lower = error.lowercased()
        if lower.contains("sign") || lower.contains("auth") { return "signin" }
        if lower.contains("model") || lower.contains("unknown") { return "model" }
        if lower.contains("limit") || lower.contains("quota") { return "limit" }
        if lower.contains("field") || lower.contains("answer") { return "field" }
        return "other"
    }

    static func hint(_ kind: String) -> String {
        switch kind {
        case "signin": return "Sign in again in Settings, then run this step."
        case "model": return "Choose another model for this step."
        case "limit": return "Wait for the limit to reset, or use a backup model."
        case "field": return "Give every path into this step the field, or mark it optional."
        default: return "Check the step settings and run it again."
        }
    }
}
