import CodyncKit
import CodyncUI
import SwiftUI
import UIKit

struct AppearanceSettings: View {
    @AppStorage(Appearance.storageKey) private var appearance = Appearance.system
    @AppStorage(AvatarStyle.storageKey, store: UserDefaults(suiteName: SharedStore.appGroup)) private var avatarStyle = AvatarStyle.plush
    @AppStorage(ChatAppearance.colourKey) private var chatColour = ""
    @AppStorage(ChatAppearance.backgroundKey) private var chatBackground = ""
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        CardSection("Appearance", footer: "Only changes appearance on this device.") {
            ValueRow("Theme") {
                ChoicePicker(selection: $appearance, options: Appearance.allCases.map { (id: $0, label: $0.label) })
            }
            ValueRow("Bot avatars") {
                ChoicePicker(selection: $avatarStyle, options: AvatarStyle.allCases.map { (id: $0, label: $0.label) })
            }
            AgentLogosToggle()
        }
        CardSection("Chat", footer: "Only changes appearance on this device.") {
            ChatChoiceRow(
                title: "Chat colour",
                detail: "The colour of your own messages, for each theme.",
                ids: ChatColours.all.map(\.id),
                selected: colourChoice.id(dark: isDark),
                swatch: { id in ChatColours.colour(id).map { Color(hex: $0.hex(dark: isDark)) } },
                onSelect: { id in
                    let next = isDark ? ChatChoice(light: colourChoice.light, dark: id) : ChatChoice(light: id, dark: colourChoice.dark)
                    ChatAppearance.set(next, key: ChatAppearance.colourKey)
                    chatColour = UserDefaults.standard.string(forKey: ChatAppearance.colourKey) ?? ""
                }
            )
            ChatChoiceRow(
                title: "Chat background",
                detail: "The colour behind the chat, for each theme.",
                ids: ChatBackgrounds.all.map(\.id),
                selected: backgroundChoice.id(dark: isDark),
                swatch: { id in ChatBackgrounds.background(id).map { Color(hex: $0.hex(dark: isDark)) } },
                onSelect: { id in
                    let next = isDark ? ChatChoice(light: backgroundChoice.light, dark: id) : ChatChoice(light: id, dark: backgroundChoice.dark)
                    ChatAppearance.set(next, key: ChatAppearance.backgroundKey)
                    chatBackground = UserDefaults.standard.string(forKey: ChatAppearance.backgroundKey) ?? ""
                }
            )
        }
    }

    private var isDark: Bool { colorScheme == .dark }

    private var colourChoice: ChatChoice {
        ChatChoice.decode(chatColour, known: ChatColours.all.map(\.id))
    }

    private var backgroundChoice: ChatChoice {
        ChatChoice.decode(chatBackground, known: ChatBackgrounds.all.map(\.id))
    }
}

/// One row of the Chat section: the colours of the theme the window shows, and which one is chosen.
private struct ChatChoiceRow: View {
    let title: String
    let detail: String
    let ids: [String]
    let selected: String?
    let swatch: (String) -> Color?
    let onSelect: (String?) -> Void
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(L10n.tr(title)).foregroundStyle(Palette.text)
                Spacer()
                Text(L10n.tr(colorScheme == .dark ? "Dark" : "Light"))
                    .font(.caption)
                    .foregroundStyle(Palette.secondary)
            }
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 10) {
                    button(nil)
                    ForEach(ids, id: \.self) { id in button(id) }
                }
                .padding(.vertical, 2)
            }
            Text(L10n.tr(detail)).font(.caption).foregroundStyle(Palette.secondary)
        }
        .padding(.vertical, 6)
    }

    private func button(_ id: String?) -> some View {
        let isSelected = selected == id
        let fill = id.flatMap(swatch)
        return Button { onSelect(id) } label: {
            ZStack {
                Circle()
                    .fill(fill ?? Palette.bubbleUser)
                    .overlay {
                        if id == nil {
                            Image(systemName: "slash.circle").font(.caption).foregroundStyle(Palette.secondary)
                        }
                    }
                    .frame(width: 26, height: 26)
                Circle()
                    .strokeBorder(isSelected ? Palette.accent : Palette.border, lineWidth: isSelected ? 2 : 1)
                    .frame(width: 32, height: 32)
            }
            .frame(width: 34, height: 34)
            .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(id.map { L10n.tr($0.capitalized) } ?? L10n.tr("None"))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

/// Apply the device preference to the window so already-presented sheets update too.
struct AppearanceWindow: UIViewRepresentable {
    let appearance: Appearance

    func makeUIView(context: Context) -> AppearanceView { AppearanceView() }

    func updateUIView(_ view: AppearanceView, context: Context) {
        view.style = switch appearance {
        case .system: .unspecified
        case .light: .light
        case .dark: .dark
        }
    }

    final class AppearanceView: UIView {
        var style: UIUserInterfaceStyle = .unspecified {
            didSet { window?.overrideUserInterfaceStyle = style }
        }

        override func didMoveToWindow() {
            super.didMoveToWindow()
            window?.overrideUserInterfaceStyle = style
        }
    }
}
