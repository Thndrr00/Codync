import Testing
import SwiftUI
@testable import CodyncKit

@Test func theAvatarCatalogIncludesTheThreeNewCreatures() {
    #expect(AvatarPalette.shapes.count == 11)
    for shape in ["ghost", "jellyfish", "octopus"] {
        #expect(AvatarPalette.shapes.contains(shape))
        #expect(AvatarPalette.isShape(shape))
        #expect(AvatarPalette.safeShape(shape) == shape)
    }
    #expect(Set(AvatarPalette.shapes).count == AvatarPalette.shapes.count)
}

@Test func anUnknownAvatarShapeFallsBackToTheBlob() {
    #expect(!AvatarPalette.isShape("monster"))
    #expect(AvatarPalette.safeShape("monster") == "blob")
    #expect(AvatarPalette.safeShape(nil) == "blob")
}

@Test func everyCatalogShapeDrawsANonEmptySilhouette() {
    let rect = CGRect(x: 0, y: 0, width: 100, height: 100)
    for shape in AvatarPalette.shapes {
        let path = CharacterShape(kind: shape).path(in: rect)
        #expect(!path.isEmpty, "\(shape) draws dots")
        #expect(path.boundingBoxOfPath.width > 0, "\(shape) has width")
        #expect(path.boundingBoxOfPath.height > 0, "\(shape) has height")
    }
}
