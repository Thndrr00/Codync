import Testing
import CodyncKit
@testable import CodyncUI

private func thread(_ id: String, _ updatedAt: Int64) -> ThreadInfo {
    ThreadInfo(id: id, title: id, fromEntry: nil, branchText: nil, archived: true,
               createdAt: updatedAt, updatedAt: updatedAt, unread: 0, lastText: nil, lastAuthor: nil)
}

@Test func aFullArchivedPageOffersAnotherPageAndKeepsSavedRows() {
    let first = (0..<ThreadPaging.pageSize).map { thread("t\($0)", Int64(100 - $0)) }
    let page = ThreadPaging.merge(first, [thread("older", 1)])
    #expect(!page.hasMore)
    #expect(page.threads.count == ThreadPaging.pageSize + 1)
    #expect(page.threads.last?.id == "older")
    #expect(ThreadPaging.cursor(page.threads) == "older")
}

@Test func aHostThatIgnoresTheCursorStopsPaging() {
    let first = (0..<ThreadPaging.pageSize).map { thread("t\($0)", Int64(100 - $0)) }
    #expect(ThreadPaging.merge(first, first).hasMore == false)
    #expect(ThreadPaging.merge([], first).hasMore == true)
}

@Test func aPartialPageHasNoOlderThreadsAndStaysNewestFirst() {
    let page = ThreadPaging.merge([thread("new", 200)], [thread("old", 100), thread("older", 50)])
    #expect(!page.hasMore)
    #expect(page.threads.map(\.id) == ["new", "old", "older"])
    #expect(ThreadPaging.cursor([]) == nil)
}
