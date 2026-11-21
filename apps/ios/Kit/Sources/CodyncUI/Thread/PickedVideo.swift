import Foundation
import PhotosUI
import SwiftUI
import UniformTypeIdentifiers

/// A clip from the Photos picker, kept as its original bytes and extension. A movie can't be
/// transcoded to JPEG the way a still can, so it is read from the system's file representation.
/// `PhotosPickerItem.loadTransferable(type: Data.self)` returns nil for movies: they need a
/// `FileRepresentation`, which is what the picker uses here (the same fix as OpenBot's importer).
struct PickedVideo: Transferable {
    let data: Data
    let filenameExtension: String

    static var transferRepresentation: some TransferRepresentation {
        FileRepresentation(contentType: .movie) { received in
            let ext = received.file.pathExtension
            return Self(
                data: try Data(contentsOf: received.file),
                filenameExtension: ext.isEmpty ? "mov" : ext,
            )
        }
    }
}
