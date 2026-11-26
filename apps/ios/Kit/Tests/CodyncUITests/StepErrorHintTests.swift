import Testing
@testable import CodyncUI

@Test func aFailedStepNamesWhatToDoFromItsError() {
    #expect(StepErrorHint.hint(StepErrorHint.kind("Couldn't sign in to the account")) == "Sign in again in Settings, then run this step.")
    #expect(StepErrorHint.hint(StepErrorHint.kind("Field is required")) == "Give every path into this step the field, or mark it optional.")
    #expect(StepErrorHint.hint(StepErrorHint.kind("Something else went wrong")).contains("run it again"))
}

@Test func aStepErrorKindCoversTheKnownFailures() {
    #expect(StepErrorHint.kind("auth failed") == "signin")
    #expect(StepErrorHint.kind("unknown model") == "model")
    #expect(StepErrorHint.kind("quota reached") == "limit")
    #expect(StepErrorHint.kind("no answer") == "field")
    #expect(StepErrorHint.kind("mystery") == "other")
}
