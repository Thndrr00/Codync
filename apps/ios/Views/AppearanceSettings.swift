import CodyncKit
import CodyncUI
import SwiftUI
import UIKit

struct AppearanceSettings: View {
    @AppStorage(Appearance.storageKey) private var appearance = Appearance.system
    @AppStorage(AvatarStyle.storageKey, store: UserDefaults(suiteName: SharedStore.appGroup)) private var avatarStyle = AvatarStyle.plush
    @AppStorage(YourBubbleStyle.storageKey) private var yourBubbles = YourBubbleStyle.solid
    @AppStorage(MotionPreference.backgroundBlurKey) private var backgroundBlur = true
    @AppStorage(MotionPreference.movingAgentFacesKey) private var movingAgentFaces = true
    @AppStorage(SearchMotionStyle.storageKey) private var searchMotion = SearchMotionStyle.fade

    var body: some View {
        CardSection("Appearance", footer: "Only changes appearance on this device.") {
            ValueRow("Theme") {
                ChoicePicker(selection: $appearance, options: Appearance.allCases.map { (id: $0, label: $0.label) })
            }
            ValueRow("Bot avatars") {
                ChoicePicker(selection: $avatarStyle, options: AvatarStyle.allCases.map { (id: $0, label: $0.label) })
            }
            VStack(alignment: .leading, spacing: 8) {
                Text(L10n.tr("Your bubbles"))
                    .foregroundStyle(Palette.text)
                SegmentedChoice(selection: $yourBubbles, options: YourBubbleStyle.allCases.map { (id: $0, label: $0.label) })
                    .onChange(of: yourBubbles) { _, style in Analytics.shared.capture(.bubbleStyleChanged(style)) }
            }
            AgentLogosToggle()
        }
        CardSection("Motion", footer: "Only changes appearance on this device.") {
            ValueRow(SearchMotionStyle.title) {
                ChoicePicker(selection: $searchMotion, options: searchMotionOptions())
            }
            HStack {
                Text(L10n.tr("Background blur")).foregroundStyle(Palette.text)
                Spacer()
                Toggle(isOn: $backgroundBlur) { EmptyView() }
                    .toggleStyle(.codync)
                    .accessibilityLabel(L10n.tr("Background blur"))
            }
            HStack {
                Text(L10n.tr("Moving agent faces")).foregroundStyle(Palette.text)
                Spacer()
                Toggle(isOn: $movingAgentFaces) { EmptyView() }
                    .toggleStyle(.codync)
                    .accessibilityLabel(L10n.tr("Moving agent faces"))
            }
        }
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
