import CodyncKit
import SwiftUI

/// A search or a list with nothing in it: the felt plush above the text. The plush is a
/// working character, so it blinks and glances, and each client's Reduce Motion handling leaves
/// it a still image. The caller translates both strings, so a query can be filled in first.
struct MarketEmpty: View {
    let title: String
    var detail: String? = nil

    var body: some View {
        VStack(spacing: 8) {
            CharacterAvatar(shape: "blob", color: "violet", size: 64, mood: .working)
            Text(title)
                .font(.body.weight(.semibold))
                .foregroundStyle(Palette.text)
            if let detail {
                Text(detail)
                    .font(.subheadline)
                    .foregroundStyle(Palette.secondary)
            }
        }
        .multilineTextAlignment(.center)
        .frame(maxWidth: .infinity)
        .padding(.vertical, 32)
    }
}
