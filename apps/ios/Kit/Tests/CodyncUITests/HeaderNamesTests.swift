import Testing
@testable import CodyncUI

@Suite struct HeaderNamesTests {
    @Test func a_header_name_is_an_http_token() {
        for good in ["Authorization", "X-Api-Key", "X_API_KEY", "X.Api.Key", "X~Key", "X1"] {
            #expect(HeaderNames.valid(good), "\(good) is usable")
        }
        for bad in ["Authorization Bearer", "X:Key", "X/Key", "X(Key)", "X@Key", ""] {
            #expect(!HeaderNames.valid(bad), "\(bad) is not usable")
        }
        #expect(HeaderNames.message.contains("Authorization"))
        #expect(HeaderNames.message.contains("Value"))
    }

    @Test func the_form_finds_the_bad_key_in_a_name_value_box() {
        #expect(HeaderNames.invalidKey(in: "Authorization: Bearer sk-1\nX-Api-Key: k") == nil)
        #expect(HeaderNames.invalidKey(in: "\n\n") == nil)
        #expect(HeaderNames.invalidKey(in: "Authorization Bearer: sk-1") == "Authorization Bearer")
        #expect(HeaderNames.invalidKey(in: "Authorization: Bearer sk-1\nX Key: k") == "X Key")
    }
}
