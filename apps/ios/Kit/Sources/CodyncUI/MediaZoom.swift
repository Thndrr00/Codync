import CodyncKit
import SwiftUI

/// Pinch and double-tap zoom for a picture or clip shown in a sheet. Fit is 1×; past that a drag
/// pans, and a second double-tap (or a bumped `fitRequest`) returns to fit. Uses SwiftUI gestures
/// only, and drops its spring animation with Reduce Motion.
/// Ported from OpenBot 0.70.0's media viewer gestures.
struct MediaZoom<Content: View>: View {
    @ViewBuilder var content: Content
    /// Bump from outside (the header's Fit button) to return to fit.
    var fitRequest: Int = 0
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var scale: CGFloat = 1
    @State private var pan: CGSize = .zero
    @State private var zoomStart: CGFloat = 1
    @State private var panStart: CGSize = .zero

    private let maximum: CGFloat = 4
    private let doubleTapScale: CGFloat = 2.5

    init(fitRequest: Int = 0, @ViewBuilder content: () -> Content) {
        self.fitRequest = fitRequest
        self.content = content()
    }

    var body: some View {
        GeometryReader { geo in
            content
                .scaleEffect(scale)
                .offset(pan)
                .contentShape(Rectangle())
                .gesture(magnify)
                .simultaneousGesture(drag(in: geo.size))
                .onTapGesture(count: 2, coordinateSpace: .local) { point in
                    if scale > 1.001 {
                        fit()
                    } else {
                        withAnimation(Motion.reduced(Motion.layout, reduceMotion)) {
                            scale = doubleTapScale
                            zoomStart = scale
                            let center = CGPoint(x: geo.size.width / 2, y: geo.size.height / 2)
                            let shifted = CGSize(width: (center.x - point.x) * scale,
                                                 height: (center.y - point.y) * scale)
                            pan = clamped(shifted, in: geo.size)
                            panStart = pan
                        }
                    }
                }
                .accessibilityHint(L10n.tr("Double-tap to zoom"))
                .onChange(of: fitRequest) { _, _ in fit() }
        }
    }

    private var magnify: some Gesture {
        MagnifyGesture()
            .onChanged { value in
                let next = min(max(zoomStart * value.magnification, 1), maximum)
                scale = next
                if next <= 1.001 { pan = .zero; panStart = .zero }
            }
            .onEnded { _ in
                zoomStart = scale
                panStart = pan
                if scale <= 1.02 { fit() }
            }
    }

    private func drag(in bounds: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 1)
            .onChanged { value in
                guard scale > 1.001 else { return }
                let moved = CGSize(width: panStart.width + value.translation.width,
                                   height: panStart.height + value.translation.height)
                pan = clamped(moved, in: bounds)
            }
            .onEnded { _ in panStart = pan }
    }

    private func fit() {
        withAnimation(Motion.reduced(Motion.layout, reduceMotion)) {
            scale = 1
            pan = .zero
            zoomStart = 1
            panStart = .zero
        }
    }

    /// Keeps a zoomed picture from being dragged off the sheet.
    private func clamped(_ value: CGSize, in bounds: CGSize) -> CGSize {
        let x = max(0, bounds.width * (scale - 1) / 2)
        let y = max(0, bounds.height * (scale - 1) / 2)
        return CGSize(width: min(max(value.width, -x), x), height: min(max(value.height, -y), y))
    }
}
