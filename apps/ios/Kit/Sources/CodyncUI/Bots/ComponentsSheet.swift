import CodyncKit
import SwiftUI

/// A computer's component gallery on the phone: the cards Codync can draw, a preview of each, and the
/// switch that withholds one from a bot. Authoring a card is the desktop's playground; a phone reads the
/// catalogue and decides who may use it.
public struct ComponentsSheet: View {
    @Environment(BotStore.self) private var model
    @State private var gallery: ComponentGallery?
    @State private var error: String?
    @State private var busy = false

    public init() {}

    public var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Components")
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text(L10n.tr("The cards Codync can draw in a chat. Turn one off and that bot can't show it."))
                        .font(.footnote)
                        .foregroundStyle(Palette.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                    if let error {
                        Text(error).font(.footnote).foregroundStyle(Palette.danger)
                    }
                    if let gallery {
                        ForEach(gallery.components) { component in
                            card(component, gallery: gallery)
                        }
                    } else if error == nil {
                        Text(L10n.tr("Loading…")).font(.footnote).foregroundStyle(Palette.tertiary)
                    }
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 16)
            }
        }
        .background(Palette.background)
        .task { await load() }
    }

    @ViewBuilder
    private func card(_ component: CatalogueComponent, gallery: ComponentGallery) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Text(ComponentName.label(component.name))
                    .font(.body.weight(.semibold))
                    .foregroundStyle(Palette.text)
                Spacer(minLength: 8)
                Text("v\(component.version)").font(.caption2).foregroundStyle(Palette.tertiary)
            }
            Text(L10n.tr(component.summary))
                .font(.caption)
                .foregroundStyle(Palette.secondary)
                .fixedSize(horizontal: false, vertical: true)
            // The host's own sample, drawn by the same view a card in the chat uses.
            ComponentCardBody(
                card: ComponentCard(name: component.name, props: component.sample),
                values: .constant([:])
            )
            if !component.data.isEmpty {
                Label(component.data.joined(separator: ", "), systemImage: "desktopcomputer")
                    .font(.caption2)
                    .foregroundStyle(Palette.tertiary)
            }
            if gallery.bots.count > 0 {
                Hairline()
                ForEach(gallery.bots) { bot in
                    Toggle(isOn: allows(bot, component.name)) {
                        Text(bot.name).font(.callout)
                    }
                    .toggleStyle(.codync)
                    .disabled(busy)
                }
            }
        }
        .padding(14)
        .background(Palette.bubbleAgent, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    /// A bot has a component unless the host says it is withheld.
    private func allows(_ bot: ComponentBot, _ component: String) -> Binding<Bool> {
        Binding(
            get: { !bot.withheld.contains(component) },
            set: { on in Task { await setAccess(bot.id, component, on) } }
        )
    }

    private func load() async {
        guard let client = model.client else { return }
        do {
            gallery = try await client.call("listComponents")
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func setAccess(_ bot: String, _ component: String, _ on: Bool) async {
        guard let client = model.client else { return }
        busy = true
        defer { busy = false }
        do {
            let body: [String: JSONValue] = [
                "bot": .string(bot),
                "component": .string(component),
                "published": .bool(on),
            ]
            let _: JSONValue = try await client.call("setComponentAccess", body)
            await load()
        } catch {
            self.error = error.localizedDescription
        }
    }
}
