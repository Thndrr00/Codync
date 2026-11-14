package com.codync.android

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import org.json.JSONArray
import org.json.JSONObject

/** A starting point for a message; same list as the desktop and iPhone prompt libraries. */
internal data class PromptItem(val title: String, val detail: String, val icon: String, val text: String)

private fun prompt(title: String, detail: String, icon: String, text: String) = PromptItem(title, detail, icon, text)

internal val promptLibrary: List<PromptItem> = listOf(
    prompt("Summarize this", "A short summary, the key points, and what to do next.", "doc.plaintext",
        "Summarize the following, in this order: one sentence saying what it is, the key points as a short list, and any decision or next step it implies. Say what is missing if something needed is not there.\n\n[paste the text or attach the file]"),
    prompt("Plan a project", "Goal, steps, order, risks and what done means.", "list.bullet.rectangle",
        "Plan this project: [what it is]. Give me the goal in one sentence, the steps in the order they should happen with the result of each, the risks with how I would notice them, and a clear test for what \"done\" means. Ask me one question first only if the answer would change the plan."),
    prompt("Review a change", "Find what breaks, with the smallest fix for each.", "checklist",
        "Review this change as a careful reviewer. For each problem, give the trigger (an input or state), the wrong result, and the smallest fix. Separate blockers from nits. If you cannot name a trigger, leave it out.\n\n[paste the diff or point me at the files]"),
    prompt("Find the risks", "What can go wrong, how I would see it, and what to do.", "exclamationmark.triangle",
        "List the risks in [the plan, change or situation]. For each one: what would trigger it, what the consequence is, how I would notice it early, and the cheapest response. Rank them by how much they would hurt, not by how likely they feel."),
    prompt("Notes into actions", "Decisions, owners, dates and open questions.", "checkmark.circle",
        "Turn these notes into decisions, actions and open questions. Each action gets an owner, a date if the notes give one, and the line it came from. Do not invent an owner or a date; write \"no owner\" or \"no date\" instead.\n\n[paste the notes]"),
    prompt("Draft an email", "One message with the reason, the ask and the next step.", "square.and.pencil",
        "Draft an email to [who] about [what]. Put the reason for writing in the first sentence, then what I need from them as one clear ask, then what happens next. Keep it under 150 words and match a plain, direct voice. Do not invent facts, prices or promises."),
    prompt("Compare two options", "Criteria from my decision, not from the products.", "arrow.left.arrow.right",
        "Compare [option A] and [option B] for [the decision I am making]. First list the criteria that matter for this decision and mark which are must-haves. Then fill a small table with a source or a reason per cell. End with which one fits and what would change the answer."),
    prompt("Debug this", "Reproduce, find the first wrong step, then the smallest fix.", "wrench.and.screwdriver",
        "Help me debug this. Here is what I expected, what happened, and the exact error:\n\n[expected]\n[actual]\n[error text]\n\nReproduce it first, then find the first point where the real state differs from the expected one. Give me at most three hypotheses with the check that would rule each out, and the smallest fix once we know which one it is."),
    prompt("Explain it simply", "The idea in plain words, then the detail that matters.", "sparkles",
        "Explain [the idea] in plain words first, in two or three sentences, as if I know the field but not this part. Then give the detail that changes how I would use it, and one example. Name anything you are unsure about instead of guessing."),
    prompt("Extract a table", "Rows and columns from text or a file, with the blanks marked.", "square.grid.3x3",
        "Extract this into a table with the columns [columns]. One row per [what a row is]. Keep values exactly as written, and write \"missing\" where a value is not there. Tell me which rows you were unsure about and why.\n\n[paste the text or attach the file]"),
    prompt("Ask me questions first", "The few questions that would change the answer.", "info.circle",
        "Before you start on [the task], ask me the few questions whose answers would change what you do. At most five, one line each, with the options you expect. If none would change it, say so and begin."),
    prompt("Check my writing", "Meaning kept, filler gone, unsupported claims flagged.", "pencil",
        "Edit this text without changing what it means. Cut filler and stock phrases, shorten long sentences, and make the actor clear. List every fact, number and claim before and after so none disappear, and flag any claim that needs a source instead of inventing one.\n\n[paste the text]"),
)

private const val PROMPTS_KEY = "codyncPromptLibrary"

private fun savedPrompts(context: android.content.Context): List<Pair<String, String>> {
    val raw = context.getSharedPreferences(PROMPTS_KEY, 0).getString(PROMPTS_KEY, null) ?: return emptyList()
    return runCatching {
        val array = JSONArray(raw)
        (0 until array.length()).map { index ->
            val item = array.getJSONObject(index)
            item.getString("title") to item.getString("text")
        }
    }.getOrDefault(emptyList())
}

private fun writeSavedPrompts(context: android.content.Context, prompts: List<Pair<String, String>>) {
    val array = JSONArray()
    for ((title, text) in prompts.take(50)) array.put(JSONObject().put("title", title).put("text", text))
    context.getSharedPreferences(PROMPTS_KEY, 0).edit().putString(PROMPTS_KEY, array.toString()).apply()
}

/** Pick a prompt to fill the message box, or save what it holds now (iOS `PromptLibraryView`). */
@Composable
internal fun PromptLibrarySheet(draft: String, pick: (String) -> Unit) {
    val context = LocalContext.current
    val dismiss = LocalDismissModal.current
    var query by remember { mutableStateOf("") }
    var saved by remember { mutableStateOf(savedPrompts(context)) }
    val needle = query.trim().lowercase()
    val shownSaved = saved.filter { needle.isEmpty() || it.first.lowercase().contains(needle) || it.second.lowercase().contains(needle) }
    val shown = promptLibrary.filter { needle.isEmpty() || it.title.lowercase().contains(needle) || it.detail.lowercase().contains(needle) }

    fun choose(text: String) {
        pick(text)
        dismiss()
    }

    Column(Modifier.fillMaxWidth()) {
        ModalHeader("Prompt library")
        CardForm {
            row { SearchField(query, { query = it }, prompt = "Search prompts") }
            CardSection(footer = "Saved on this phone. Picking a prompt fills the message box; edit it before sending.") {
                row {
                    SecondaryButton("Save what the message box holds", enabled = draft.isNotBlank()) {
                        val text = draft.trim()
                        val title = text.lineSequence().firstOrNull()?.take(60) ?: "My prompt"
                        saved = listOf(title to text) + saved.filter { it.second != text }
                        writeSavedPrompts(context, saved)
                    }
                }
            }
            if (shownSaved.isNotEmpty()) CardSection("Your prompts") {
                for ((title, text) in shownSaved) row {
                    Row(Modifier.fillMaxWidth().clickable { choose(text) }, verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol("pencil", size = 16.dp, tint = Palette.secondary)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(title, style = Type.body, color = Palette.text)
                            Text(text.lineSequence().firstOrNull() ?: "", style = Type.footnote, color = Palette.secondary,
                                maxLines = 2, overflow = TextOverflow.Ellipsis)
                        }
                        IconButton("Remove", "trash") {
                            saved = saved.filter { it.second != text }
                            writeSavedPrompts(context, saved)
                        }
                    }
                }
            }
            if (shown.isEmpty() && shownSaved.isEmpty()) CardSection {
                row { Text("No prompt matches that.", style = Type.body, color = Palette.secondary) }
            } else if (shown.isNotEmpty()) CardSection("Prompts") {
                for (prompt in shown) row {
                    Row(Modifier.fillMaxWidth().clickable { choose(prompt.text) }, verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol(prompt.icon, size = 16.dp, tint = Palette.secondary)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(prompt.title, style = Type.body, color = Palette.text)
                            Text(prompt.detail, style = Type.footnote, color = Palette.secondary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                        }
                    }
                }
            }
        }
    }
}

