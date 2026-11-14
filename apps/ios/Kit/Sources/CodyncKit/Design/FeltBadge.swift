import SwiftUI

/// Felt badges: the warm, soft look of OpenBot's sidebar, drawn with Codync's own shapes and
/// gradients. The amber ball marks a bot that needs you (its cream yarn mark), and a count sits in
/// a felt capsule. Both keep the same background ring as the status dots they replace.
public enum Felt {
    public static let amberTop = Color(hex: 0xF5B54E)
    public static let amberBottom = Color(hex: 0xC97A0E)
    public static let cream = Color(hex: 0xFFF4DA)
}

/// A small yarn mark: two crossing threads, drawn rather than a bitmap.
struct YarnMark: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let w = rect.width
        let h = rect.height
        path.move(to: CGPoint(x: 0.16 * w, y: 0.34 * h))
        path.addCurve(
            to: CGPoint(x: 0.84 * w, y: 0.64 * h),
            control1: CGPoint(x: 0.34 * w, y: 0.02 * h),
            control2: CGPoint(x: 0.66 * w, y: 0.98 * h)
        )
        path.move(to: CGPoint(x: 0.2 * w, y: 0.72 * h))
        path.addCurve(
            to: CGPoint(x: 0.8 * w, y: 0.3 * h),
            control1: CGPoint(x: 0.4 * w, y: 0.96 * h),
            control2: CGPoint(x: 0.6 * w, y: 0.04 * h)
        )
        return path
    }
}

/// The amber felt ball with its cream yarn mark: a bot that needs you.
public struct FeltBall: View {
    let size: CGFloat

    public init(size: CGFloat) {
        self.size = size
    }

    public var body: some View {
        Circle()
            .fill(
                RadialGradient(
                    colors: [Felt.amberTop, Felt.amberBottom],
                    center: .topLeading,
                    startRadius: 0,
                    endRadius: size
                )
            )
            .overlay(alignment: .topLeading) {
                Circle()
                    .fill(Felt.cream.opacity(0.35))
                    .frame(width: size * 0.42, height: size * 0.42)
                    .blur(radius: size * 0.06)
                    .offset(x: size * 0.1, y: size * 0.08)
            }
            .overlay {
                YarnMark()
                    .stroke(Felt.cream, style: StrokeStyle(lineWidth: max(1, size * 0.11), lineCap: .round))
                    .padding(size * 0.18)
            }
            .frame(width: size, height: size)
            .overlay(Circle().stroke(Palette.background, lineWidth: 2))
            .accessibilityHidden(true)
    }
}

/// An unread count in a felt capsule: the accent ink with a soft top light and the same ring.
public struct FeltCountBadge: View {
    let count: Int

    public init(count: Int) {
        self.count = count
    }

    public var body: some View {
        Text("\(count)")
            .font(.caption2.bold())
            .foregroundStyle(Palette.onAccent)
            .padding(.horizontal, 6)
            .frame(minWidth: 18, minHeight: 18)
            .background(
                LinearGradient(
                    colors: [Palette.accentFill.opacity(0.92), Palette.accentFill.opacity(0.72)],
                    startPoint: .top,
                    endPoint: .bottom
                ),
                in: Capsule()
            )
            .overlay {
                Capsule()
                    .fill(Color.white.opacity(0.16))
                    .frame(height: 6)
                    .padding(.horizontal, 4)
                    .frame(maxHeight: .infinity, alignment: .top)
                    .padding(.top, 2)
                    .allowsHitTesting(false)
            }
            .overlay(Capsule().strokeBorder(Palette.border.opacity(0.7), lineWidth: 0.5))
    }
}
