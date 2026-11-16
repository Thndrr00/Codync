import Foundation

/// One of Codync's own gallery cards (host catalogue version 1): a catalogue name plus props the host
/// checked. The app draws the name with its own controls; nothing here is markup, a script or a
/// component from the outside.
public struct ComponentCard: Codable, Hashable, Sendable {
    public var version: Int
    /// metric | comparison | timeline | table | checklist | form
    public var name: String
    public var title: String?
    public var props: ComponentProps
    /// The data function the host resolved some props from, when it did.
    public var source: String?

    public init(version: Int = 1, name: String, title: String? = nil, props: ComponentProps, source: String? = nil) {
        self.version = version
        self.name = name
        self.title = title
        self.props = props
        self.source = source
    }
}

/// A card's content, by component. Every field is optional because each card uses its own.
public struct ComponentProps: Codable, Hashable, Sendable {
    /// metric
    public var label: String?
    public var value: String?
    public var change: String?
    /// up | down | flat
    public var trend: String?
    public var series: [Double]?
    /// comparison and table
    public var columns: [String]?
    public var rows: [[JSONValue]]?
    /// timeline
    public var events: [ComponentEvent]?
    /// checklist
    public var items: [ComponentItem]?
    /// form
    public var fields: [BlockField]?
    public var submit: String?
    /// Every card.
    public var caption: String?

    public init() {}
}

/// One timeline point.
public struct ComponentEvent: Codable, Hashable, Sendable, Identifiable {
    public var at: String
    public var title: String
    public var detail: String?
    /// info | warning | done
    public var tone: String?

    public var id: String { "\(at)·\(title)" }
}

/// One checklist row.
public struct ComponentItem: Codable, Hashable, Sendable, Identifiable {
    public var label: String
    public var done: Bool
    public var note: String?

    public var id: String { label }
}

/// A card's name as the person's language shows it.
public enum ComponentName {
    public static func label(_ name: String) -> String {
        switch name {
        case "metric": L10n.tr("Metric")
        case "comparison": L10n.tr("Comparison")
        case "timeline": L10n.tr("Timeline")
        case "table": L10n.tr("Table")
        case "checklist": L10n.tr("Checklist")
        case "form": L10n.tr("Form")
        default: L10n.tr("Card")
        }
    }
}

/// A form card's answers, built from what the person typed, in the shape the host checks.
public enum ComponentAnswers {
    public static func initial(_ card: ComponentCard) -> [String: JSONValue] {
        var values: [String: JSONValue] = [:]
        for field in card.props.fields ?? [] {
            values[field.id] = field.type == "toggle" ? .bool(false) : .string("")
        }
        return values
    }

    /// Blank values are left out, so the host can require them.
    public static func answers(_ card: ComponentCard, _ values: [String: JSONValue]) -> [String: JSONValue] {
        var answers: [String: JSONValue] = [:]
        for field in card.props.fields ?? [] {
            let value = values[field.id] ?? .null
            switch field.type {
            case "number":
                if let number = value.number {
                    answers[field.id] = .number(number)
                } else if let text = value.string, let number = Double(text.replacingOccurrences(of: ",", with: ".")) {
                    answers[field.id] = .number(number)
                }
            case "toggle":
                answers[field.id] = .bool(value.bool ?? false)
            default:
                if let text = value.string?.trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty {
                    answers[field.id] = .string(text)
                }
            }
        }
        return answers
    }

    public static func complete(_ card: ComponentCard, _ values: [String: JSONValue]) -> Bool {
        (card.props.fields ?? []).allSatisfy { field in
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

/// The gallery the host serves: the catalogue with each card's schema, a sample and whether a bot may
/// use it, the saved cards, and each bot's withhold list.
public struct ComponentGallery: Codable, Hashable, Sendable {
    public var version: Int
    public var components: [CatalogueComponent]
    public var instances: [ComponentInstance]
    public var bots: [ComponentBot]
}

public struct CatalogueComponent: Codable, Hashable, Sendable, Identifiable {
    public var name: String
    public var version: Int
    public var summary: String
    public var data: [String]
    public var sample: ComponentProps
    /// Whether the bot the gallery was read for may use it.
    public var available: Bool?

    public var id: String { name }
}

/// A card the playground saved. The iPhone lists them and may withhold them; it does not author them.
public struct ComponentInstance: Codable, Hashable, Sendable, Identifiable {
    public var id: String
    public var name: String
    public var title: String
    public var props: ComponentProps
    public var published: Bool
    public var valid: Bool?
}

public struct ComponentBot: Codable, Hashable, Sendable, Identifiable {
    public var id: String
    public var name: String
    /// agent | group
    public var kind: String
    public var withheld: [String]
}
