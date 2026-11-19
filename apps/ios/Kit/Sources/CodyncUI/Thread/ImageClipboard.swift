import UIKit

/// The picture itself on the clipboard, as the image its bytes decode to (desktop `copyPicture`).
enum ImageClipboard {
    /// Puts `data` on the pasteboard; false when it isn't a picture the pasteboard can hold.
    @MainActor static func copy(_ data: Data) -> Bool {
        guard let image = UIImage(data: data) else { return false }
        UIPasteboard.general.image = image
        return true
    }
}
