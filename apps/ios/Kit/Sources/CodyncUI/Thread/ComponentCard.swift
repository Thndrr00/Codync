import CodyncKit
import SwiftUI

/// One of Codync's own gallery cards in a chat (host catalogue version 1): a `permission` entry with
/// `toolKind: "component"`. A `form` card's answers go back through the same path as any approval.
struct ComponentCardView: View {
    let entry: Entry
    /// The option whose answer is on its way to the computer, if any.
    var answering: String?
    let respond: (String?, [String: JSONValue]?) -> Void
    @State private var values: [String: JSONValue] = [:]

    private var card: ComponentCard? { entry.data.component }
    private var pending: Bool { entry.data.status == "pending" }

    var body: some View {
        if let card {
            CardSection(card.title ?? ComponentName.label(card.name)) {
                ComponentCardBody(card: card, pending: pending, values: $values,
                                  answering: answering, respond: respond)
            }
            .frame(maxWidth: 420, alignment: .leading)
            .padding(.trailing, 40)
            .onAppear {
                if values.isEmpty { values = entry.data.values ?? ComponentAnswers.initial(card) }
            }
        }
    }
}

/// A card's content, drawn from its catalogue name: the same view serves the chat and the gallery's
/// preview, so the two can never drift apart.
struct ComponentCardBody: View {
    let card: ComponentCard
    var pending = false
    @Binding var values: [String: JSONValue]
    /// The option whose answer is on its way, and how to send one; none in a preview.
    var answering: String? = nil
    var respond: ((String?, [String: JSONValue]?) -> Void)? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let caption = card.props.caption {
                Text(caption).font(.caption).foregroundStyle(Palette.secondary)
            }
            content
            if card.source != nil {
                Label(L10n.tr("Filled by the computer"), systemImage: "desktopcomputer")
                    .font(.caption2)
                    .foregroundStyle(Palette.tertiary)
            }
            if pending, let fields = card.props.fields, !fields.isEmpty {
                if !ComponentAnswers.complete(card, values) {
                    Text(L10n.tr("Fill in the required fields to continue."))
                        .font(.caption)
                        .foregroundStyle(Palette.tertiary)
                }
                HStack(spacing: 8) {
                    Button(card.props.submit ?? L10n.tr("Send")) {
                        respond?("submit", ComponentAnswers.answers(card, values))
                    }
                    .buttonStyle(.primary)
                    .disabled(answering != nil || !ComponentAnswers.complete(card, values))
                    Button(L10n.tr("Cancel")) { respond?(nil, nil) }
                        .buttonStyle(.secondary)
                        .disabled(answering != nil)
                }
            } else if pending, card.name == "form" {
                // A form with no fields asks nothing; the host refuses one, so this is unreachable
                // unless a newer catalogue sent it.
                EmptyView()
            }
        }
    }

    @ViewBuilder
    private var content: some View {
        switch card.name {
        case "metric":
            metric
        case "comparison":
            grid(comparison: true)
        case "timeline":
            timeline
        case "table":
            if (card.props.rows ?? []).isEmpty { empty } else { grid(comparison: false) }
        case "checklist":
            if (card.props.items ?? []).isEmpty { empty } else { checklist }
        case "form":
            fields
        default:
            // The catalogue is versioned with the app: a card this build doesn't know draws nothing.
            EmptyView()
        }
    }

    private var empty: some View {
        Text(L10n.tr("Nothing to show yet.")).font(.caption).foregroundStyle(Palette.tertiary)
    }

    private var metric: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(card.props.label ?? "").font(.caption.weight(.medium)).foregroundStyle(Palette.secondary)
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(card.props.value ?? "").font(.title2.weight(.semibold)).monospacedDigit()
                if let change = card.props.change {
                    Text("\(arrow) \(change)")
                        .font(.footnote)
                        .foregroundStyle(card.props.trend == "down" ? Palette.attention : Palette.secondary)
                }
            }
            if let series = card.props.series, series.count > 1 {
                Sparkline(values: series).frame(height: 40)
            }
        }
    }

    private var arrow: String {
        switch card.props.trend {
        case "up": "↑"
        case "down": "↓"
        default: "→"
        }
    }

    private func grid(comparison: Bool) -> some View {
        let columns = card.props.columns ?? []
        let rows = card.props.rows ?? []
        return ScrollView(.horizontal, showsIndicators: false) {
            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                GridRow {
                    ForEach(columns.indices, id: \.self) { column in
                        Text(columns[column]).font(.caption.weight(.semibold)).foregroundStyle(Palette.secondary)
                    }
                }
                ForEach(rows.indices, id: \.self) { index in
                    GridRow {
                        ForEach(rows[index].indices, id: \.self) { column in
                            Text(cell(rows[index][column]))
                                .font(.caption)
                                .foregroundStyle(comparison && column == 0 ? Palette.secondary : Palette.text)
                        }
                    }
                }
            }
        }
    }

    private var timeline: some View {
        let events = card.props.events ?? []
        return VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(events.enumerated()), id: \.offset) { item in
                HStack(alignment: .top, spacing: 10) {
                    VStack(spacing: 0) {
                        Circle().fill(dot(item.element.tone)).frame(width: 7, height: 7).padding(.top, 5)
                        if item.offset < events.count - 1 {
                            Rectangle().fill(Palette.border).frame(width: 1).frame(maxHeight: .infinity)
                        }
                    }
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(spacing: 8) {
                            Text(item.element.at).font(.caption).foregroundStyle(Palette.tertiary)
                            Text(item.element.title).font(.callout.weight(.medium))
                        }
                        if let detail = item.element.detail {
                            Text(detail)
                                .font(.footnote)
                                .foregroundStyle(Palette.secondary)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                    .padding(.bottom, 12)
                    Spacer(minLength: 0)
                }
            }
        }
    }

    private func dot(_ tone: String?) -> Color {
        switch tone {
        case "warning": Palette.warning
        case "done": Palette.added
        default: Palette.accent
        }
    }

    private var checklist: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(card.props.items ?? []) { item in
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: item.done ? "checkmark.circle.fill" : "circle")
                        .font(.caption)
                        .foregroundStyle(item.done ? Palette.accent : Palette.secondary)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(item.label)
                            .font(.callout)
                            .foregroundStyle(item.done ? Palette.secondary : Palette.text)
                            .strikethrough(item.done, color: Palette.border)
                        if let note = item.note {
                            Text(note).font(.caption).foregroundStyle(Palette.tertiary)
                        }
                    }
                    Spacer(minLength: 0)
                }
            }
        }
    }

    private var fields: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(card.props.fields ?? []) { field in
                fieldRow(field)
            }
        }
    }

    @ViewBuilder
    private func fieldRow(_ field: BlockField) -> some View {
        if field.type == "toggle" {
            Toggle(isOn: toggle(field)) { Text(field.label) }
                .toggleStyle(.codync)
                .disabled(!pending)
        } else {
            Field(field.label + (field.isRequired ? " *" : "")) {
                switch field.type {
                case "select":
                    ChoicePicker(selection: text(field),
                                 options: (field.options ?? []).map { (id: $0.value, label: $0.label) })
                        .disabled(!pending)
                case "multiline":
                    TextField(field.placeholder ?? field.label, text: text(field), axis: .vertical)
                        .lineLimit(2...6)
                        .fieldBox()
                        .disabled(!pending)
                default:
                    TextField(field.placeholder ?? field.label, text: text(field))
                        .keyboardType(field.type == "number" ? .decimalPad : .default)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .fieldBox()
                        .disabled(!pending)
                }
            }
        }
    }

    private func text(_ field: BlockField) -> Binding<String> {
        Binding(get: { values[field.id]?.string ?? "" }, set: { values[field.id] = .string($0) })
    }

    private func toggle(_ field: BlockField) -> Binding<Bool> {
        Binding(get: { values[field.id]?.bool ?? false }, set: { values[field.id] = .bool($0) })
    }

    private func cell(_ value: JSONValue) -> String {
        switch value {
        case .null: ""
        case let .string(text): text
        case let .number(number): number.formatted()
        case let .bool(on): on ? "✓" : ""
        case let .array(items): items.compactMap(\.string).joined(separator: " ")
        case .object: ""
        }
    }
}

/// A metric's sparkline: a quiet line under the number, in the theme's accent.
private struct Sparkline: View {
    let values: [Double]

    var body: some View {
        let peak = max(values.map(abs).max() ?? 1, 1)
        Canvas { context, size in
            var path = Path()
            for (index, value) in values.enumerated() {
                let x = values.count > 1 ? size.width * CGFloat(index) / CGFloat(values.count - 1) : 0
                let y = size.height - size.height * CGFloat(abs(value) / peak) * 0.9
                if index == 0 {
                    path.move(to: CGPoint(x: x, y: y))
                } else {
                    path.addLine(to: CGPoint(x: x, y: y))
                }
            }
            context.stroke(path, with: .color(Palette.accent), lineWidth: 1.5)
        }
        .accessibilityLabel(values.map { $0.formatted() }.joined(separator: ", "))
    }
}
