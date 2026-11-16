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
    @State private var waiting: [WaitingItem] = []
    @State private var waitTitle = ""
    @State private var waitWho = ""
    @State private var waitDue = ""
    @State private var templates: [TeamTemplate] = []
    @State private var links: [SharedLink] = []
    @State private var feeds: [CalendarFeed] = []
    @State private var events: [CalendarEvent] = []
    @State private var feedName = ""
    @State private var feedURL = ""
    @State private var emailAccounts: [EmailAccount] = []
    @State private var messages: [EmailMessage] = []
    @State private var emailAccount = ""
    @State private var emailName = ""
    @State private var emailAddress = ""
    @State private var emailPassword = ""
    @State private var imapHost = ""
    @State private var smtpHost = ""
    @State private var emailUsername = ""
    @State private var mailTo = ""
    @State private var mailSubject = ""
    @State private var mailBody = ""
    @State private var error: String?
    @State private var loading = false
    @State private var title = ""
    @State private var detail = ""
    @State private var adding = false

    private enum Tab: String, CaseIterable {
        case board, teams, links, calendar, email
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
                case .email: email
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
            CardSection("Waiting on", footer: "Replies, reviews and deliveries other people owe you. They show in the daily digest and a bot can list them.") {
                TextField("What are you waiting on", text: $waitTitle).plainTextInput().workspaceField()
                TextField("From (optional)", text: $waitWho).plainTextInput().workspaceField()
                TextField("Due (2026-11-20, optional)", text: $waitDue).plainTextInput().workspaceField()
                HStack {
                    Spacer()
                    Button("Add") { addWaiting() }
                        .buttonStyle(.secondary)
                        .disabled(waitTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
                ForEach(waiting.filter { !$0.done }) { item in
                    HStack(spacing: 8) {
                        Button { finishWaiting(item) } label: {
                            Image(systemName: "circle").foregroundStyle(Palette.tertiary)
                        }
                        .buttonStyle(.plain)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.title).foregroundStyle(Palette.text)
                            Text([item.counterpart, waitingDue(item)].filter { !$0.isEmpty }.joined(separator: " · "))
                                .font(.caption)
                                .foregroundStyle(Palette.secondary)
                                .opacity(item.counterpart.isEmpty && waitingDue(item).isEmpty ? 0 : 1)
                        }
                        Spacer(minLength: 0)
                    }
                    .contextActions { [MenuItem("Delete", icon: "trash", destructive: true, divider: true) { deleteWaiting(item) }] }
                }
                if waiting.allSatisfy({ $0.done }), !loading {
                    Text("Nothing is waiting on anyone.").font(.caption).foregroundStyle(Palette.secondary)
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

    private func waitingDue(_ item: WaitingItem) -> String {
        guard let due = item.dueAt else { return "" }
        return Date(timeIntervalSince1970: Double(due) / 1000)
            .formatted(.dateTime.month(.abbreviated).day())
    }

    private func addWaiting() {
        let title = waitTitle.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty else { return }
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        let due = waitDue.trimmingCharacters(in: .whitespacesAndNewlines)
        let dueAt = due.isEmpty ? nil : formatter.date(from: due).map { Int($0.timeIntervalSince1970 * 1000) }
        if !due.isEmpty, dueAt == nil {
            error = "The due date is a day, like 2026-11-20."
            return
        }
        Task {
            do {
                _ = try await store.saveWaitingItem(
                    title: title,
                    counterpart: waitWho.trimmingCharacters(in: .whitespacesAndNewlines),
                    dueAt: dueAt
                )
                waiting = try await store.waitingItems()
                waitTitle = ""
                waitWho = ""
                waitDue = ""
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func finishWaiting(_ item: WaitingItem) {
        Task {
            do {
                _ = try await store.setWaitingDone(item.id, done: true)
                waiting = try await store.waitingItems()
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func deleteWaiting(_ item: WaitingItem) {
        Task {
            do {
                _ = try await store.deleteWaitingItem(item.id)
                waiting = try await store.waitingItems()
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

    // MARK: email

    private var email: some View {
        Group {
            if emailAccounts.isEmpty {
                CardSection { Text("No email account yet. Add one below.").foregroundStyle(Palette.secondary) }
            } else {
                CardSection("Accounts") {
                    ForEach(emailAccounts) { account in
                        HStack {
                            Button {
                                emailAccount = account.id
                            } label: {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(account.name).foregroundStyle(Palette.text)
                                    Text(account.address).font(.caption).foregroundStyle(Palette.secondary)
                                }
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            Spacer(minLength: 0)
                            if emailAccount == account.id {
                                Text("Reading").font(.caption).foregroundStyle(Palette.secondary)
                            }
                            Button("Remove") { removeAccount(account) }
                                .buttonStyle(.secondary)
                        }
                    }
                    HStack {
                        Spacer()
                        Button("Read inbox") { readInbox() }
                            .buttonStyle(.secondary)
                            .disabled(emailAccount.isEmpty)
                    }
                }
                if !messages.isEmpty {
                    CardSection("Inbox") {
                        ForEach(messages) { message in
                            VStack(alignment: .leading, spacing: 2) {
                                Text(message.subject.isEmpty ? "(no subject)" : message.subject)
                                    .foregroundStyle(Palette.text)
                                Text("\(message.from) · \(message.date)")
                                    .font(.caption).foregroundStyle(Palette.secondary)
                            }
                        }
                    }
                }
            }

            CardSection("Add an account", footer: "IMAP uses TLS on 993; SMTP uses STARTTLS on 587. An app password works where the provider requires one.") {
                TextField("Name (optional)", text: $emailName).plainTextInput().workspaceField()
                TextField("me@example.com", text: $emailAddress)
                    .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput().workspaceField()
                SecureField("Password or app password", text: $emailPassword).plainTextInput().workspaceField()
                TextField("IMAP server (imap.example.com)", text: $imapHost)
                    .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput().workspaceField()
                TextField("SMTP server (smtp.example.com)", text: $smtpHost)
                    .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput().workspaceField()
                TextField("Username (defaults to the address)", text: $emailUsername)
                    .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput().workspaceField()
                HStack {
                    Spacer()
                    Button("Save account") { addAccount() }
                        .buttonStyle(.secondary)
                        .disabled(emailAddress.isEmpty || imapHost.isEmpty || smtpHost.isEmpty || emailPassword.isEmpty)
                }
            }

            if !emailAccounts.isEmpty {
                CardSection("Send an email", footer: "It is sent at once from the selected account.") {
                    TextField("To", text: $mailTo)
                        .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput().workspaceField()
                    TextField("Subject", text: $mailSubject).plainTextInput().workspaceField()
                    TextField("Message", text: $mailBody, axis: .vertical).plainTextInput().workspaceField()
                    HStack {
                        Spacer()
                        Button("Send") { sendMail() }
                            .buttonStyle(.secondary)
                            .disabled(mailTo.isEmpty || (mailBody.isEmpty && mailSubject.isEmpty))
                    }
                }
            }
        }
    }

    private func readInbox() {
        guard !emailAccount.isEmpty else { return }
        Task {
            do {
                messages = try await store.emailInbox(account: emailAccount)
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func addAccount() {
        Task {
            do {
                _ = try await store.saveEmailAccount(
                    name: emailName.isEmpty ? emailAddress : emailName,
                    address: emailAddress.trimmingCharacters(in: .whitespacesAndNewlines),
                    password: emailPassword,
                    imapHost: imapHost.trimmingCharacters(in: .whitespacesAndNewlines),
                    smtpHost: smtpHost.trimmingCharacters(in: .whitespacesAndNewlines),
                    username: emailUsername.trimmingCharacters(in: .whitespacesAndNewlines)
                )
                emailAccounts = try await store.emailAccounts()
                emailAccount = emailAccounts.last?.id ?? emailAccount
                emailName = ""
                emailAddress = ""
                emailPassword = ""
                imapHost = ""
                smtpHost = ""
                emailUsername = ""
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func removeAccount(_ account: EmailAccount) {
        Task {
            do {
                _ = try await store.deleteEmailAccount(account.id)
                emailAccounts = try await store.emailAccounts()
                if emailAccount == account.id {
                    emailAccount = emailAccounts.first?.id ?? ""
                    messages = []
                }
                error = nil
            } catch { self.error = message(error) }
        }
    }

    private func sendMail() {
        Task {
            do {
                _ = try await store.sendEmail(account: emailAccount, to: mailTo, subject: mailSubject, body: mailBody)
                mailTo = ""
                mailSubject = ""
                mailBody = ""
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
            async let mails = store.emailAccounts()
            async let pending = store.waitingItems()
            (tasks, templates, links, feeds, events, emailAccounts, waiting) = try await (board, teams, shared, calendars, agenda, mails, pending)
            if emailAccount.isEmpty { emailAccount = emailAccounts.first?.id ?? "" }
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
        case .email: return "Email"
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
