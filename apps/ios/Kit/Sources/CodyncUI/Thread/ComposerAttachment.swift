import CodyncKit
import SwiftUI
import UniformTypeIdentifiers

/// A local attachment, shown before it is uploaded or sent.
struct ComposerAttachment: View {
    let file: OutgoingFile
    let remove: () -> Void
    @State private var image: CGImage?
    @State private var loaded = false
    // Ported from OpenBot 0.66.0: tap an image you attached to see it full size before you send it.
    @State private var preview = false
    @Environment(\.displayScale) private var displayScale

    private var size: CGFloat { 112 }
    private var expectsImage: Bool {
        UTType(filenameExtension: (file.name as NSString).pathExtension)?.conforms(to: .image) == true
    }

    var body: some View {
        Group {
            if image != nil || (expectsImage && !loaded) {
                ZStack {
                    RoundedRectangle(cornerRadius: 16).fill(Palette.surface)
                    if let image {
                        Image(decorative: image, scale: displayScale)
                            .resizable()
                            .scaledToFit()
                    } else {
                        Image(systemName: "photo").foregroundStyle(Palette.secondary)
                    }
                }
                .frame(width: size, height: size)
                .clipShape(RoundedRectangle(cornerRadius: 16))
                .accessibilityLabel("Image attachment: \(file.name)")
                .onTapGesture {
                    guard image != nil else { return }
                    preview = true
                }
                .overlay(alignment: .topTrailing) {
                    removeButton
                        .foregroundStyle(.white)
                        .background(.black.opacity(0.75), in: Circle())
                        .padding(4)
                }
                .codyncSheet(isPresented: $preview) {
                    if let image {
                        VStack(spacing: 12) {
                            ModalHeader { Text(file.name) }
                            Image(decorative: image, scale: displayScale)
                                .resizable()
                                .scaledToFit()
                                .frame(maxWidth: .infinity, maxHeight: .infinity)
                        }
                        .padding(20)
                    }
                }
            } else {
                HStack(spacing: 6) {
                    Image(systemName: AttachmentIcon.symbol(file.name))
                        .foregroundStyle(Palette.secondary)
                    Text(file.name)
                        .font(.footnote)
                        .foregroundStyle(Palette.text)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .frame(maxWidth: 160, alignment: .leading)
                    removeButton.foregroundStyle(Palette.secondary)
                }
                .padding(.leading, 10)
                .padding(.trailing, 4)
                .padding(.vertical, 4)
                .background(Palette.surface, in: Capsule())
            }
        }
        .task(id: file.id) {
            let data = file.data
            let pixels = Int(size * displayScale)
            let thumbnail = await Task.detached(priority: .userInitiated) {
                ImageData.thumbnail(data, maxPixels: pixels)
            }.value
            guard !Task.isCancelled else { return }
            image = thumbnail
            loaded = true
        }
    }

    private var removeButton: some View {
        Button(action: remove) {
            Image(systemName: "xmark")
                .font(.system(size: 11, weight: .bold))
                .frame(width: 28, height: 28)
                .contentShape(Circle())
        }
        .buttonStyle(PressScale())
        .accessibilityLabel("Remove \(file.name)")
        .help("Remove")
    }
}
