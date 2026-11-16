import CodyncKit
import Foundation
import SwiftUI

/// An encrypted copy of the computer's database in the person's own WebDAV folder.
struct BackupSettingsView: View {
    @Environment(BotStore.self) private var store
    @Environment(\.dismiss) private var dismiss
    @State private var status: CloudBackupStatus?
    @State private var url = ""
    @State private var username = ""
    @State private var password = ""
    @State private var keep = "14"
    @State private var enabled = false
    @State private var busy = false
    @State private var error: String?
    @State private var notice: String?

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader {
                HStack(spacing: 8) {
                    Image(systemName: "icloud").font(.title2).accessibilityHidden(true)
                    Text("Backups")
                }
            }
            CardForm {
                CardSection("Cloud backup", footer: "Codync seals the database with this computer's vault key before uploading. The server only sees ciphertext.") {
                    TextField("WebDAV folder (https://…)", text: $url)
                        .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput()
                    TextField("Username", text: $username)
                        .textInputAutocapitalization(.never).autocorrectionDisabled().plainTextInput()
                    SecureField(status?.hasPassword == true ? "Password saved; leave empty to keep it" : "Password", text: $password)
                        .plainTextInput()
                    HStack {
                        Text("Keep").foregroundStyle(Palette.secondary)
                        Spacer(minLength: 0)
                        TextField("14", text: $keep).keyboardType(.numberPad).plainTextInput().frame(width: 80)
                    }
                    Toggle(isOn: $enabled) { Text("Back up every day") }
                        .toggleStyle(.codync)
                    Text("A backup is uploaded only while the computer's host runs.")
                        .font(.caption).foregroundStyle(Palette.secondary)
                }
                CardSection {
                    HStack {
                        Button("Save") { save(runNow: false) }
                            .buttonStyle(.secondary)
                            .disabled(busy || url.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        Spacer(minLength: 0)
                        Button("Back up now") { save(runNow: true) }
                            .buttonStyle(.secondary)
                            .disabled(busy || status?.configured != true)
                    }
                }
                if let status {
                    CardSection("Last backup") {
                        if let last = status.lastAt {
                            Text(when(last)).foregroundStyle(Palette.text)
                        } else {
                            Text("No backup yet.").foregroundStyle(Palette.secondary)
                        }
                        if let failure = status.lastError {
                            Text(failure).font(.caption).foregroundStyle(Palette.danger)
                        }
                        ForEach(status.files.reversed()) { file in
                            HStack {
                                Text(file.name).font(.caption).foregroundStyle(Palette.secondary).lineLimit(1)
                                Spacer(minLength: 0)
                                Text(when(file.at)).font(.caption).foregroundStyle(Palette.tertiary)
                            }
                        }
                    }
                }
                if let notice { Text(notice).font(.caption).foregroundStyle(Palette.secondary) }
                if let error { Text(error).font(.caption).foregroundStyle(Palette.danger) }
            }
        }
        .task { load() }
    }

    private func load() {
        Task {
            do {
                let next = try await store.cloudBackupStatus()
                status = next
                url = next.url
                username = next.username
                keep = String(next.keep)
                enabled = next.enabled
                error = nil
            } catch { error = message(error) }
        }
    }

    private func save(runNow: Bool) {
        let count = Int(keep) ?? 14
        guard count >= 1, count <= 100 else {
            error = "Keep between 1 and 100 backups."
            return
        }
        busy = true
        notice = nil
        Task {
            do {
                status = try await store.saveCloudBackup(
                    url: url.trimmingCharacters(in: .whitespacesAndNewlines),
                    username: username.trimmingCharacters(in: .whitespacesAndNewlines),
                    password: password,
                    enabled: enabled,
                    keep: count
                )
                password = ""
                if runNow {
                    let upload = try await store.runCloudBackup()
                    notice = "Uploaded \(upload.name) (\(max(1, upload.bytes / 1024)) KB)."
                } else {
                    notice = "Saved."
                }
                load()
            } catch { error = message(error) }
            busy = false
        }
    }

    private func when(_ ms: Int) -> String {
        Date(timeIntervalSince1970: Double(ms) / 1000)
            .formatted(.dateTime.day().month(.abbreviated).hour().minute())
    }

    private func message(_ error: Error) -> String {
        (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
    }
}
