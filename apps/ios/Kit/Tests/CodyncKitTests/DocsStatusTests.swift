import Foundation
import Testing
@testable import CodyncKit

/// `docsStatus` in `host/src/docs/mod.rs`: the folder the index reads, what it holds, and whether a limit
/// cut the walk short. The host also names when it was built; the card has no use for it.
@Test func docsStatusReadsTheIndexedFolderAndItsContents() throws {
    let json = #"{"folder":"/Users/me/code/docs","files":42,"bytes":131072,"truncated":false,"builtAt":1763000000000}"#
    let status = try JSONDecoder().decode(DocsStatus.self, from: Data(json.utf8))
    #expect(status.folder == "/Users/me/code/docs")
    #expect(status.files == 42)
    #expect(status.bytes == 131_072)
    #expect(status.truncated == false)
}

@Test func docsStatusSaysWhenTheIndexStoppedAtItsLimit() throws {
    let json = #"{"folder":"/code","files":4000,"bytes":25165824,"truncated":true,"builtAt":1763000000000}"#
    let status = try JSONDecoder().decode(DocsStatus.self, from: Data(json.utf8))
    #expect(status.truncated)
    #expect(status.files == 4_000)
}
