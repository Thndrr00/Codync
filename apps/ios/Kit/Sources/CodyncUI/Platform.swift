import CodyncKit
import SwiftUI
import UIKit

enum Pasteboard {
    static func copy(_ text: String?) {
        guard let text else { return }
        UIPasteboard.general.string = text
    }
}

extension View {
    func plainTextInput() -> some View {
        textInputAutocapitalization(.never).autocorrectionDisabled()
    }
}

extension View {
    /// Liquid Glass where the OS has it (content scrolls visibly underneath), a material before that.
    /// With "Background blur" off, every one of these surfaces falls back to a plain fill.
    func glass(in shape: some Shape) -> some View {
        modifier(FrostedSurface(shape: shape, nativeGlass: true))
    }

    /// The frosted material on the remote-screen chrome; a plain fill with "Background blur" off.
    func frosted(in shape: some Shape) -> some View {
        modifier(FrostedSurface(shape: shape, nativeGlass: false))
    }
}

/// One hook for the device's "Background blur" preference: every frosted surface reads it here.
private struct FrostedSurface<S: Shape>: ViewModifier {
    @AppStorage(MotionPreference.backgroundBlurKey) private var backgroundBlur = true
    let shape: S
    /// Whether this surface uses the OS's native Liquid Glass when it is available.
    let nativeGlass: Bool

    func body(content: Content) -> some View {
        if !backgroundBlur {
            content.background(Palette.surface, in: shape)
        } else if #available(iOS 26, *), nativeGlass {
            content.glassEffect(.regular, in: shape)
        } else {
            content.background(.ultraThinMaterial, in: shape)
        }
    }
}
