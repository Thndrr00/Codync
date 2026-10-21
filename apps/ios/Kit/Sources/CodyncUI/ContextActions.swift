import SwiftUI

/// A row of emoji above a message's menu (Slack's quick reactions).
public struct ReactionPick {
    public var emoji: [String]
    public var chosen: [String]
    public var toggle: (String) -> Void

    public init(emoji: [String], chosen: [String], toggle: @escaping (String) -> Void) {
        self.emoji = emoji
        self.chosen = chosen
        self.toggle = toggle
    }
}

public extension View {
    /// Long-press opens the system context menu; `reactions` puts a quick-reaction row on top.
    func contextActions(reactions: ReactionPick? = nil, _ items: @escaping () -> [MenuItem]) -> some View {
        modifier(ContextActions(items: items, reactions: reactions))
    }

    func contextActions<Preview: View>(@ViewBuilder preview: () -> Preview, _ items: @escaping () -> [MenuItem]) -> some View {
        modifier(ContextActions(items: items, reactions: nil, preview: AnyView(preview())))
    }
}

private struct ContextActions: ViewModifier {
    let items: () -> [MenuItem]
    let reactions: ReactionPick?
    var preview: AnyView?

    func body(content: Content) -> some View {
        if let preview {
            content.contextMenu { menu } preview: { preview }
        } else {
            content.contextMenu { menu }
        }
    }

    @ViewBuilder private var menu: some View {
        if let reactions {
            ControlGroup {
                ForEach(reactions.emoji, id: \.self) { emoji in
                    let chosen = reactions.chosen.contains(emoji)
                    Toggle(isOn: Binding(get: { chosen }, set: { _ in reactions.toggle(emoji) })) { Text(emoji) }
                }
            }
            .controlGroupStyle(.palette)
        }
        ForEach(items()) { item in
            if item.divider { Divider() }
            systemMenuRow(item)
        }
    }
}

