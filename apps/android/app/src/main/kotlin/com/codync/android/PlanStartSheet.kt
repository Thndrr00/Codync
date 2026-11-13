package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.codync.android.core.PlanStart
import com.codync.android.design.*
import kotlinx.coroutines.launch

/** "10 s", "1 min", "1 min 30 s". */
internal fun planDelayText(ms: Long): String {
    val minutes = ms / 60_000
    val seconds = ((ms % 60_000) / 1_000.0).toInt()
    return when {
        minutes == 0L -> "$seconds s"
        seconds == 0 -> "$minutes min"
        else -> "$minutes min $seconds s"
    }
}

/**
 * "Start the work": when a plan card of a terminal agent starts the plan by itself, for every agent on this
 * computer. Ported from OpenBot's Settings > Agents > "Start the work".
 */
@Composable internal fun PlanStartSheet(state: AppState, store: AppStore) {
    var setting by remember { mutableStateOf<PlanStart?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var customOpen by remember { mutableStateOf(false) }
    var amount by remember { mutableStateOf("1") }
    var minutes by remember { mutableStateOf(true) }
    val scope = rememberCoroutineScope()
    val computer = state.computers.firstOrNull()?.id
    LaunchedEffect(computer) {
        setting = runCatching { store.planStart() }.getOrElse {
            error = it.message ?: "That did not work."
            null
        }
    }
    fun save(afterMs: Long?) {
        if (busy) return
        busy = true
        error = null
        scope.launch {
            try {
                store.setPlanStart(afterMs)
                setting = store.planStart()
                customOpen = false
            } catch (e: Exception) {
                error = e.message ?: "That did not work."
            } finally {
                busy = false
            }
        }
    }
    fun typed(current: PlanStart): Long? {
        val value = amount.trim().toLongOrNull() ?: return null
        if (amount.trim().any { !it.isDigit() }) return null
        val ms = value * if (minutes) 60_000 else 1_000
        return ms.takeIf { it in current.min..current.max }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Plan cards")
        CardForm {
            val current = setting
            if (current == null) {
                CardSection { row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) } }
            } else {
                val footer = current.afterMs?.let {
                    tr("The work starts after {time}, or when you press Start.").replace("{time}", planDelayText(it))
                } ?: tr("The plan waits until you press Start. You get a notification.")
                CardSection(footer = footer) {
                    row {
                        ValueRow("Start the work") {
                            val options = buildList {
                                add("wait" to tr("When I press Start"))
                                current.presets.forEach { add(it.toString() to tr("After {time}").replace("{time}", planDelayText(it))) }
                                current.afterMs?.takeIf { it !in current.presets }?.let {
                                    add(it.toString() to tr("After {time}").replace("{time}", planDelayText(it)))
                                }
                                add("custom" to tr("Custom time…"))
                            }
                            ChoicePicker(current.afterMs?.toString() ?: "wait", options, { id ->
                                if (id == "custom") customOpen = true else save(if (id == "wait") null else id.toLong())
                            }, enabled = !busy, label = tr("Start the work"))
                        }
                    }
                    if (customOpen) {
                        row { FieldBox(amount, { amount = it }, "5", fill = Palette.bubbleUser) }
                        row {
                            ValueRow("Time before the work starts") {
                                ChoicePicker(if (minutes) "min" else "s",
                                    listOf("s" to tr("sec"), "min" to tr("min")), { minutes = it == "min" },
                                    label = tr("Time before the work starts"))
                            }
                        }
                        val custom = typed(current)
                        row {
                            Text(
                                if (custom == null) tr("Enter a whole number of at least 5 seconds.") else tr("At least 5 seconds."),
                                style = Type.footnote,
                                color = if (custom == null) Palette.danger else Palette.secondary,
                                modifier = Modifier.weight(1f)
                            )
                            SecondaryButton(tr("Cancel"), enabled = !busy) { customOpen = false }
                            PrimaryButton(tr("Set"), enabled = !busy && custom != null) { save(custom) }
                        }
                    }
                }
            }
            error?.let { message -> row { Text(message, style = Type.footnote, color = Palette.danger) } }
            if (busy) row { Text(tr("Saving…"), style = Type.footnote, color = Palette.secondary) }
        }
    }
}
