import Foundation

/// Whether the reader has scrolled near enough to the top of a thread to load older messages
/// (OpenBot nightly t3-smooth-scroll-load-older). The applied snapshot keeps the first visible row
/// in place, so the messages being read do not move while the older page comes in. Pure so the
/// threshold can be tested without a collection view.
enum ConversationNearTop {
    /// The top quarter of the viewport, generous enough to start loading before the row appears.
    static let fraction: CGFloat = 0.25

    static func isNear(offset: CGFloat, inset: CGFloat, height: CGFloat) -> Bool {
        guard height > 0 else { return false }
        return offset + inset < height * fraction
    }
}
