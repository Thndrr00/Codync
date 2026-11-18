import CodyncKit
import SwiftUI

/// A small fact that belongs to a row (a count, a usage limit): quiet text on a capsule, so it
/// reads beside the row's own name instead of competing with it.
struct Chip: View {
    let text: String

    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text)
            .font(.caption2)
            .foregroundStyle(Palette.secondary)
            .lineLimit(1)
            .padding(.horizontal, 7)
            .padding(.vertical, 2)
            .background(Palette.surface, in: Capsule())
    }
}
