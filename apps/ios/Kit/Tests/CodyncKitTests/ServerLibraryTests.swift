import Foundation
import Testing
@testable import CodyncKit

private func artifact(_ id: String, at: Int64) -> ChatLibraryItem {
    ChatLibraryItem(Artifact(id: id, botId: "b1", title: id, size: 1, sha256: "", createdAt: 1, updatedAt: at, entryId: nil))
}

private func file(_ id: String, name: String, at: Int64, image: Bool) -> ServerLibraryItem {
    let item = ChatLibraryItem(.file(SharedFile(id: id, name: name, size: 1, sha256: ""), entryId: "e-\(id)"), at: at)
    return ServerLibraryItem(botId: "b1", botName: "Bot", image: image, item: item)
}

@Test func theServerLibraryFilterKeepsImagesAndFilesApart() {
    let photo = file("photo", name: "Trip.PNG", at: 4, image: true)
    let plan = file("plan", name: "plan.md", at: 2, image: false)
    let report = ServerLibraryItem(botId: "b1", botName: "Bot", image: false, item: artifact("report", at: 3))
    let items = [photo, report, plan]

    #expect(ServerLibraryFilter.all.apply(to: items).map(\.at) == [4, 3, 2] as [Int64])
    #expect(ServerLibraryFilter.images.apply(to: items).map(\.at) == [4] as [Int64])
    #expect(ServerLibraryFilter.files.apply(to: items).map(\.at) == [3, 2] as [Int64], "an artifact counts as a file")
    #expect(ServerLibraryFilter.images.apply(to: [ServerLibraryItem]()).isEmpty)
}

@Test func aServerLibraryItemReadsWhatTheHostSends() throws {
    let wire = """
    {"kind":"file","at":5,"entryId":"e1","botId":"b1","botName":"Writer","image":true,
     "file":{"id":"f1","name":"photo.png","size":9,"sha256":"abc"}}
    """
    let item = try #require(JSONDecoder().decode(ServerLibraryItem.Wire.self, from: Data(wire.utf8)).serverItem)
    #expect(item.id == "b1/file/e1/f1")
    #expect(item.botName == "Writer")
    #expect(item.image)
    #expect(item.at == 5)
}
