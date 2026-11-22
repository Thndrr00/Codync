import Testing
@testable import CodyncUI

/// Older messages load before the top row is reached, and only while the reader is up there.
@Test func olderPagesLoadNearTheTopOnly() {
    #expect(ConversationNearTop.isNear(offset: 0, inset: 0, height: 800))
    #expect(ConversationNearTop.isNear(offset: 100, inset: 0, height: 800))
    #expect(ConversationNearTop.isNear(offset: 20, inset: 80, height: 800))
    #expect(!ConversationNearTop.isNear(offset: 300, inset: 0, height: 800))
    #expect(!ConversationNearTop.isNear(offset: 0, inset: 0, height: 0))
}
