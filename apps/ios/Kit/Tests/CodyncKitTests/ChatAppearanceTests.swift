import Foundation
import Testing
@testable import CodyncKit

struct ChatAppearanceTests {
    private let colours = ChatColours.all.map(\.id)

    @Test func aColourSavedBeforeTheThemesSplitAppliesToBoth() {
        let choice = ChatChoice.decode("green", known: colours)
        #expect(choice.light == "green")
        #expect(choice.dark == "green")
        #expect(choice.encoded == "green/green")
    }

    @Test func eachThemeKeepsItsOwnColourAndNoneIsADash() {
        let choice = ChatChoice.decode("red/-", known: colours)
        #expect(choice.light == "red")
        #expect(choice.dark == nil)
        #expect(choice.encoded == "red/-")
        #expect(ChatChoice.decode("-/blue", known: colours).dark == "blue")
        #expect(ChatChoice.decode("", known: colours) == ChatChoice())
        #expect(ChatChoice.decode("purple", known: colours) == ChatChoice())
    }

    @Test func cyclingWalksThePresetsThenReturnsToNone() {
        var choice = ChatChoice()
        for expected in ["red", "orange", "yellow", "green", "cyan", "sky", "blue", "indigo", "violet", "magenta", "pink", nil] {
            choice = choice.cycling(dark: false, ids: colours)
            #expect(choice.light == expected)
        }
        #expect(choice.dark == nil)
    }

    @Test func theElevenPresetsAndTheirBackgroundsShareTheirNames() {
        #expect(ChatColours.all.count == 11)
        #expect(ChatBackgrounds.all.count == 11)
        for colour in ChatColours.all {
            #expect(ChatBackgrounds.background(colour.id) != nil)
            #expect(colour.light != colour.dark)
        }
    }
}
