package com.codync.android.core

/**
 * Local syntax highlighting for fenced code (idea 25): a small scanner that marks comments,
 * strings, numbers and keywords. Unknown languages and blocks past [MAX_CHARS] return no spans,
 * so they render as plain text; nothing here ever reaches the network.
 */
object CodeHighlight {
    enum class Kind { Keyword, Literal, Comment, Number }

    data class Span(val start: Int, val end: Int, val kind: Kind)

    /** Longer blocks stay plain text: tokenizing them would stall a streaming reply. */
    const val MAX_CHARS = 20_000

    private class Family(val lineComments: List<String>, val blockComments: Boolean, val backtickStrings: Boolean, val keywords: Set<String>)

    private val cLike = Family(listOf("//"), true, true, setOf(
        "abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "data", "default", "defer",
        "do", "else", "enum", "extends", "false", "final", "finally", "fn", "for", "fun", "func", "function", "if", "impl",
        "implements", "import", "in", "interface", "internal", "is", "let", "match", "mod", "mut", "new", "null", "nil", "object",
        "override", "package", "private", "protected", "pub", "public", "return", "self", "static", "struct", "super", "switch",
        "this", "throw", "throws", "trait", "true", "try", "type", "typealias", "use", "val", "var", "void", "when", "where", "while", "yield",
    ))
    private val hash = Family(listOf("#"), false, false, setOf(
        "and", "as", "async", "await", "break", "case", "class", "continue", "def", "del", "do", "done", "elif", "else", "end",
        "esac", "except", "export", "False", "false", "fi", "finally", "for", "from", "function", "if", "import", "in", "is",
        "lambda", "None", "nil", "not", "null", "or", "pass", "raise", "return", "then", "True", "true", "try", "while", "with", "yield",
    ))
    private val json = Family(emptyList(), false, false, setOf("true", "false", "null"))

    private val families = mapOf(
        "kotlin" to cLike, "kt" to cLike, "java" to cLike, "swift" to cLike, "rust" to cLike, "rs" to cLike, "go" to cLike,
        "js" to cLike, "javascript" to cLike, "ts" to cLike, "typescript" to cLike, "tsx" to cLike, "jsx" to cLike, "c" to cLike,
        "cpp" to cLike, "c++" to cLike, "cs" to cLike, "csharp" to cLike, "dart" to cLike,
        "python" to hash, "py" to hash, "sh" to hash, "bash" to hash, "zsh" to hash, "shell" to hash, "yaml" to hash, "yml" to hash,
        "toml" to hash, "ruby" to hash, "rb" to hash, "dockerfile" to hash,
        "json" to json, "jsonc" to json,
    )

    fun supports(language: String): Boolean = family(language) != null

    /** Ordered, non-overlapping spans of [code]; empty when the language is unknown or the block too long. */
    fun spans(code: String, language: String): List<Span> {
        val family = family(language) ?: return emptyList()
        if (code.length > MAX_CHARS) return emptyList()
        val spans = mutableListOf<Span>()
        var i = 0
        while (i < code.length) {
            val c = code[i]
            when {
                family.lineComments.any { code.startsWith(it, i) } -> {
                    val end = lineEnd(code, i)
                    spans += Span(i, end, Kind.Comment); i = end
                }
                family.blockComments && code.startsWith("/*", i) -> {
                    val close = code.indexOf("*/", i + 2)
                    val end = if (close < 0) code.length else close + 2
                    spans += Span(i, end, Kind.Comment); i = end
                }
                c == '"' || c == '\'' || (c == '`' && family.backtickStrings) -> {
                    val end = stringEnd(code, i, multiline = c == '`')
                    spans += Span(i, end, Kind.Literal); i = end
                }
                c.isDigit() && (i == 0 || !(code[i - 1].isLetterOrDigit() || code[i - 1] == '_')) -> {
                    var end = i + 1
                    while (end < code.length && (code[end].isLetterOrDigit() || code[end] == '.' || code[end] == '_')) end++
                    spans += Span(i, end, Kind.Number); i = end
                }
                c.isLetter() || c == '_' -> {
                    var end = i + 1
                    while (end < code.length && (code[end].isLetterOrDigit() || code[end] == '_')) end++
                    if (code.substring(i, end) in family.keywords) spans += Span(i, end, Kind.Keyword)
                    i = end
                }
                else -> i++
            }
        }
        return spans
    }

    private fun family(language: String): Family? = families[language.trim().substringBefore(' ').lowercase()]

    private fun lineEnd(code: String, from: Int): Int = code.indexOf('\n', from).let { if (it < 0) code.length else it }

    /** The index after the string opened at [from]; an unclosed one ends at its line (or, for backticks, the text). */
    private fun stringEnd(code: String, from: Int, multiline: Boolean): Int {
        val quote = code[from]
        var i = from + 1
        while (i < code.length) {
            val c = code[i]
            when {
                c == '\\' -> i += 2
                c == quote -> return i + 1
                c == '\n' && !multiline -> return i
                else -> i++
            }
        }
        return code.length
    }
}
