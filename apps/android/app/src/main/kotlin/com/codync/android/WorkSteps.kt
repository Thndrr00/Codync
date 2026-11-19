package com.codync.android

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.codync.android.core.Entry
import com.codync.android.design.Disclosure
import com.codync.android.design.Text

/** One trace row, or a run of tool steps folded into a single "worked on N steps" line. */
sealed interface WorkSlot {
    data class Row(val entry: Entry) : WorkSlot
    data class Group(val entries: List<Entry>) : WorkSlot
}

/** Folds runs of at least `min` consecutive tool steps; shorter runs stay visible row by row. */
fun groupWorkSteps(entries: List<Entry>, min: Int = 3): List<WorkSlot> {
    val out = mutableListOf<WorkSlot>()
    val run = mutableListOf<Entry>()
    fun flush() {
        if (run.size >= min) out += WorkSlot.Group(run.toList())
        else run.forEach { out += WorkSlot.Row(it) }
        run.clear()
    }
    for (entry in entries) {
        if (entry.kind == "tool") run += entry
        else {
            flush()
            out += WorkSlot.Row(entry)
        }
    }
    flush()
    return out
}

/** A folded group of tool steps: the count is visible, the rows open on tap. */
@Composable
internal fun WorkStepsGroup(entries: List<Entry>) {
    var expanded by remember { mutableStateOf(false) }
    Disclosure(expanded = expanded, toggle = { expanded = !expanded }, label = {
        Text("Worked on ${entries.size} steps")
    }) {
        for (entry in entries) TraceRow(entry)
    }
}
