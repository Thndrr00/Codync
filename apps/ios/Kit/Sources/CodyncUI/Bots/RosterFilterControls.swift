import CodyncKit
import SwiftUI

public struct RosterFilterControls: View {
    @Binding var query: String
    @Binding var filter: RosterFilter
    @Binding var byProject: Bool
    @Binding var showHistory: Bool
    @Binding var showLibrary: Bool

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(query: Binding<String>, filter: Binding<RosterFilter>, byProject: Binding<Bool>, showHistory: Binding<Bool>,
                showLibrary: Binding<Bool>) {
        _query = query
        _filter = filter
        _byProject = byProject
        _showHistory = showHistory
        _showLibrary = showLibrary
    }

    public var body: some View {
        VStack(spacing: 8) {
            SearchField(text: $query)
            HStack {
                DropdownMenu {
                    RosterFilter.allCases.map { value in
                        MenuItem(value.title, selected: filter == value) {
                            withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { filter = value }
                        }
                    }
                } label: { Text(filter.title).font(.caption) }
                Spacer()
                Button(byProject ? "By project" : "Manual order") { withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { byProject.toggle() } }
                    .buttonStyle(.secondary)
                Button { withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { showHistory = true } } label: { Image(systemName: "magnifyingglass").accessibilityLabel("Search history") }
                    .buttonStyle(.secondary)
                Button { showLibrary = true } label: { Image(systemName: "books.vertical").accessibilityLabel(L10n.tr("Server Library")) }
                    .buttonStyle(.secondary)
            }
        }.padding(.vertical, 8)
    }
}
