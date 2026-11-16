import CodyncKit
import SwiftUI
import UniformTypeIdentifiers

/// A local attachment, shown before it is uploaded or sent. Its upload starts when it is attached: a
/// ring shows the bytes the computer has and cancels it, and a file that failed stays with its reason
/// and a Retry.
struct ComposerAttachment: View {
    let file: OutgoingFile
    let upload: FileUpload?
    let remove: () -> Void
    let cancel: () -> Void
    let retry: () -> Void
    @State private var image: CGImage?
    @State private var loaded = false
    @Environment(\.displayScale) private var displayScale

    private var size: CGFloat { 112 }
    private var expectsImage: Bool {
        UTType(filenameExtension: (file.name as NSString).pathExtension)?.conforms(to: .image) == true
    }
    private var showsImage: Bool { image != nil || (expectsImage && !loaded) }
    private var failed: Bool { upload?.hasFailed == true }

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Group {
                if showsImage {
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
                    .overlay(alignment: .topTrailing) {
                        removeButton
                            .foregroundStyle(.white)
                            .background(.black.opacity(0.75), in: Circle())
                            .padding(4)
                    }
                    .overlay(alignment: .bottomLeading) {
                        if let upload, upload.isUploading {
                            UploadRing(upload: upload, cancel: cancel)
                                .padding(6)
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
                        if let upload, upload.isUploading {
                            Text("\(upload.percent)%")
                                .font(.caption2.monospacedDigit())
                                .foregroundStyle(Palette.secondary)
                        }
                        if let upload, upload.isUploading {
                            UploadRing(upload: upload, cancel: cancel)
                        }
                        removeButton.foregroundStyle(Palette.secondary)
                    }
                    .padding(.leading, 10)
                    .padding(.trailing, 4)
                    .padding(.vertical, 4)
                    .background(Palette.surface, in: Capsule())
                }
            }
            if failed {
                HStack(spacing: 6) {
                    Text(L10n.tr(upload?.reason ?? "Upload failed"))
                        .font(.caption2)
                        .foregroundStyle(Palette.danger)
                        .lineLimit(1)
                    Button(L10n.tr("Retry"), action: retry)
                        .font(.caption2.weight(.semibold))
                        .buttonStyle(.plain)
                        .foregroundStyle(Palette.accent)
                }
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

/// The bytes the computer has of one file, around a stop mark: a tap stops that file's upload.
private struct UploadRing: View {
    let upload: FileUpload
    let cancel: () -> Void

    var body: some View {
        Button(action: cancel) {
            ZStack {
                Circle()
                    .stroke(.white.opacity(0.3), lineWidth: 2.5)
                Circle()
                    .trim(from: 0, to: max(0.02, Double(upload.percent) / 100))
                    .stroke(Palette.accent, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                    .animation(Motion.hover, value: upload.percent)
                Image(systemName: "xmark")
                    .font(.system(size: 7, weight: .bold))
                    .foregroundStyle(.white)
            }
            .frame(width: 20, height: 20)
            .contentShape(Circle())
        }
        .buttonStyle(PressScale())
        .accessibilityLabel(L10n.tr("Cancel upload"))
        .help("Cancel upload")
    }
}
