import CodyncKit
import SwiftUI
import UIKit

/// A coding agent's logo, by ACP registry id or by Codync's id for agents outside the registry,
/// drawn in one colour: grey at rest, an accent while it is chosen; a terminal glyph otherwise.
public struct AgentIcon: View {
    let registry: String?
    let size: CGFloat
    let tint: Color

    public init(registry: String?, size: CGFloat = 20, tint: Color = Palette.secondary) {
        self.registry = registry
        self.size = size
        self.tint = tint
    }

    public var body: some View {
        Group {
            if let name = registry.map({ "agent-\($0)" }), Self.exists(name) {
                Image(name, bundle: .module).resizable().scaledToFit()
            } else {
                Image(systemName: "terminal").resizable().scaledToFit().padding(size * 0.1)
            }
        }
        .frame(width: size, height: size)
        .foregroundStyle(tint)
    }

    /// The logo alone, for places that take an `Image` (the system menu).
    public static func image(registry: String?) -> Image {
        if let name = registry.map({ "agent-\($0)" }), exists(name) { Image(name, bundle: .module) } else { Image(systemName: "terminal") }
    }

    private static func exists(_ name: String) -> Bool {
        UIImage(named: name, in: .module, with: nil) != nil
    }
}
