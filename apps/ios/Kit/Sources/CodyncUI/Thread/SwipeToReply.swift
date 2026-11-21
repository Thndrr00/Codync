import CodyncKit
import SwiftUI

/// A horizontal swipe on a chat bubble quotes it for a reply, with the same 64-point threshold the
/// Android row uses. The bubble follows the finger a little and springs back; with Reduce Motion the
/// return is instant. Vertical scroll and text selection keep the gesture as soon as they win.
struct SwipeToReply: ViewModifier {
    let reply: (() -> Void)?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var dragged: CGFloat = 0

    /// Matches Android's 64 dp, so both phones commit a reply after the same travel.
    private static let threshold: CGFloat = 64

    func body(content: Content) -> some View {
        content
            .offset(x: dragged)
            .gesture(
                DragGesture(minimumDistance: 20)
                    .onChanged { value in
                        guard abs(value.translation.width) > abs(value.translation.height) else { return }
                        dragged = max(-Self.threshold, min(Self.threshold, value.translation.width))
                    }
                    .onEnded { value in
                        let committed = abs(value.translation.width) >= Self.threshold
                        withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { dragged = 0 }
                        if committed { reply?() }
                    }
            )
    }
}

extension View {
    /// Reuses a message row's reply action; nil where the message can't be replied to.
    func swipeToReply(_ reply: (() -> Void)?) -> some View {
        modifier(SwipeToReply(reply: reply))
    }
}
