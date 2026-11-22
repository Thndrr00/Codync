package com.codync.android.design

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The shape catalog the avatar picker offers, including the three new creatures. */
class AvatarShapeTest {
    @Test fun theCatalogDrawsElevenDistinctShapes() {
        assertEquals(11, AvatarPalette.shapes.size)
        assertEquals(AvatarPalette.shapes.size, AvatarPalette.shapes.toSet().size)
        for (shape in listOf("ghost", "jellyfish", "octopus")) {
            assertTrue(shape in AvatarPalette.shapes)
            assertTrue(AvatarPalette.isShape(shape))
            assertEquals(shape, AvatarPalette.safeShape(shape))
        }
    }

    @Test fun anUnknownShapeFallsBackToTheBlob() {
        assertFalse(AvatarPalette.isShape("monster"))
        assertFalse(AvatarPalette.isShape(null))
        assertEquals("blob", AvatarPalette.safeShape("monster"))
        assertEquals("blob", AvatarPalette.safeShape(null))
    }
}
