import CodyncKit
import SwiftUI

/// Someone who uses this computer with their own phone or bots (`people` on the host). The owner is implicit:
/// a bot or connector with no person is theirs.
struct PersonRecord: Decodable, Sendable, Identifiable {
    let id: String
    var name: String
    var email: String?
}

/// A paired device as the computer lists it for its owner; `personId` is the person it acts as.
struct HostDevice: Decodable, Sendable, Identifiable {
    var id: String { key }
    let key: String
    var name: String
    var personId: String?
    var person: String?
}

extension BotStore {
    func peopleList() async throws -> [PersonRecord] {
        struct Reply: Decodable, Sendable { var people: [PersonRecord] }
        let reply: Reply = try await ready().call("people")
        return reply.people
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
    @State private var devices: [HostDevice] = []
    @State private var name = ""
    @State private var email = ""
    @State private var busy = false
    @State private var error: String?
    @State private var removing: PersonRecord?
    @State private var loaded = false

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
                        ValueRow(person.email ?? L10n.tr("Person")) {
                            Button("Remove") { removing = person }
                                .buttonStyle(.secondary)
                                .disabled(busy)
                        }
                        .accessibilityLabel("\(person.name), \(person.email ?? "")")
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
            message: L10n.tr("Their phone, bots, chats and connectors become yours. Nothing is deleted.")
        ) {
            [DialogAction(L10n.tr("Remove"), destructive: true) {
                if let person = removing { Task { await remove(person) } }
            }]
        }
    }

    private var choices: [(id: String, label: String)] {
        [(id: "", label: L10n.tr("This computer's owner"))] + people.map { (id: $0.id, label: $0.name) }
    }

    private func load() async {
        do {
            async let people = store.peopleList()
            async let devices = store.authorizedDevices()
            self.people = try await people
            self.devices = try await devices
            loaded = true
            error = nil
        } catch {
            self.error = error.localizedDescription
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
