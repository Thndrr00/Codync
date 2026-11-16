import CodyncKit
import SwiftUI

/// A declarative UI block the bot shows (card K3): the host validated it against Codync's catalogue
/// and posted it as an approval card, so answering uses the same path as every other approval.
struct DeclarativeCard: View {
    let entry: Entry
    /// The option whose answer is on its way to the computer, if any.
    var answering: String?
    let respond: (String?, [String: JSONValue]?) -> Void
    @State private var values: [String: JSONValue] = [:]

    private var block: DeclarativeBlock? { entry.data.block }
    private var pending: Bool { entry.data.status == "pending" }

    var body: some View {
        if let block {
            CardSection(block.title) {
                ForEach(Array(block.body.enumerated()), id: \.offset) { item in
                    row(item.element)
                }
                if pending, !block.actions.isEmpty {
                    if block.fields.contains(where: \.isRequired), !BlockAnswers.complete(block, values) {
                        Text(L10n.tr("Fill in the required fields to continue."))
                            .font(.caption)
                            .foregroundStyle(Palette.tertiary)
                    }
                    actionRow(block)
                } else if !pending, !block.actions.isEmpty {
                    Text(L10n.tr(entry.data.status == "answered" ? "Answers sent" : "Cancel"))
                        .font(.caption)
                        .foregroundStyle(Palette.secondary)
                }
            }
            .frame(maxWidth: 420, alignment: .leading)
            .padding(.trailing, 40)
            .onAppear {
                if values.isEmpty { values = entry.data.values ?? BlockAnswers.initial(block) }
            }
        }
    }

    @ViewBuilder
    private func row(_ component: BlockComponent) -> some View {
        switch component {
        case let .heading(text):
            Text(text).font(.headline).foregroundStyle(Palette.text)
        case let .text(text):
            Text(text).font(.callout).foregroundStyle(Palette.secondary).fixedSize(horizontal: false, vertical: true)
        case .divider:
            Hairline()
        case let .callout(tone, text):
            HStack(alignment: .top, spacing: 8) {
                Image(systemName: tone == "warning" ? "exclamationmark.triangle.fill" : "info.circle.fill")
                    .font(.caption)
                    .foregroundStyle(tone == "warning" ? Palette.warning : Palette.accent)
                Text(text).font(.footnote).fixedSize(horizontal: false, vertical: true)
            }
        case let .field(field):
            fieldRow(field)
        case let .table(columns, rows):
            table(columns: columns, rows: rows)
        case let .chart(chartType, labels, points):
            chart(chartType: chartType, labels: labels, values: points)
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
                    ChoicePicker(selection: selection(field), options: (field.options ?? []).map { (id: $0.value, label: $0.label) })
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

    private func table(columns: [String], rows: [[JSONValue]]) -> some View {
        ScrollView(.horizontal, showsIndicators: false) {
            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                GridRow {
                    ForEach(columns.indices, id: \.self) { column in
                        Text(columns[column]).font(.caption.weight(.semibold)).foregroundStyle(Palette.secondary)
                    }
                }
                ForEach(rows.indices, id: \.self) { index in
                    GridRow {
                        ForEach(rows[index].indices, id: \.self) { column in
                            Text(cell(rows[index][column])).font(.caption).foregroundStyle(Palette.text)
                        }
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func chart(chartType: String, labels: [String], values points: [Double]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            if chartType == "line" {
                Canvas { context, size in
                    guard points.count > 1 else { return }
                    let peak = max(points.map(abs).max() ?? 1, 1)
                    var path = Path()
                    for (index, point) in points.enumerated() {
                        let x = size.width * CGFloat(index) / CGFloat(points.count - 1)
                        let y = size.height - (size.height - 6) * CGFloat(abs(point) / peak)
                        if index == 0 { path.move(to: CGPoint(x: x, y: y)) } else { path.addLine(to: CGPoint(x: x, y: y)) }
                    }
                    context.stroke(path, with: .color(Palette.accent), lineWidth: 2)
                }
                .frame(height: 76)
            } else {
                HStack(alignment: .bottom, spacing: 8) {
                    ForEach(points.indices, id: \.self) { index in
                        VStack(spacing: 4) {
                            RoundedRectangle(cornerRadius: 4, style: .continuous)
                                .fill(Palette.accent.opacity(0.85))
                                .frame(width: 22, height: max(4, 64 * abs(points[index]) / max(points.map(abs).max() ?? 1, 1)))
                            Text(labels.indices.contains(index) ? labels[index] : "")
                                .font(.caption2)
                                .foregroundStyle(Palette.tertiary)
                                .lineLimit(1)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            HStack(spacing: 8) {
                ForEach(labels.indices, id: \.self) { index in
                    Text(labels[index]).font(.caption2).foregroundStyle(Palette.tertiary).lineLimit(1)
                }
            }
        }
    }

    private func actionRow(_ block: DeclarativeBlock) -> some View {
        HStack(spacing: 8) {
            ForEach(block.actions) { action in
                actionButton(action, block: block)
            }
            if !block.actions.contains(where: { $0.kind == "destructive" }) {
                Button(L10n.tr("Cancel")) { respond(nil, nil) }
                    .buttonStyle(.secondary)
                    .disabled(answering != nil)
            }
        }
    }

    @ViewBuilder
    private func actionButton(_ action: BlockAction, block: DeclarativeBlock) -> some View {
        let send = { respond(action.id, block.fields.isEmpty ? nil : BlockAnswers.answers(block, values)) }
        if action.kind == "primary" {
            Button(action.label, action: send)
                .buttonStyle(.primary)
                .disabled(answering != nil || !BlockAnswers.complete(block, values))
        } else {
            Button(action.label, action: send)
                .buttonStyle(.secondary)
                .disabled(answering != nil || !BlockAnswers.complete(block, values))
        }
    }

    private func text(_ field: BlockField) -> Binding<String> {
        Binding(
            get: {
                if let text = values[field.id]?.string { return text }
                if let number = values[field.id]?.number { return String(number) }
                return ""
            },
            set: { values[field.id] = .string($0) }
        )
    }

    private func toggle(_ field: BlockField) -> Binding<Bool> {
        Binding(get: { values[field.id]?.bool ?? false }, set: { values[field.id] = .bool($0) })
    }

    private func selection(_ field: BlockField) -> Binding<String> {
        Binding(
            get: { values[field.id]?.string ?? field.options?.first?.value ?? "" },
            set: { values[field.id] = .string($0) }
        )
    }

    private func cell(_ value: JSONValue) -> String {
        switch value {
        case .null: return ""
        case let .string(text): return text
        case let .number(number): return number.formatted()
        case let .bool(on): return on ? "✓" : "—"
        case let .array(items): return items.compactMap(\.string).joined(separator: ", ")
        case let .object(fields): return fields.keys.sorted().joined(separator: ", ")
        }
    }
}
