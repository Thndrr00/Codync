import Foundation

/// A JSON value without a fixed shape: a block's table cells, a form's answers, a view's state.
public enum JSONValue: Codable, Hashable, Sendable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: any Decoder) throws {
        let single = try decoder.singleValueContainer()
        if single.decodeNil() {
            self = .null
        } else if let value = try? single.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? single.decode(Double.self) {
            self = .number(value)
        } else if let value = try? single.decode(String.self) {
            self = .string(value)
        } else if let value = try? single.decode([JSONValue].self) {
            self = .array(value)
        } else {
            self = .object(try single.decode([String: JSONValue].self))
        }
    }

    public func encode(to encoder: any Encoder) throws {
        var single = encoder.singleValueContainer()
        switch self {
        case .null: try single.encodeNil()
        case let .bool(value): try single.encode(value)
        case let .number(value): try single.encode(value)
        case let .string(value): try single.encode(value)
        case let .array(value): try single.encode(value)
        case let .object(value): try single.encode(value)
        }
    }

    public var string: String? {
        if case let .string(value) = self { return value }
        return nil
    }

    public var bool: Bool? {
        if case let .bool(value) = self { return value }
        return nil
    }

    public var number: Double? {
        if case let .number(value) = self { return value }
        return nil
    }

    public var object: [String: JSONValue]? {
        if case let .object(value) = self { return value }
        return nil
    }
}

/// A declarative UI block a bot shows in its chat (card K3). The host checked it against Codync's
/// catalogue before it was posted; the app only ever draws what is in this shape.
public struct DeclarativeBlock: Codable, Hashable, Sendable {
    public var version: Int
    public var title: String
    public var body: [BlockComponent]
    public var actions: [BlockAction]

    /// The form's fields, in body order.
    public var fields: [BlockField] {
        body.compactMap { (component: BlockComponent) -> BlockField? in
            if case let .field(field) = component { return field }
            return nil
        }
    }
}

/// The action kinds a block may declare (`primary`, `secondary`, `destructive`).
public struct BlockAction: Codable, Hashable, Sendable, Identifiable {
    public var id: String
    public var label: String
    public var kind: String
}

public struct BlockFieldOption: Codable, Hashable, Sendable {
    public var value: String
    public var label: String
}

/// `text` | `multiline` | `number` | `date` | `time` | `select` | `toggle`.
public struct BlockField: Codable, Hashable, Sendable, Identifiable {
    public var id: String
    public var label: String
    public var type: String
    public var required: Bool?
    public var placeholder: String?
    public var options: [BlockFieldOption]?
    /// A starting value the agent suggested; the person may change it before sending.
    public var value: JSONValue?

    public var isRequired: Bool { required == true }
}

public enum BlockComponent: Codable, Hashable, Sendable {
    case heading(String)
    case text(String)
    case divider
    case callout(tone: String, text: String)
    case field(BlockField)
    case table(columns: [String], rows: [[JSONValue]])
    case chart(chartType: String, labels: [String], values: [Double])

    private enum CodingKeys: String, CodingKey {
        case type, text, tone, field, columns, rows, chartType, labels, values
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .type) {
        case "heading":
            self = .heading(try container.decode(String.self, forKey: .text))
        case "text":
            self = .text(try container.decode(String.self, forKey: .text))
        case "divider":
            self = .divider
        case "callout":
            self = .callout(
                tone: try container.decodeIfPresent(String.self, forKey: .tone) ?? "info",
                text: try container.decode(String.self, forKey: .text)
            )
        case "field":
            self = .field(try container.decode(BlockField.self, forKey: .field))
        case "table":
            self = .table(
                columns: try container.decode([String].self, forKey: .columns),
                rows: try container.decode([[JSONValue]].self, forKey: .rows)
            )
        case "chart":
            self = .chart(
                chartType: try container.decode(String.self, forKey: .chartType),
                labels: try container.decode([String].self, forKey: .labels),
                values: try container.decode([Double].self, forKey: .values)
            )
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type, in: container, debugDescription: "Unknown block component."
            )
        }
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case let .heading(text):
            try container.encode("heading", forKey: .type)
            try container.encode(text, forKey: .text)
        case let .text(text):
            try container.encode("text", forKey: .type)
            try container.encode(text, forKey: .text)
        case .divider:
            try container.encode("divider", forKey: .type)
        case let .callout(tone, text):
            try container.encode("callout", forKey: .type)
            try container.encode(tone, forKey: .tone)
            try container.encode(text, forKey: .text)
        case let .field(field):
            try container.encode("field", forKey: .type)
            try container.encode(field, forKey: .field)
        case let .table(columns, rows):
            try container.encode("table", forKey: .type)
            try container.encode(columns, forKey: .columns)
            try container.encode(rows, forKey: .rows)
        case let .chart(chartType, labels, values):
            try container.encode("chart", forKey: .type)
            try container.encode(chartType, forKey: .chartType)
            try container.encode(labels, forKey: .labels)
            try container.encode(values, forKey: .values)
        }
    }
}

/// The form's answers by field id, built from what the person filled in.
public enum BlockAnswers {
    public static func initial(_ block: DeclarativeBlock) -> [String: JSONValue] {
        var values: [String: JSONValue] = [:]
        for field in block.fields {
            values[field.id] = field.value ?? empty(field.type)
        }
        return values
    }

    private static func empty(_ type: String) -> JSONValue {
        switch type {
        case "toggle": return .bool(false)
        case "number": return .string("")
        default: return .string("")
        }
    }

    /// The answers to send: blank values are left out, so the host can require them.
    public static func answers(_ block: DeclarativeBlock, _ values: [String: JSONValue]) -> [String: JSONValue] {
        var answers: [String: JSONValue] = [:]
        for field in block.fields {
            let value = values[field.id] ?? .null
            switch field.type {
            case "number":
                if let number = value.number {
                    answers[field.id] = JSONValue.number(number)
                } else if let text = value.string, let number = Double(text.replacingOccurrences(of: ",", with: ".")) {
                    answers[field.id] = JSONValue.number(number)
                }
            case "toggle":
                answers[field.id] = JSONValue.bool(value.bool ?? false)
            default:
                if let text = value.string?.trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty {
                    answers[field.id] = JSONValue.string(text)
                }
            }
        }
        return answers
    }

    /// Whether every required field has an answer, so the card can enable its buttons.
    public static func complete(_ block: DeclarativeBlock, _ values: [String: JSONValue]) -> Bool {
        block.fields.allSatisfy { field in
            guard field.isRequired else { return true }
            let value = values[field.id] ?? .null
            switch field.type {
            case "toggle": return true
            case "number":
                if value.number != nil { return true }
                return value.string.flatMap { Double($0.replacingOccurrences(of: ",", with: ".")) } != nil
            default:
                return !(value.string?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true)
            }
        }
    }
}
