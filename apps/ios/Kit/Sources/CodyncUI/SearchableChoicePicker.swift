import CodyncKit
import SwiftUI

/// Picks one value from a long list: shows the current choice in a pill, opens a sheet with a
/// search field and the matching choices. Use instead of `ChoicePicker` when the list is long.
/// The system `Menu` cannot host a search field, so this presents a `.codyncSheet`.
public struct SearchableChoicePicker<ID: Hashable>: View {
    @Binding var selection: ID
    let options: [(id: ID, label: String)]
    let title: String
    @Binding var query: String
    var fill: Color
    var fitsAvailableWidth: Bool
    @State private var presenting = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(selection: Binding<ID>, options: [(id: ID, label: String)], title: String, query: Binding<String>,
                fill: Color = .clear, fitsAvailableWidth: Bool = false) {
        _selection = selection
        self.options = options
        self.title = title
        _query = query
        self.fill = fill
        self.fitsAvailableWidth = fitsAvailableWidth
    }

    public var body: some View {
        Button {
            withAnimation(Motion.reduced(Motion.fade, reduceMotion)) {
                query = ""
                presenting = true
            }
        } label: {
            HStack(spacing: 6) {
                Text(options.first { $0.id == selection }?.label ?? "Choose").lineLimit(1)
                Image(systemName: "chevron.down").font(.caption2.weight(.semibold)).foregroundStyle(Palette.secondary)
            }
            .font(.body)
            .foregroundStyle(Palette.text)
            .pill(fill: fill)
            .overlay {
                if fill == .clear { RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Palette.border) }
            }
        }
        .buttonStyle(.plain)
        .fixedSize(horizontal: !fitsAvailableWidth, vertical: true)
        .help(options.first { $0.id == selection }?.label ?? "Choose")
        .codyncSheet(isPresented: $presenting) {
            VStack(spacing: 0) {
                ModalHeader(title)
                // The search field stays pinned above the list so it is always reachable, matching
                // the desktop menu; only the choices scroll.
                VStack(alignment: .leading, spacing: 0) {
                    SearchField("Search", text: $query)
                        .padding(.horizontal, 20)
                        .padding(.bottom, 12)
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 22) {
                            if options.isEmpty {
                                Text("No matches.").foregroundStyle(Palette.secondary)
                            } else {
                                CardSection {
                                    ForEach(options, id: \.id) { option in
                                        Button {
                                            withAnimation(Motion.reduced(Motion.fade, reduceMotion)) {
                                                selection = option.id
                                                presenting = false
                                            }
                                        } label: {
                                            HStack(spacing: 10) {
                                                Text(option.label).foregroundStyle(Palette.text)
                                                Spacer(minLength: 8)
                                                Image(systemName: "checkmark")
                                                    .font(.caption.weight(.semibold))
                                                    .foregroundStyle(Palette.text)
                                                    .opacity(option.id == selection ? 1 : 0)
                                            }
                                            .contentShape(Rectangle())
                                        }
                                        .buttonStyle(.plain)
                                        .accessibilityAddTraits(option.id == selection ? .isSelected : [])
                                    }
                                }
                            }
                        }
                        .font(.body)
                        .padding(20)
                    }
                    .scrollDismissesKeyboard(.interactively)
                }
            }
            .background(Palette.background)
        }
    }
}
