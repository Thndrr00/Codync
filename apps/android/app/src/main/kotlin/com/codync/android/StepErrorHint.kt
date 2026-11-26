/** A short what-to-do hint for a failed workflow step, from the error the host recorded. */
internal object StepErrorHint {
    fun kind(error: String): String {
        val lower = error.lowercase()
        if ("sign" in lower || "auth" in lower) return "signin"
        if ("model" in lower || "unknown" in lower) return "model"
        if ("limit" in lower || "quota" in lower) return "limit"
        if ("field" in lower || "answer" in lower) return "field"
        return "other"
    }

    fun hint(kind: String): String = when (kind) {
        "signin" -> "Sign in again in Settings, then run this step."
        "model" -> "Choose another model for this step."
        "limit" -> "Wait for the limit to reset, or use a backup model."
        "field" -> "Give every path into this step the field, or mark it optional."
        else -> "Check the step settings and run it again."
    }
}
