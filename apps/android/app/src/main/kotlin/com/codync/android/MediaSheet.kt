package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.*
import com.codync.android.design.*

/**
 * The picker behind `/image` and `/video` (iOS `MediaSheet`): the models this bot's agent can make
 * files with, the frame, and for a video its length, resolution and voice. It writes the command
 * into the box, so the person only describes what to make (host/src/media).
 * Ported from OpenBot 0.70.0 `WorkflowMediaModelPicker.tsx` and `WorkflowImageNode.tsx`.
 */
@Composable internal fun MediaSheet(store: AppStore, botId: String, asked: String, draft: String, dismiss: () -> Unit,
    use: (String) -> Unit) {
    var answer by remember { mutableStateOf<MediaAnswer?>(null) }
    var failed by remember { mutableStateOf(false) }
    var kind by remember(asked) { mutableStateOf(asked) }
    var choice by remember { mutableStateOf<MediaChoice?>(null) }
    // Every opening asks again: another agent account may answer, and a bot's agent can change.
    LaunchedEffect(botId, asked) {
        answer = null
        failed = false
        choice = null
        try {
            val loaded = store.mediaModels(botId)
            answer = loaded
            choice = loaded.firstChoice(asked)
        } catch (cancelled: kotlinx.coroutines.CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            failed = true
        }
    }
    val models = answer.modelsOf(kind)
    val model = models.firstOrNull { it.id == choice?.model }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(tr(kind.title())) {
            PrimaryButton("Use", enabled = choice != null) {
                choice?.let { use(mediaDraft(it, draft)) }
                dismiss()
            }
        }
        CardForm {
            CardSection {
                row {
                    SegmentedChoice(kind, listOf("image" to tr("Image"), "video" to tr("Video")), { picked ->
                        kind = picked
                        choice = answer.firstChoice(picked)
                    })
                }
                answer?.agent?.let { agent ->
                    row {
                        Text(tr("Made by {agent}, paid with its subscription").replace("{agent}", agent), style = Type.footnote,
                            color = Palette.secondary)
                    }
                }
                row {
                    when {
                        failed -> Text(tr("This computer's host is not answering."), style = Type.body, color = Palette.warning)
                        answer == null -> Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Spinner()
                            Text(tr("Checking what this bot can makeâ€¦"), style = Type.body, color = Palette.secondary)
                        }
                        answer?.agent == null -> Text(tr("This botâ€™s agent has no image tool. Pick one that does, or use ComfyUI."),
                            style = Type.body, color = Palette.secondary)
                        models.isEmpty() -> Text(tr("This botâ€™s agent makes no {kind}.").replace("{kind}", tr(kind.title()).lowercase()),
                            style = Type.body, color = Palette.secondary)
                        else -> Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            if (answer?.installed == false) Text(tr("This botâ€™s agent is not installed on this computer yet."),
                                style = Type.footnote, color = Palette.warning)
                            if (models.size > 1) ChoiceList(choice?.model.orEmpty(), models.map { Triple(it.id, it.name, mediaDetail(it, answer?.installed != false)) }, { id ->
                                models.firstOrNull { it.id == id }?.let { next -> choice = choice?.onModel(next) }
                            }) else model?.let { Text(mediaDetail(it, answer?.installed != false), style = Type.footnote, color = Palette.secondary) }
                        }
                    }
                }
                val picked = choice
                if (picked != null && model != null) {
                    row { Text(tr("Aspect ratio"), style = Type.footnote, color = Palette.secondary) }
                    row {
                        TabChoice(picked.aspect, model.capabilities.aspects.map { it to it }, { choice = picked.copy(aspect = it) })
                    }
                    if (picked.kind == "video" && model.capabilities.durations.isNotEmpty()) {
                        row { Text(tr("Length"), style = Type.footnote, color = Palette.secondary) }
                        row {
                            TabChoice(picked.duration, model.capabilities.durations.map { it to "${it}s" }, { choice = picked.copy(duration = it) })
                        }
                    }
                    if (picked.kind == "video" && model.capabilities.resolutions.isNotEmpty()) {
                        row { Text(tr("Resolution"), style = Type.footnote, color = Palette.secondary) }
                        row {
                            TabChoice(picked.resolution, model.capabilities.resolutions.map { it to it }, { choice = picked.copy(resolution = it) })
                        }
                    }
                    val voices = model.capabilities.voices
                    if (picked.kind == "video" && voices != null) {
                        row { Text(tr("Voice"), style = Type.footnote, color = Palette.secondary) }
                        row {
                            TabChoice(picked.voice.orEmpty(), listOf("" to tr("None")) + voices.map { it.id to it.name }, {
                                choice = picked.copy(voice = it.ifEmpty { null })
                            })
                        }
                    }
                    if (picked.kind == "image" && model.capabilities.transparent) {
                        row {
                            CodyncSwitch(picked.transparent, { choice = picked.copy(transparent = it) }) {
                                Text(tr("Transparent background"), style = Type.body, color = Palette.text)
                            }
                        }
                    }
                    row {
                        SelectionContainer {
                            Text(mediaDraft(picked, draft), Modifier.fillMaxWidth().padding(vertical = 2.dp), style = Type.footnote.copy(fontFamily = Type.monospaced),
                                color = Palette.secondary, maxLines = 3, overflow = TextOverflow.Ellipsis)
                        }
                    }
                }
            }
        }
    }
}

/** The sheet's title for a command. */
private fun String.title(): String = if (this == "video") "Video" else "Image"

