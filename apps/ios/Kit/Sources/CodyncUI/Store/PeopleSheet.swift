import CodyncKit
import SwiftUI

/// Someone who uses this computer with their own phone or bots (`people` on the host). The owner is implicit:
/// a bot or connector with no person is theirs.
struct PersonRecord: Decodable, Sendable, Identifiable {
    let id: String
    var name: String
    var email: String?
    /// `admin` manages people, connectors and bots; `member` only their own.
    var role: String?
}

/// A paired device as the computer lists it for its owner; `personId` is the person it acts as.
struct HostDevice: Decodable, Sendable, Identifiable {
    var id: String { key }
    let key: String
    var name: String
    var personId: String?
    var person: String?
}

/// One member session, paired device or live invitation (`memberSessions`).
struct MemberSessionRecord: Decodable, Sendable, Identifiable {
    let id: String
    var kind: String
    var person: String
    var email: String?
    var role: String?
    var name: String?
    var platform: String?
    var createdAt: Int64
    var lastSeenAt: Int64?
    var expiresAt: Int64?
    var connected: Bool
}

/// A one-time invitation the owner or an admin hands a member (`inviteMember`).
struct MemberInviteRecord: Decodable, Sendable {
    var code: String
    var expiresAt: Int64
    var pairingUrl: String?
    var note: String?
}

extension BotStore {
    func peopleList() async throws -> [PersonRecord] {
        struct Reply: Decodable, Sendable { var people: [PersonRecord] }
        let reply: Reply = try await ready().call("people")
        return reply.people
    }

    /// Whether this caller is the owner: an admin's phone sees the people but can't change a role.
    func canManageRoles() async throws -> Bool {
        struct Reply: Decodable, Sendable { var canManageRoles: Bool? }
        let reply: Reply = try await ready().call("people")
        return reply.canManageRoles ?? true
    }

    /// Every member session, paired member device and live invitation; empty on a host without them.
    func memberSessions() async throws -> [MemberSessionRecord] {
        struct Reply: Decodable, Sendable { var sessions: [MemberSessionRecord] }
        let reply: Reply = try await ready().call("memberSessions")
        return reply.sessions
    }

    /// Invites a member by email, or replaces that person's earlier invitation.
    func inviteMember(name: String, email: String, role: String) async throws -> (person: PersonRecord, invite: MemberInviteRecord) {
        struct Body: Encodable { var name: String; var email: String; var role: String }
        struct Reply: Decodable, Sendable { var person: PersonRecord; var invite: MemberInviteRecord }
        let reply: Reply = try await ready().call("inviteMember", Body(name: name, email: email, role: role))
        return (reply.person, reply.invite)
    }

    /// Ends one member session or cancels one invitation.
    func revokeMemberSession(_ id: String) async throws {
        struct Body: Encodable { var session: String }
        let _: Empty = try await ready().call("revokeMemberSession", Body(session: id))
    }

    func authorizedDevices() async throws -> [HostDevice] {
        struct Reply: Decodable, Sendable { var devices: [HostDevice] }
        let reply: Reply = try await ready().call("devices")
        return reply.devices
    }

    func addPerson(name: String, email: String) async throws {
        struct Body: Encodable { var name: String; var email: String }
        let _: Empty = try await ready().call("addPerson", Body(name: name, email: email))
    }

    func removePerson(_ id: String) async throws {
        let _: Empty = try await ready().call("removePerson", ["id": id])
    }

    /// Changes a person's role; only the owner is allowed to.
    func setPersonRole(id: String, name: String, email: String, role: String) async throws {
        struct Body: Encodable { var id: String; var name: String; var email: String; var role: String }
        let _: Empty = try await ready().call("updatePerson", Body(id: id, name: name, email: email, role: role))
    }

    /// `person` empty gives the device back to the owner.
    func setDevicePerson(_ device: String, person: String) async throws {
        struct Body: Encodable { var device: String; var person: String }
        let _: Empty = try await ready().call("setDevicePerson", Body(device: device, person: person))
    }
}

/// Who is on this computer besides its owner: add or remove a person, and assign a paired phone to one.
/// Only the owner reaches this: a member's phone is refused by the host, and this sheet then shows the error.
/// There is no member password: a person is the label the owner puts on a device or a bot.
public struct PeopleSheet: View {
    public init() {}
    @Environment(BotStore.self) private var store
    @State private var people: [PersonRecord] = []
    @State private var sessions: [MemberSessionRecord] = []
    @State private var devices: [HostDevice] = []
    @State private var name = ""
    @State private var email = ""
    @State private var inviteName = ""
    @State private var inviteEmail = ""
    @State private var inviteRole = "member"
    @State private var issued: MemberInviteRecord?
    @State private var issuedTo = ""
    @State private var busy = false
    @State private var error: String?
    @State private var removing: PersonRecord?
    @State private var loaded = false
    @State private var canManageRoles = true

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("People")
            CardForm {
                CardSection(
                    "People on this computer",
                    footer: "Someone who uses this computer with their own phone or their own bots. Their phone sees only their bots, chats and connectors, plus the ones shared with everyone."
                ) {
                    if loaded, people.isEmpty {
                        Text(L10n.tr("No one else yet")).font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    ForEach(people) { person in
                        ValueRow(person.name, detail: person.email) {
                            HStack(spacing: 8) {
                                if canManageRoles {
                                    ChoicePicker(
                                        selection: Binding(
                                            get: { person.role ?? "member" },
                                            set: { role in setRole(person, role: role) }
                                        ),
                                        options: [(id: "member", label: L10n.tr("Member")), (id: "admin", label: L10n.tr("Admin"))]
                                    )
                                } else {
                                    Text(L10n.tr(person.role == "admin" ? "Admin" : "Member"))
                                        .font(.caption)
                                        .foregroundStyle(Palette.secondary)
                                }
                                Button("Remove") { removing = person }
                                    .buttonStyle(.secondary)
                                    .disabled(busy)
                            }
                        }
                        .accessibilityLabel("\(person.name), \(person.role ?? "member")")
                    }
                    if !people.isEmpty || loaded {
                        TextField(L10n.tr("Name"), text: $name)
                            .textFieldStyle(.plain)
                        TextField(L10n.tr("Email (optional)"), text: $email)
                            .textFieldStyle(.plain)
                            .textInputAutocapitalization(.never)
                            .keyboardType(.emailAddress)
                        Button(busy ? L10n.tr("Adding…") : L10n.tr("Add a person")) { add() }
                            .buttonStyle(.primary)
                            .disabled(busy || name.trimmingCharacters(in: .whitespaces).isEmpty)
                    }
                }
                CardSection(
                    "Invite a member",
                    footer: "An invitation is a one-time code the member scans on their phone, or signs in with at the computer. They act as that person, with only their own bots, chats and connectors."
                ) {
                    TextField(L10n.tr("Name (optional)"), text: $inviteName)
                        .textFieldStyle(.plain)
                    TextField(L10n.tr("Email"), text: $inviteEmail)
                        .textFieldStyle(.plain)
                        .plainTextInput()
                        .keyboardType(.emailAddress)
                    if canManageRoles {
                        ChoicePicker(
                            selection: $inviteRole,
                            options: [(id: "member", label: L10n.tr("Member")), (id: "admin", label: L10n.tr("Admin"))]
                        )
                    }
                    Button(busy ? L10n.tr("Inviting…") : L10n.tr("Invite")) { sendInvite() }
                        .buttonStyle(.primary)
                        .disabled(busy || inviteEmail.trimmingCharacters(in: .whitespaces).isEmpty)
                    if let issued {
                        Text(L10n.tr("Invitation for {name}").replacingOccurrences(of: "{name}", with: issuedTo))
                            .font(.caption)
                            .foregroundStyle(Palette.secondary)
                        Text(issued.code).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                        if let link = issued.pairingUrl {
                            HStack(spacing: 8) {
                                Button(L10n.tr("Copy invitation link")) { Pasteboard.copy(link) }
                                    .buttonStyle(.secondary)
                                Button(L10n.tr("Copy code")) { Pasteboard.copy(issued.code) }
                                    .buttonStyle(.secondary)
                            }
                        }
                        Text(issued.note ?? L10n.tr("The code works once."))
                            .font(.caption)
                            .foregroundStyle(Palette.tertiary)
                    }
                }
                CardSection(
                    "Member sessions",
                    footer: "A session is a paired phone or a signed-in app. Ending one revokes that device at once; the person and their bots stay."
                ) {
                    if loaded, sessions.isEmpty {
                        Text(L10n.tr("None yet")).font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    ForEach(sessions) { session in
                        ValueRow(session.person, detail: sessionDetail(session)) {
                            HStack(spacing: 8) {
                                Text(session.kind == "invite" ? L10n.tr("Pending")
                                     : session.connected ? L10n.tr("Connected") : L10n.tr("Signed out"))
                                    .font(.caption)
                                    .foregroundStyle(Palette.secondary)
                                Button(L10n.tr("End session")) { revoke(session) }
                                    .buttonStyle(.secondary)
                                    .disabled(busy)
                            }
                        }
                    }
                }
                CardSection(
                    "Assign a device",
                    footer: "A member's phone only sees and changes what is theirs."
                ) {
                    if loaded, devices.isEmpty {
                        Text(L10n.tr("None yet")).font(.footnote).foregroundStyle(Palette.secondary)
                    }
                    ForEach(devices) { device in
                        ValueRow(device.name) {
                            ChoicePicker(
                                selection: Binding(
                                    get: { device.personId ?? "" },
                                    set: { assign(device, person: $0) }
                                ),
                                options: choices
                            )
                        }
                    }
                }
                if let error {
                    Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
                }
                if busy {
                    Text(L10n.tr("Saving…")).font(.caption).foregroundStyle(Palette.secondary)
                }
            }
        }
        .background(Palette.background)
        .presentationDetents([.large])
        .task { await load() }
        .codyncDialog(
            L10n.tr("Remove {name}?").replacingOccurrences(of: "{name}", with: removing?.name ?? L10n.tr("Person")),
            isPresented: Binding(get: { removing != nil }, set: { if !$0 { removing = nil } }),
            message: L10n.tr("Their phone loses access at once. Their bots, chats and connectors become yours. Nothing is deleted.")
        ) {
            [DialogAction(L10n.tr("Remove"), destructive: true) {
                if let person = removing { Task { await remove(person) } }
            }]
        }
    }

    private var choices: [(id: String, label: String)] {
        [(id: "", label: L10n.tr("This computer's owner"))] + people.map { (id: $0.id, label: $0.name) }
    }

    private func sessionDetail(_ session: MemberSessionRecord) -> String {
        if session.kind == "invite" {
            guard let expiresAt = session.expiresAt else { return L10n.tr("Invitation") }
            let date = Date(timeIntervalSince1970: TimeInterval(expiresAt) / 1000)
            return L10n.tr("Invitation") + " · " + date.formatted(date: .abbreviated, time: .shortened)
        }
        return [session.name, session.platform].compactMap { $0 }.joined(separator: " · ")
    }

    private func load() async {
        do {
            async let people = store.peopleList()
            async let devices = store.authorizedDevices()
            async let sessions = store.memberSessions()
            async let roles = store.canManageRoles()
            self.people = try await people
            self.devices = try await devices
            self.sessions = (try? await sessions) ?? []
            self.canManageRoles = (try? await roles) ?? true
            loaded = true
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func sendInvite() {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                let answer = try await store.inviteMember(name: inviteName, email: inviteEmail, role: inviteRole)
                issued = answer.invite
                issuedTo = answer.person.name
                inviteName = ""
                inviteEmail = ""
                inviteRole = "member"
                await load()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }

    private func revoke(_ session: MemberSessionRecord) {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                try await store.revokeMemberSession(session.id)
                await load()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }

    private func add() {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                try await store.addPerson(name: name, email: email)
                name = ""
                email = ""
                await load()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }

    private func remove(_ person: PersonRecord) {
        busy = true
        error = nil
        Task {
            do {
                try await store.removePerson(person.id)
                await load()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }

    private func setRole(_ person: PersonRecord, role: String) {
        guard !busy else { return }
        busy = true
        error = nil
        Task {
            do {
                try await store.setPersonRole(id: person.id, name: person.name, email: person.email ?? "", role: role)
                await load()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }

    private func assign(_ device: HostDevice, person: String) {
        busy = true
        error = nil
        Task {
            do {
                try await store.setDevicePerson(device.key, person: person)
                await load()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
