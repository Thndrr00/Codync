package com.codync.android.core

import kotlinx.serialization.Serializable

/**
 * Making a picture or a clip with the bot's own agent (host/src/media, iOS `Media`): the host's
 * `mediaModels` says which models, frames and lengths that agent can work with; the picker turns a
 * choice into the `/image …` or `/video …` message that carries it, and the host writes the request.
 * Ported from OpenBot 0.70.0 `packages/ui/src/features/workflows/WorkflowMediaModelPicker.tsx`.
 */
@Serializable
data class MediaModel(
    val id: String,
    val name: String,
    /** The company that makes the model. */
    val maker: String,
    /** The subscription that pays for the file. */
    val subscription: String,
    /** What the model makes: `image`, `video`. */
    val kinds: List<String>,
    val aspects: List<String>,
    /** An image of this model can have a clear background. */
    val transparent: Boolean,
    /** A video of this model can speak in a preset voice. */
    val voices: Boolean,
    val capabilities: MediaCapabilities = MediaCapabilities(),
) {
    val makesImage: Boolean get() = "image" in kinds
    val makesVideo: Boolean get() = "video" in kinds
}

@Serializable
data class MediaCapabilities(
    val aspects: List<String> = emptyList(),
    val durations: List<Int> = emptyList(),
    val resolutions: List<String> = emptyList(),
    val transparent: Boolean = false,
    /** Null for a model whose videos don't speak. */
    val voices: List<MediaVoice>? = null,
)

@Serializable
data class MediaVoice(val id: String, val name: String)

/** The host's answer to `mediaModels`: no `agent` when the bot's agent has no image tool. */
@Serializable
data class MediaAnswer(
    val agent: String? = null,
    val installed: Boolean = false,
    val models: List<MediaModel> = emptyList(),
)

/** What the picker chose; the flags of the message follow it. */
data class MediaChoice(
    val kind: String,
    val model: String,
    val aspect: String,
    /** Seconds; only a video has one. */
    val duration: Int,
    /** `480p`, `720p`…; only a video has one. */
    val resolution: String,
    val transparent: Boolean,
    val voice: String?,
)

/** The flags and their values, in the order the host reads them. */
private val VALUED = listOf("--model", "--aspect", "--duration", "--resolution", "--voice", "--first", "--last", "--ref")

/** The models of a kind, the first one chosen by default. */
fun MediaAnswer?.modelsOf(kind: String): List<MediaModel> =
    this?.models.orEmpty().filter { if (kind == "video") it.makesVideo else it.makesImage }

/** The choice the picker starts on: the kind's first model at its first frame, length and resolution. */
fun MediaAnswer?.firstChoice(kind: String): MediaChoice? {
    val model = modelsOf(kind).firstOrNull() ?: return null
    return MediaChoice(
        kind = kind,
        model = model.id,
        aspect = model.capabilities.aspects.firstOrNull() ?: "1:1",
        duration = model.capabilities.durations.firstOrNull() ?: 0,
        resolution = model.capabilities.resolutions.firstOrNull().orEmpty(),
        transparent = false,
        voice = null,
    )
}

/** The same choice on another model, keeping only the settings the new one makes. */
fun MediaChoice.onModel(model: MediaModel): MediaChoice {
    val caps = model.capabilities
    return copy(
        model = model.id,
        aspect = aspect.takeIf { it in caps.aspects } ?: caps.aspects.firstOrNull() ?: "1:1",
        duration = duration.takeIf { it in caps.durations } ?: caps.durations.firstOrNull() ?: 0,
        resolution = resolution.takeIf { it in caps.resolutions } ?: caps.resolutions.firstOrNull().orEmpty(),
        voice = voice?.takeIf { id -> caps.voices.orEmpty().any { it.id == id } },
        transparent = transparent && caps.transparent,
    )
}

/** The words of [draft] that are the description: after the command and its flags. */
fun mediaPrompt(draft: String): String {
    val words = draft.trim().split(WHITESPACE).filter { it.isNotEmpty() }.toMutableList()
    // A command being typed (`/`, `/ima`, `/image`) is not part of the description.
    if (words.firstOrNull()?.startsWith("/") == true) words.removeAt(0)
    var i = 0
    while (i < words.size) {
        val word = words[i]
        if (word == "--transparent") i += 1
        else if (word in VALUED) i += 2
        else break
    }
    return words.drop(i).joinToString(" ")
}

/** The chat message that asks for [choice], keeping the description already typed in [draft]. */
fun mediaDraft(choice: MediaChoice, draft: String): String {
    val flags = mutableListOf("--model ${choice.model}", "--aspect ${choice.aspect}")
    if (choice.kind == "video") {
        flags += "--duration ${choice.duration}"
        flags += "--resolution ${choice.resolution}"
        choice.voice?.let { flags += "--voice $it" }
    } else if (choice.transparent) {
        flags += "--transparent"
    }
    val prompt = mediaPrompt(draft)
    val command = "/${choice.kind} ${flags.joinToString(" ")}"
    return if (prompt.isEmpty()) "$command " else "$command $prompt"
}

/** The line under a model: who makes it and what pays for it, and when its agent isn't installed. */
fun mediaDetail(model: MediaModel, agentInstalled: Boolean): String {
    val base = "${model.maker} · ${model.subscription}"
    return if (agentInstalled) base else "$base · not installed"
}

/** Whether the picker is offered: a group has no agent of its own. */
fun Bot.mediaOffered(): Boolean = kind != "group"

private val WHITESPACE = Regex("\\s+")
