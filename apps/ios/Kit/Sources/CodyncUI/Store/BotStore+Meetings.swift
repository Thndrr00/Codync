import CodyncKit
import Foundation

/// A meeting note as a list shows it: title, date and whether minutes exist.
public struct MeetingSummary: Decodable, Identifiable, Equatable, Sendable {
    public var id: String
    public var title: String
    public var startedAt: Int
    public var finishedAt: Int
    public var chars: Int
    public var hasMinutes: Bool
}

/// One meeting's transcript and, once asked for, its minutes.
public struct MeetingNote: Decodable, Identifiable, Equatable, Sendable {
    public var id: String
    public var title: String
    public var startedAt: Int
    public var finishedAt: Int
    public var chars: Int
    public var hasMinutes: Bool
    public var transcript: String
    public var minutes: String?
}

/// A recording that just started on the computer.
public struct MeetingStart: Decodable, Equatable, Sendable {
    public var id: String
    public var title: String
    public var startedAt: Int
}

extension BotStore {
    private struct NotesWrap: Decodable { var notes: [MeetingSummary] }
    private struct NoteWrap: Decodable { var note: MeetingNote }

    public func meetingNotes() async throws -> [MeetingSummary] {
        let client = try await ready()
        let wrap: NotesWrap = try await client.call("meetingNotes")
        return wrap.notes
    }

    public func meetingNote(_ id: String) async throws -> MeetingNote {
        let client = try await ready()
        let wrap: NoteWrap = try await client.call("meetingNote", ["id": id])
        return wrap.note
    }

    /// Starts a recording on this computer; `audio` goes to `appendMeetingAudio` as raw 16 kHz PCM16.
    public func startMeeting(title: String) async throws -> MeetingStart {
        let client = try await ready()
        return try await client.call("startMeeting", ["title": title])
    }

    public func appendMeetingAudio(_ id: String, pcm: Data) async throws {
        let client = try await ready()
        _ = try await client.call(
            "appendMeetingAudio",
            ["id": id, "audio": pcm.base64EncodedString()],
            timeout: 120
        )
    }

    public func finishMeeting(_ id: String) async throws -> MeetingNote {
        let client = try await ready()
        let wrap: NoteWrap = try await client.call("finishMeeting", ["id": id], timeout: 600)
        return wrap.note
    }

    @discardableResult
    public func cancelMeeting(_ id: String) async throws -> Bool {
        struct Wrap: Decodable { var cancelled: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("cancelMeeting", ["id": id])
        return wrap.cancelled
    }

    @discardableResult
    public func meetingMinutes(_ id: String) async throws -> MeetingNote {
        let client = try await ready()
        let wrap: NoteWrap = try await client.call("meetingMinutes", ["id": id], timeout: 600)
        return wrap.note
    }

    @discardableResult
    public func deleteMeetingNote(_ id: String) async throws -> Bool {
        struct Wrap: Decodable { var deleted: Bool }
        let client = try await ready()
        let wrap: Wrap = try await client.call("deleteMeetingNote", ["id": id])
        return wrap.deleted
    }
}
