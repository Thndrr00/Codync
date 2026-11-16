import CodyncKit
import CodyncUI
import Foundation
import SwiftUI
import UIKit

/// The board, the saved teams and the read-only links this computer keeps. iPhone reach for
/// host/src/store/board.rs, chat/team_templates.rs and share.rs.
struct WorkspaceSheet: View {
    @Environment(BotStore.self) private var store
    @Environment(\.openURL) private var openURL
    @State private var tab = Tab.board
    @State private var tasks: [BoardTask] = []
    @State private var templates: [TeamTemplate] = []
    @State private var links: [SharedLink] = []
    @State private var feeds: [CalendarFeed] = []
    @State private var events: [CalendarEvent] = []
    @State private var feedName = ""
    @State private var feedURL = ""
    @State private var error: String?
    @State private var loading = false
    @State private var title = ""
    @State private var detail = ""
    @State private var adding = false

    private enum Tab: String, CaseIterable {
        case board, teams, links, calendar
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Workspace") {
                if loading { Spinner() }
            }
            CardForm {
                SegmentedChoice(selection: $tab, options: Tab.allCases.map { (id: $0, label: $0.label) })
                switch tab {
                case .board: board
                case .teams: teams
                case .links: linksList
                case .calendar: calendar
                }
                if let error {
                    Text(error).font(.footnote).foregroundStyle(Palette.danger)
                }
            }
        }
        .task { await load() }
        .refreshable { await load() }
    }

    // MARK: board

    private var board: some View {
        Group {
            CardSection("Add a card", footer: "A card can name the bot that owns it; moving it is yours to do.") {
                TextField("What needs doing", text: $title).plainTextInput().workspaceField()
                if adding {
                    TextField("Detail (optional)", text: $detail, axis: .vertical).plainTextInput().workspaceField()
                }
                HStack {
                    Button(adding ? "Fewer options" : "More options") { adding.toggle() }
                        .buttonStyle(.plain)
                        .foregroundStyle(Palette.secondary)
                    Spacer()
                    Button("Add") { add() }
                        .buttonStyle(.secondary)
                        .disabled(title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
            ForEach(BoardTask.statuses, id: \.self) { status in
                let column = tasks.filter { $0.status == status }
                if !column.isEmpty {
                    CardSection(statusTitle(status)) {
                        ForEach(column) { task in
                            boardRow(task)
                        }
                    }
                }
            }
            if tasks.isEmpty, !loading {
                CardSection {
                    Text("Nothing on the board yet.").foregroundStyle(Palette.secondary)
                }
            }
        }
    }

    private func boardRow(_ task: BoardTask) -> some View {
        Button {
            move(task)
        } label: {
            HStack(spacing: 10) {
                Image(systemName: task.isDone ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(task.isDone ? Palette.accent : Palette.tertiary)
                VStack(alignment: .leading, spacing: 2) {
                    Text(task.title)
                        .foregroundStyle(Palette.text)
                        .strikethrough(task.isDone)
                    if !task.detail.isEmpty {
                        Text(task.detail).font(.caption).foregroundStyle(Palette.secondary).lineLimit(2)
                    }
                }
                Spacer(minLength: 0)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .contextActions {
            BoardTask.statuses.filter { $0 != task.status }.map { status in
                MenuItem("Move to \(statusTitle(status))", icon: "arrow.right") { set(task, status: status) }
            } + [MenuItem("Delete", icon: "trash", destructive: true, divider: true) { delete(task) }]
        }
    }

    private func statusTitle(_ status: String) -> String {
        switch status {
        case "doing": return "Doing"
        case "waiting": return "Waiting"
        case "done": return "Done"
        default: return "To do"
        }
    }

    private func add() {
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let notes = detail.trimmingCharacters(in: .whitespacesAndNewlines)
        Task {
            do {
                tasks = try await store.boardTasks()
                _ = try await store.saveBoardTask(title: trimmed, detail: notes)
                tasks = try await store.boardTasks()
                title = ""
                detail = ""
                adding = false
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func move(_ task: BoardTask) {
        let order = BoardTask.statuses
        let next = order[((order.firstIndex(of: task.status) ?? 0) + 1) % order.count]
        set(task, status: next)
    }

    private func set(_ task: BoardTask, status: String) {
        Task {
            do {
                _ = try await store.moveBoardTask(task.id, to: status)
                tasks = try await store.boardTasks()
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func delete(_ task: BoardTask) {
        Task {
            do {
                _ = try await store.deleteBoardTask(task.id)
                tasks = try await store.boardTasks()
                error = nil
            } catch { self.error = message(error) }
        }
    }

    // MARK: teams

    private var teams: some View {
        Group {
            if templates.isEmpty, !loading {
                CardSection { Text("No saved teams yet. Save a group as a team on the computer.").foregroundStyle(Palette.secondary) }
            }
            ForEach(templates) { template in
                CardSection(template.name, footer: template.description.isEmpty ? nil : template.description) {
                    if !template.members.isEmpty {
                        Text(template.members.joined(separator: " · "))
                            .font(.caption)
                            .foregroundStyle(Palette.secondary)
                    }
                    HStack {
                        Spacer()
                        Button("Add to this computer") { install(template) }
                            .buttonStyle(.secondary)
                    }
                }
            }
        }
    }

    private func install(_ template: TeamTemplate) {
        Task {
            do {
                _ = try await store.installTeamTemplate(template.id)
                error = nil
            } catch { self.error = message(error) }
        }
    }

    // MARK: links

    private var linksList: some View {
        Group {
            if links.isEmpty, !loading {
                CardSection { Text("No read-only links. Publishing a document or artifact creates one.").foregroundStyle(Palette.secondary) }
            }
            ForEach(links) { link in
                CardSection(link.title, footer: "Anyone with the link can read it until it expires or is revoked.") {
                    Text(link.url).font(.caption.monospaced()).foregroundStyle(Palette.secondary).lineLimit(1)
                    HStack(spacing: 10) {
                        Button("Copy link") { UIPasteboard.general.string = link.url }
                            .buttonStyle(.secondary)
                        Button("Open") { if let url = URL(string: link.url) { openURL(url) } }
                            .buttonStyle(.secondary)
                        Spacer()
                        Button("Revoke", role: .destructive) { revoke(link) }
                            .buttonStyle(.secondary)
                    }
                }
            }
        }
    }

    private func revoke(_ link: SharedLink) {
        Task {
            do {
                _ = try await store.revokeSharedLink(link.token)
                links = try await store.sharedLinks()
                error = nil
            } catch { self.error = message(error) }
        }
    }

    // MARK: calendar

    private var calendar: some View {
        Group {
            CardSection("Add a calendar", footer: "Paste the .ics address from Google, Apple or Outlook. It is read only.") {
                TextField("Name (optional)", text: $feedName).plainTextInput().workspaceField()
                TextField("https://…/calendar.ics", text: $feedURL)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .plainTextInput()
                    .workspaceField()
                HStack {
                    Spacer()
                    Button("Add calendar") { addFeed() }
                        .buttonStyle(.secondary)
                        .disabled(feedURL.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
            if feeds.isEmpty {
                CardSection { Text("No calendars yet.").foregroundStyle(Palette.secondary) }
            } else {
                CardSection("Calendars") {
                    ForEach(feeds) { feed in
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(feed.name).foregroundStyle(Palette.text)
                                Text(feed.url).font(.caption).foregroundStyle(Palette.secondary).lineLimit(1)
                            }
                            Spacer(minLength: 0)
                            Button("Remove") { removeFeed(feed) }
                                .buttonStyle(.secondary)
                        }
                    }
                }
            }
            if events.isEmpty {
                CardSection { Text("No events in the next two weeks.").foregroundStyle(Palette.secondary) }
            } else {
                CardSection("Next two weeks") {
                    ForEach(events) { event in
                        HStack(alignment: .top, spacing: 10) {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(event.title).foregroundStyle(Palette.text)
                                Text(when(event)).font(.caption).foregroundStyle(Palette.secondary)
                                if let place = event.location, !place.isEmpty {
                                    Text(place).font(.caption).foregroundStyle(Palette.secondary)
                                }
                            }
                            Spacer(minLength: 0)
                        }
                    }
                }
            }
        }
    }

    private func when(_ event: CalendarEvent) -> String {
        let date = Date(timeIntervalSince1970: Double(event.start) / 1000)
        let day = date.formatted(date: .abbreviated, time: .omitted)
        return event.allDay ? day : "\(day), \(date.formatted(date: .omitted, time: .shortened))"
    }

    private func addFeed() {
        let url = feedURL.trimmingCharacters(in: .whitespacesAndNewlines)
        let name = feedName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty else { return }
        Task {
            do {
                let feed = try await store.addCalendarFeed(name: name.isEmpty ? "Calendar" : name, url: url)
                feeds.append(feed)
                feedName = ""
                feedURL = ""
                events = try await store.calendarAgenda()
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func removeFeed(_ feed: CalendarFeed) {
        Task {
            do {
                _ = try await store.removeCalendarFeed(feed.id)
                feeds = try await store.calendarFeeds()
                events = try await store.calendarAgenda()
                error = nil
            } catch { self.error = message(error) }
        }
    }

    // MARK: loading

    private func load() async {
        loading = true
        defer { loading = false }
        do {
            async let board = store.boardTasks()
            async let teams = store.teamTemplates()
            async let shared = store.sharedLinks()
            async let calendars = store.calendarFeeds()
            async let agenda = store.calendarAgenda()
            (tasks, templates, links, feeds, events) = try await (board, teams, shared, calendars, agenda)
            error = nil
        } catch { self.error = message(error) }
    }

    private func message(_ error: Error) -> String {
        (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
}

private extension WorkspaceSheet.Tab {
    var label: String {
        switch self {
        case .board: return "Board"
        case .teams: return "Teams"
        case .links: return "Links"
        case .calendar: return "Calendar"
        }
    }
}

private extension View {
    /// A rounded input box, the same shape as the Fields in settings.
    func workspaceField() -> some View {
        padding(10)
            .background(Palette.surface, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
    }
}
