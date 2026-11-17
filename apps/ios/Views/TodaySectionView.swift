import CodyncUI
import SwiftUI

/// The host's brief for today, above the board: what is due, what waits for an answer, today's
/// calendar and what the person is waiting on.
struct TodaySectionView: View {
    let store: BotStore
    @State private var digest: DailyDigest?
    @State private var weather: WeatherReport?
    @State private var place = ""
    @State private var editingPlace = false
    @State private var weatherError: String?
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        CardSection("Today", footer: "Read from this computer's board, approvals, calendars and waiting items. No model is asked.") {
            if let digest {
                Text(digest.text)
                    .font(.callout)
                    .foregroundStyle(Palette.text)
                Text(parts(digest).joined(separator: " · "))
                    .font(.caption)
                    .foregroundStyle(Palette.secondary)
                    .opacity(parts(digest).isEmpty ? 0 : 1)
            } else if !busy {
                Text("Nothing to show yet.").font(.caption).foregroundStyle(Palette.secondary)
            }
            if let weather {
                HStack(spacing: 8) {
                    Text(weather.text).font(.caption).foregroundStyle(Palette.secondary)
                    Spacer(minLength: 0)
                    Button("Change place") { editingPlace = true }
                        .buttonStyle(.plain)
                        .font(.caption)
                        .foregroundStyle(Palette.secondary)
                }
            }
            if weather == nil || editingPlace {
                TextField("Weather place (Madrid)", text: $place).plainTextInput()
                HStack {
                    Spacer()
                    Button("Save place") { savePlace() }
                        .buttonStyle(.secondary)
                        .disabled(place.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
            if let weatherError, weather == nil {
                Text(weatherError).font(.caption).foregroundStyle(Palette.tertiary)
            }
            HStack {
                Spacer()
                Button(busy ? "Reading…" : "Refresh") { load() }
                    .buttonStyle(.secondary)
                    .disabled(busy)
            }
            if let error {
                Text(error).font(.caption).foregroundStyle(Palette.danger)
            }
        }
        .task { load() }
    }

    private func load() {
        busy = true
        Task {
            do {
                digest = try await store.dailyDigest()
                error = nil
            } catch { self.error = message(error) }
            do {
                weather = try await store.weather()
                weatherError = nil
            } catch { weather = nil; weatherError = message(error) }
            busy = false
        }
    }

    private func savePlace() {
        let wanted = place.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !wanted.isEmpty else { return }
        Task {
            do {
                weather = try await store.setWeatherPlace(wanted)
                place = ""
                editingPlace = false
                weatherError = nil
            } catch { weatherError = message(error) }
        }
    }

    private func parts(_ digest: DailyDigest) -> [String] {
        var parts: [String] = []
        if !digest.overdue.isEmpty { parts.append("\(digest.overdue.count) overdue") }
        if !digest.dueToday.isEmpty { parts.append("\(digest.dueToday.count) due today") }
        if !digest.approvals.isEmpty { parts.append("\(digest.approvals.count) approvals") }
        if !digest.events.isEmpty { parts.append("\(digest.events.count) events") }
        if !digest.waiting.isEmpty { parts.append("\(digest.waiting.count) waiting for") }
        return parts
    }

    private func message(_ error: Error) -> String {
        (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
}
