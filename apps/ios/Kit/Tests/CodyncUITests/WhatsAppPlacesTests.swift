import Testing
@testable import CodyncUI

@Suite struct WhatsAppPlacesTests {
    @Test func the_owners_own_chat_is_a_place_even_on_a_dedicated_number() {
        let me = "5215555555555@s.whatsapp.net"
        #expect(WhatsAppPlaces.list(me: me, allowed: []) == [WhatsAppPlace(id: me, own: true)])
        #expect(WhatsAppPlaces.list(me: me, allowed: ["friend@s.whatsapp.net"]) == [
            WhatsAppPlace(id: me, own: true), WhatsAppPlace(id: "friend@s.whatsapp.net", own: false),
        ])
    }

    @Test func an_unlinked_number_has_no_own_place_and_the_allowlist_is_not_duplicated() {
        #expect(WhatsAppPlaces.list(me: nil, allowed: ["friend@s.whatsapp.net"]) == [
            WhatsAppPlace(id: "friend@s.whatsapp.net", own: false),
        ])
        let me = "me@s.whatsapp.net"
        #expect(WhatsAppPlaces.list(me: me, allowed: [me, "friend@s.whatsapp.net"]) == [
            WhatsAppPlace(id: me, own: true), WhatsAppPlace(id: "friend@s.whatsapp.net", own: false),
        ])
    }
}
