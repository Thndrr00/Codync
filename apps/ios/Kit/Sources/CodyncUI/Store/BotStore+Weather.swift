import CodyncKit
import Foundation

/// Current weather for a place, from Open-Meteo (no key).
public struct WeatherReport: Decodable, Equatable, Sendable {
    public var place: String
    public var text: String
    public var temperature: Double?
    public var condition: String?
}

extension BotStore {
    private struct Body: Encodable {
        var place: String?
        var save: Bool
    }

    public func weather(place: String? = nil, save: Bool = false) async throws -> WeatherReport {
        let client = try await ready()
        return try await client.call(
            "weather",
            Body(place: place?.trimmingCharacters(in: .whitespacesAndNewlines), save: save),
            timeout: 30
        )
    }

    @discardableResult
    public func setWeatherPlace(_ place: String) async throws -> WeatherReport {
        try await weather(place: place, save: true)
    }
}
