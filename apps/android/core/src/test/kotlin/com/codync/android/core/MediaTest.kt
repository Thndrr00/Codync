package com.codync.android.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MediaTest {
    private fun model(id: String, kinds: List<String>, caps: MediaCapabilities = MediaCapabilities()) =
        MediaModel(id, id, "x", "y", kinds, listOf("1:1", "16:9"), false, false, caps)

    private val answer = MediaAnswer(
        agent = "grok",
        installed = true,
        models = listOf(
            model("grok-imagine", listOf("image", "video"),
                MediaCapabilities(listOf("1:1", "16:9"), listOf(6, 10), listOf("480p", "720p"), true, listOf(MediaVoice("ara", "Ara")))),
        ),
    )

    @Test fun `a kind offers only the models that make it, starting on the first`() {
        assertEquals(listOf("grok-imagine"), answer.modelsOf("image").map { it.id })
        assertEquals(emptyList<MediaModel>(), MediaAnswer(models = listOf(model("gpt-image", listOf("image")))).modelsOf("video"))
        assertNull((null as MediaAnswer?).firstChoice("image"))
        assertEquals(MediaChoice("video", "grok-imagine", "1:1", 6, "480p", false, null), answer.firstChoice("video"))
    }

    @Test fun `another model keeps only the settings it makes`() {
        val choice = MediaChoice("video", "grok-imagine", "16:9", 10, "720p", true, "ara")
        val moved = choice.onModel(model("other", listOf("video")))
        assertEquals("other", moved.model)
        assertEquals("1:1", moved.aspect)
        assertEquals(0, moved.duration)
        assertNull(moved.voice)
        assertEquals(false, moved.transparent)
    }

    @Test fun `the description after the flags is kept and the flags are rebuilt`() {
        assertEquals("a red fox", mediaPrompt("a red fox"))
        assertEquals("a red fox", mediaPrompt("/image --model grok-imagine --aspect 16:9 a red fox"))
        assertEquals("a pan", mediaPrompt("/video --ref /tmp/a.png --voice ara a pan"))
        assertEquals("a cup", mediaPrompt("/image --transparent a cup"))
        assertEquals("a fox", mediaPrompt("/ima a fox"))
        assertEquals("a fox", mediaPrompt("/ a fox"))
        assertEquals("", mediaPrompt("/image"))
        val choice = MediaChoice("image", "gpt-image", "4:3", 0, "", true, null)
        assertEquals("/image --model gpt-image --aspect 4:3 --transparent a cup", mediaDraft(choice, "/image a cup"))
        assertEquals("/image --model gpt-image --aspect 4:3 --transparent ", mediaDraft(choice, "/"))
        val video = choice.copy(kind = "video", duration = 10, resolution = "720p", voice = "ara")
        assertEquals("/video --model gpt-image --aspect 4:3 --duration 10 --resolution 720p --voice ara ", mediaDraft(video, ""))
    }

    @Test fun `a model line names who pays, and a missing agent too`() {
        assertEquals("x · y", mediaDetail(model("m", listOf("image")), true))
        assertTrue(mediaDetail(model("m", listOf("image")), false).endsWith("not installed"))
    }
}
