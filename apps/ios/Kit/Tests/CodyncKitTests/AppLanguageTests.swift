import Observation
import Synchronization
import Foundation
import Testing
@testable import CodyncKit

struct AppLanguageTests {
    @Test func whatFillPutsInStaysAsWritten() {
        #expect(L10n.fill("{name} and {other}", ["name": "Build {other}", "other": "Reviewer"]) == "Build {other} and Reviewer")
        #expect(L10n.fill("{name} and {missing}", ["name": "Build"]) == "Build and {missing}")
    }

    @Test func theLanguageChoiceIsReadFromTheStoreItWasWrittenTo() {
        let name = "AppLanguageTests.\(UUID().uuidString)"
        let suite = UserDefaults(suiteName: name)!
        defer { suite.removePersistentDomain(forName: name) }
        #expect(AppLanguage.preference(in: suite) == .system)
        suite.set(AppLanguage.zh.rawValue, forKey: AppLanguage.storageKey)
        #expect(AppLanguage.preference(in: suite) == .zh)
        #expect(L10n.tr("Pair a computer", preference: AppLanguage.preference(in: suite), systemIsChinese: false) == "Ã©â€¦ÂÃ¥Â¯Â¹Ã¤Â¸â‚¬Ã¥ÂÂ°Ã§â€ÂµÃ¨â€žâ€˜")
    }

    @Test func theSharedLanguageStoreIsTheAppGroup() {
        let probe = "appLanguageProbe.\(UUID().uuidString)"
        defer { AppLanguage.defaults.removeObject(forKey: probe) }
        AppLanguage.defaults.set("1", forKey: probe)
        let group = UserDefaults(suiteName: SharedStore.appGroup)
        #expect(group?.string(forKey: probe) == "1")
        #expect(UserDefaults.standard.string(forKey: probe) == nil)
    }

    @Test func chineseFollowsTheChoiceNotTheOtherLanguage() {
        #expect(AppLanguage.usesChinese(preference: .zh, systemIsChinese: false))
        #expect(AppLanguage.usesChinese(preference: .zhHant, systemIsChinese: false))
        #expect(!AppLanguage.usesChinese(preference: .en, systemIsChinese: true))
        #expect(AppLanguage.usesChinese(preference: .system, systemIsChinese: true))
        #expect(!AppLanguage.usesChinese(preference: .system, systemIsChinese: false))
    }

    @Test func systemChineseIsThePreferredLanguagePrefix() {
        #expect(AppLanguage.systemIsChinese(languages: ["zh-Hans-CN", "en"]))
        #expect(AppLanguage.systemIsChinese(languages: ["zh-Hant-TW"]))
        #expect(AppLanguage.systemScript(languages: ["zh-Hant-TW"]) == .hant)
        #expect(AppLanguage.systemScript(languages: ["zh-TW"]) == .hant)
        #expect(AppLanguage.systemScript(languages: ["zh-HK"]) == .hant)
        #expect(AppLanguage.systemScript(languages: ["zh-Hans-CN"]) == .hans)
        #expect(AppLanguage.systemScript(languages: ["zh-CN"]) == .hans)
        #expect(AppLanguage.systemScript(languages: ["en-US"]) == nil)
        #expect(!AppLanguage.systemIsChinese(languages: ["en-US", "zh-Hans"]))
        #expect(!AppLanguage.systemIsChinese(languages: []))
    }

    @Test func phrasesUseSimplifiedChineseOnlyWhenThatLanguageIsActive() {
        #expect(L10n.tr("Pair a computer", preference: .zh, systemIsChinese: false) == "Ã©â€¦ÂÃ¥Â¯Â¹Ã¤Â¸â‚¬Ã¥ÂÂ°Ã§â€ÂµÃ¨â€žâ€˜")
        #expect(L10n.tr("Pair a computer", preference: .en, systemIsChinese: true) == "Pair a computer")
        #expect(L10n.tr("Pair a computer", preference: .system, systemIsChinese: true, systemLanguages: ["zh-Hans-CN"]) == "Ã©â€¦ÂÃ¥Â¯Â¹Ã¤Â¸â‚¬Ã¥ÂÂ°Ã§â€ÂµÃ¨â€žâ€˜")
        #expect(L10n.tr("Settings", preference: .system, systemIsChinese: true, systemLanguages: ["zh-Hant-TW"]) == "Ã¨Â¨Â­Ã¥Â®Å¡")
        #expect(L10n.tr("Not a phrase", preference: .zh, systemIsChinese: false) == "Not a phrase")
        #expect(AppLanguage.menuTitle(.zh, preference: .en, systemIsChinese: true) == "Ã§Â®â‚¬Ã¤Â½â€œÃ¤Â¸Â­Ã¦â€“â€¡")
        #expect(AppLanguage.menuTitle(.zhHant, preference: .en, systemIsChinese: true) == "Ã§Â¹ÂÃ©Â«â€Ã¤Â¸Â­Ã¦â€“â€¡")
        #expect(AppLanguage.menuTitle(.system, preference: .zh, systemIsChinese: false) == "Ã¨Â·Å¸Ã©Å¡ÂÃ§Â³Â»Ã§Â»Å¸")
    }

    @Test func usageCaptionAndScannerHintTranslateAsTheirOwnPhrases() {
        #expect(L10n.tr("Weekly", preference: .zh, systemIsChinese: false) == "Ã¦Â¯ÂÃ¥â€˜Â¨")
        #expect(L10n.tr("highest usage", preference: .zh, systemIsChinese: false) == "Ã§â€Â¨Ã©â€¡ÂÃ¦Å“â‚¬Ã©Â«Ëœ")
        #expect(L10n.tr("Weekly Ã‚Â· highest usage", preference: .zh, systemIsChinese: false) == "Weekly Ã‚Â· highest usage")
        #expect(L10n.tr("Find nearby barcodes.", preference: .zh, systemIsChinese: false) == "Ã¥Â¯Â»Ã¦â€°Â¾Ã©â„¢â€žÃ¨Â¿â€˜Ã§Å¡â€žÃ¦ÂÂ¡Ã¥Â½Â¢Ã§Â ÂÃ£â‚¬â€š")
        #expect(L10n.tr("Open Codync to connect a computer and check {provider} usage.", preference: .zh, systemIsChinese: false)
            == "Ã¦â€°â€œÃ¥Â¼â‚¬ CodyncÃ¯Â¼Å’Ã¨Â¿Å¾Ã¦Å½Â¥Ã¤Â¸â‚¬Ã¥ÂÂ°Ã§â€ÂµÃ¨â€žâ€˜Ã¥ÂÅ½Ã¥ÂÂ³Ã¥ÂÂ¯Ã¦Å¸Â¥Ã§Å“â€¹ {provider} Ã§â€Â¨Ã©â€¡ÂÃ£â‚¬â€š")
    }

    @Test func settingsAndBotChromeTranslateAsTheirOwnPhrases() {
        #expect(L10n.tr("Personal", preference: .zh, systemIsChinese: false) == "Ã¤Â¸ÂªÃ¤ÂºÂº")
        #expect(L10n.tr("This Mac", preference: .zh, systemIsChinese: false) == "Ã¨Â¿â„¢Ã¥ÂÂ° Mac")
        #expect(L10n.tr("Show in notch", preference: .zh, systemIsChinese: false) == "Ã¥Å“Â¨Ã¥Â±ÂÃ¥Â¹â€¢Ã©Â¡Â¶Ã©Æ’Â¨Ã¦ËœÂ¾Ã§Â¤Âº")
        #expect(L10n.tr("Show in Notch", preference: .zh, systemIsChinese: false) == "Ã¥Å“Â¨Ã¥Â±ÂÃ¥Â¹â€¢Ã©Â¡Â¶Ã©Æ’Â¨Ã¦ËœÂ¾Ã§Â¤Âº")
        #expect(L10n.tr("Voice input shortcut", preference: .zh, systemIsChinese: false) == "Ã¨Â¯Â­Ã©Å¸Â³Ã¨Â¾â€œÃ¥â€¦Â¥Ã¥Â¿Â«Ã¦ÂÂ·Ã©â€Â®")
        #expect(L10n.tr("Right Ã¢Å’Ëœ twice", preference: .zh, systemIsChinese: false) == "Ã¥ÂÂ³ Ã¢Å’Ëœ Ã¤Â¸Â¤Ã¦Â¬Â¡")
        #expect(L10n.tr("Claude Code (not installed)", preference: .zh, systemIsChinese: false) == "Claude CodeÃ¯Â¼Ë†Ã¦Å“ÂªÃ¥Â®â€°Ã¨Â£â€¦Ã¯Â¼â€°")
        #expect(L10n.tr("Copy template", preference: .zh, systemIsChinese: false) == "Ã¥Â¤ÂÃ¥Ë†Â¶Ã¦Â¨Â¡Ã¦ÂÂ¿")
        #expect(L10n.tr("Siri and Dictation are disabled", preference: .zh, systemIsChinese: false) == "Siri Ã¥â€™Å’Ã¥ÂÂ¬Ã¥â€ â„¢Ã¥Â·Â²Ã¥â€¦Â³Ã©â€”Â­")
        #expect(L10n.tr("the Codync cloud refused (403 forbidden): Sign in on the computer to use remote screen relay.", preference: .zh, systemIsChinese: false) == "Codync Ã¤Âºâ€˜Ã¦â€¹â€™Ã§Â»ÂÃ¤Âºâ€ Ã¯Â¼Ë†403 Ã§Â¦ÂÃ¦Â­Â¢Ã¯Â¼â€°Ã¯Â¼Å¡Ã¨Â¯Â·Ã¥â€¦Ë†Ã¥Å“Â¨Ã§â€ÂµÃ¨â€žâ€˜Ã¤Â¸Å Ã§â„¢Â»Ã¥Â½â€¢Ã¯Â¼Å’Ã¦â€°ÂÃ¨Æ’Â½Ã¤Â½Â¿Ã§â€Â¨Ã¨Â¿Å“Ã§Â¨â€¹Ã¥Â±ÂÃ¥Â¹â€¢Ã¤Â¸Â­Ã§Â»Â§Ã£â‚¬â€š")
        #expect(L10n.tr("Privacy", preference: .zh, systemIsChinese: false) == "Ã©Å¡ÂÃ§Â§Â")
        #expect(L10n.tr("Share usage analytics", preference: .zh, systemIsChinese: false) == "Ã¥â€¦Â±Ã¤ÂºÂ«Ã§â€Â¨Ã©â€¡ÂÃ¥Ë†â€ Ã¦Å¾Â")
        #expect(L10n.tr("Personal workspace", preference: .zh, systemIsChinese: false) == "Ã¤Â¸ÂªÃ¤ÂºÂºÃ¥Â·Â¥Ã¤Â½Å“Ã¥Å’Âº")
        #expect(L10n.tr("Fast mode", preference: .zh, systemIsChinese: false) == "Ã¥Â¿Â«Ã©â‚¬Å¸Ã¦Â¨Â¡Ã¥Â¼Â")
        #expect(L10n.tr("Use the computer", preference: .zh, systemIsChinese: false) == "Ã¤Â½Â¿Ã§â€Â¨Ã§â€ÂµÃ¨â€žâ€˜")
        #expect(L10n.tr("{size} pt (Default)", preference: .zh, systemIsChinese: false) == "{size} ptÃ¯Â¼Ë†Ã©Â»ËœÃ¨Â®Â¤Ã¯Â¼â€°")
        #expect(L10n.tr("Cursor Ã‚Â· Personal workspace", preference: .zh, systemIsChinese: false) == "Cursor Ã‚Â· Personal workspace")
    }

    @Test func chatMenuAndMemoryTranslateAsTheirOwnPhrases() {
        #expect(L10n.tr("Full conversation", preference: .zh, systemIsChinese: false) == "Ã¥Â®Å’Ã¦â€¢Â´Ã¥Â¯Â¹Ã¨Â¯Â")
        #expect(L10n.tr("Settings", preference: .zh, systemIsChinese: false) == "Ã¨Â®Â¾Ã§Â½Â®")
        #expect(L10n.tr("Settings", preference: .zhHant, systemIsChinese: false) == "Ã¨Â¨Â­Ã¥Â®Å¡")
        #expect(L10n.tr("Files", preference: .zhHant, systemIsChinese: false) == "Ã¦Âªâ€Ã¦Â¡Ë†")
        #expect(L10n.tr("Delete bot", preference: .zh, systemIsChinese: false) == "Ã¥Ë†Â Ã©â„¢Â¤ Bot")
        #expect(L10n.tr("Connected", preference: .zh, systemIsChinese: false) == "Ã¥Â·Â²Ã¨Â¿Å¾Ã¦Å½Â¥")
        #expect(L10n.tr("Ask {name}", preference: .zh, systemIsChinese: false) == "Ã©â€”Â® {name}")
        #expect(L10n.tr("Said", preference: .zh, systemIsChinese: false) == "Ã¨Â¯Â´Ã¤Âºâ€ ")
        #expect(L10n.tr("Stopped.", preference: .zh, systemIsChinese: false) == "Ã¥Â·Â²Ã¥ÂÅ“Ã¦Â­Â¢Ã£â‚¬â€š")
        #expect(L10n.tr("Today", preference: .zh, systemIsChinese: false) == "Ã¤Â»Å Ã¥Â¤Â©")
        #expect(L10n.tr("Yesterday", preference: .zh, systemIsChinese: false) == "Ã¦ËœÂ¨Ã¥Â¤Â©")
        #expect(L10n.tr("No dreams yet.", preference: .zh, systemIsChinese: false) == "Ã¨Â¿ËœÃ¦Â²Â¡Ã¦Å“â€°Ã¦Â¢Â¦Ã¥Â¢Æ’Ã£â‚¬â€š")
        #expect(L10n.tr("Standing instructions", preference: .zh, systemIsChinese: false) == "Ã¥Â¸Â¸Ã©Â©Â»Ã¦Å’â€¡Ã¤Â»Â¤")
        #expect(L10n.tr("Agent", preference: .zh, systemIsChinese: false) == "Ã¦â„¢ÂºÃ¨Æ’Â½Ã¤Â½â€œ")
        #expect(L10n.tr("Manage the team", preference: .zh, systemIsChinese: false) == "Ã§Â®Â¡Ã§Ââ€ Ã¥â€ºÂ¢Ã©ËœÅ¸")
        #expect(L10n.tr("Skills", preference: .zh, systemIsChinese: false) == "Ã¦Å â‚¬Ã¨Æ’Â½")
        #expect(L10n.tr("Search agents, connectors and skills", preference: .zh, systemIsChinese: false) == "Ã¦ÂÅ“Ã§Â´Â¢Ã¦â„¢ÂºÃ¨Æ’Â½Ã¤Â½â€œÃ£â‚¬ÂÃ¨Â¿Å¾Ã¦Å½Â¥Ã¥â„¢Â¨Ã¥â€™Å’Ã¦Å â‚¬Ã¨Æ’Â½")
        #expect(L10n.tr("Apps", preference: .zh, systemIsChinese: false) == "Ã¥Âºâ€Ã§â€Â¨")
        #expect(L10n.tr("Agents", preference: .zh, systemIsChinese: false) == "Ã¦â„¢ÂºÃ¨Æ’Â½Ã¤Â½â€œ")
        #expect(L10n.tr("Installed", preference: .zh, systemIsChinese: false) == "Ã¥Â·Â²Ã¥Â®â€°Ã¨Â£â€¦")
        #expect(L10n.tr("Not installed", preference: .zh, systemIsChinese: false) == "Ã¦Å“ÂªÃ¥Â®â€°Ã¨Â£â€¦")
        #expect(L10n.tr("Set up", preference: .zh, systemIsChinese: false) == "Ã¨Â®Â¾Ã§Â½Â®")
        #expect(L10n.tr("Writing a replyÃ¢â‚¬Â¦", preference: .zh, systemIsChinese: false) == "Ã¦Â­Â£Ã¥Å“Â¨Ã¥â€ â„¢Ã¥â€ºÅ¾Ã¥Â¤ÂÃ¢â‚¬Â¦")
        #expect(L10n.tr("Downloading Codebuddy CodeÃ¢â‚¬Â¦", preference: .zh, systemIsChinese: false) == "Ã¦Â­Â£Ã¥Å“Â¨Ã¤Â¸â€¹Ã¨Â½Â½ Codebuddy CodeÃ¢â‚¬Â¦")
        #expect(L10n.tr("Needs approval: Bash", preference: .zh, systemIsChinese: false) == "Ã©Å“â‚¬Ã¨Â¦ÂÃ¦â€°Â¹Ã¥â€¡â€ Ã¯Â¼Å¡Bash")
        #expect(L10n.tr("Storage", preference: .zh, systemIsChinese: false) == "Ã¥Â­ËœÃ¥â€šÂ¨")
        #expect(L10n.tr("New skill", preference: .zh, systemIsChinese: false) == "Ã¦â€“Â°Ã¥Â»ÂºÃ¦Å â‚¬Ã¨Æ’Â½")
        #expect(L10n.tr("Composio API key", preference: .zh, systemIsChinese: false) == "Composio API Ã¥Â¯â€ Ã©â€™Â¥")
        #expect(L10n.tr("When to use it", preference: .zh, systemIsChinese: false) == "Ã¤Â½â€¢Ã¦â€”Â¶Ã¤Â½Â¿Ã§â€Â¨")
        #expect(L10n.tr("Add connector", preference: .zh, systemIsChinese: false) == "Ã¦Â·Â»Ã¥Å Â Ã¨Â¿Å¾Ã¦Å½Â¥Ã¥â„¢Â¨")
        #expect(L10n.tr("Name your group", preference: .zh, systemIsChinese: false) == "Ã§Â»â„¢Ã§Â¾Â¤Ã¨ÂÅ Ã¨ÂµÂ·Ã¤Â¸ÂªÃ¥ÂÂÃ¥Â­â€”")
        #expect(L10n.tr("Nothing installed", preference: .zh, systemIsChinese: false) == "Ã¨Â¿ËœÃ¦Â²Â¡Ã¦Å“â€°Ã¥Â®â€°Ã¨Â£â€¦")
        #expect(L10n.tr("No setup needed.", preference: .zh, systemIsChinese: false) == "Ã¦â€”Â Ã©Å“â‚¬Ã¨Â®Â¾Ã§Â½Â®Ã£â‚¬â€š")
        #expect(L10n.tr("FORWARDXU-MC3 Ã‚Â· Connected", preference: .zh, systemIsChinese: false) == "FORWARDXU-MC3 Ã‚Â· Connected")
    }
}


@Test func additionalLanguagesResolveRegionsAndLoadBundledCatalogs() {
    for language in [AppLanguage.ja, .ko, .fr, .de, .es, .ru, .it] {
        #expect(AppLanguage.resolved(preference: .system, languages: [language.rawValue + "-XX"]) == language)
        #expect(AppLanguage.resolved(preference: language, languages: ["zh-TW"]) == language)
        let translated = L10n.tr("Settings", preference: language, systemIsChinese: false)
        #expect(translated != "Settings")
        #expect(L10n.tr("Settings", preference: .system, systemIsChinese: false, systemLanguages: [language.rawValue]) == translated)
        #expect(L10n.tr("An unknown phrase", preference: language) == "An unknown phrase")
        #expect(AppLanguage.locale(preference: language, systemIsChinese: true).language.languageCode?.identifier == language.rawValue)
    }
    #expect(AppLanguage.resolved(preference: .system, languages: ["nl-NL"]) == .en)
    #expect(AppLanguage.resolved(preference: .system, languages: ["ru_RU.UTF-8"]) == .ru)
    #expect(AppLanguage.menuTitle(.ru, preference: .en, systemIsChinese: true) == "ÃÂ Ã‘Æ’Ã‘ÂÃ‘ÂÃÂºÃÂ¸ÃÂ¹")
    #expect(AppLanguage.resolved(preference: .system, languages: ["it_IT.UTF-8"]) == .it)
    #expect(AppLanguage.menuTitle(.it, preference: .en, systemIsChinese: true) == "Italiano")

}


    @Test func brazilianPortuguesePersistsAndLoadsItsBundledCatalog() {
        let name = "AppLanguageTests.Portuguese.\(UUID().uuidString)"
        let suite = UserDefaults(suiteName: name)!
        defer { suite.removePersistentDomain(forName: name) }
        AppLanguage.setPreference(.ptBR, in: suite)
        #expect(suite.string(forKey: AppLanguage.storageKey) == "pt-BR")
        #expect(AppLanguage.preference(in: suite) == .ptBR)
        #expect(AppLanguage(rawValue: "pt-BR") == .ptBR)
        #expect(AppLanguage.ptBR.catalogName == "pt-BR")
        #expect(Array(AppLanguage.allCases.suffix(4)) == [.es, .ptBR, .ru, .ar])
        #expect(AppLanguage.menuTitle(.ptBR, preference: .en, systemIsChinese: true) == "PortuguÃƒÂªs (Brasil)")
        #expect(!AppLanguage.usesChinese(preference: .ptBR, systemIsChinese: true))
        let locale = AppLanguage.locale(preference: .ptBR, systemIsChinese: true)
        #expect(locale.language.languageCode?.identifier == "pt")
        #expect(locale.region?.identifier == "BR")
        let translated = L10n.tr("Settings", preference: AppLanguage.preference(in: suite), systemIsChinese: false)
        #expect(translated != "Settings")
        #expect(L10n.tr("An unknown phrase", preference: .ptBR) == "An unknown phrase")
        for tag in ["pt", "pt-BR", "pt_BR.UTF-8", "pt-PT", "pt-AO", "pt-MZ", "PT_cv.UTF-8"] {
            #expect(AppLanguage.resolved(preference: .system, languages: [tag]) == .ptBR)
            #expect(L10n.tr("Settings", preference: .system, systemIsChinese: false, systemLanguages: [tag]) == translated)
        }
        #expect(AppLanguage.resolved(preference: .ptBR, languages: ["zh-TW"]) == .ptBR)
        #expect(L10n.tr("Settings", preference: .en, systemLanguages: ["pt-BR"]) == "Settings")
        #expect(AppLanguage.resolved(preference: .system, languages: ["en-US"]) == .en)
        #expect(AppLanguage.resolved(preference: .system, languages: []) == .en)

}
    @Test func changingLanguageInvalidatesObservedTranslationConsumers() {
        let store = UserDefaults(suiteName: "language-observation-\(UUID().uuidString)")!
        let changed = Mutex(false)
        withObservationTracking {
            _ = AppLanguage.preference(in: store)
        } onChange: {
            changed.withLock { $0 = true }
        }
        AppLanguage.setPreference(.ja, in: store)
        #expect(changed.withLock { $0 })
        #expect(AppLanguage.preference(in: store) == .ja)
        store.removeObject(forKey: AppLanguage.storageKey)
    }

    @Test func arabicResolvesMirrorsAndLoadsItsBundledCatalog() {
        #expect(AppLanguage.resolved(preference: .system, languages: ["ar-SA"]) == .ar)
        #expect(AppLanguage.resolved(preference: .ar, languages: ["zh-TW"]) == .ar)
        #expect(AppLanguage.menuTitle(.ar, preference: .en, systemIsChinese: true) == "Ã˜Â§Ã™â€žÃ˜Â¹Ã˜Â±Ã˜Â¨Ã™Å Ã˜Â©")
        #expect(!AppLanguage.usesChinese(preference: .ar, systemIsChinese: true))
        #expect(AppLanguage.readsRightToLeft(preference: .ar, languages: ["en-US"]))
        #expect(AppLanguage.readsRightToLeft(preference: .system, languages: ["ar-EG"]))
        #expect(!AppLanguage.readsRightToLeft(preference: .es, languages: ["ar-EG"]))
        #expect(AppLanguage.locale(preference: .ar, systemIsChinese: true).language.languageCode?.identifier == "ar")
        #expect(L10n.tr("Settings", preference: .ar, systemIsChinese: false) != "Settings")
        #expect(L10n.tr("An unknown phrase", preference: .ar) == "An unknown phrase")
}
    @Test func botRoutineAndMessageSlotsKeepWhatTheUserWrote() {
        func ru(_ text: String) -> String { L10n.tr(text, preference: .ru, systemIsChinese: false) }
        #expect(ru("Created bot connected") == "ÃÂ¡ÃÂ¾ÃÂ·ÃÂ´ÃÂ°ÃÂ½ ÃÂ±ÃÂ¾Ã‘â€š connected")
        #expect(ru("Created bot Reviewer").hasSuffix("Reviewer"))
        #expect(ru("Routine Ã‚Â· ÃÅ¸Ã‘â‚¬ÃÂ¾ÃÂ²ÃÂµÃ‘â‚¬ÃÂºÃÂ° ÃÂ¿ÃÂ¾Ã‘â€¡Ã‘â€šÃ‘â€¹") == "ÃÂ Ã‘Æ’Ã‘â€šÃÂ¸ÃÂ½ÃÂ° Ã‚Â· ÃÅ¸Ã‘â‚¬ÃÂ¾ÃÂ²ÃÂµÃ‘â‚¬ÃÂºÃÂ° ÃÂ¿ÃÂ¾Ã‘â€¡Ã‘â€šÃ‘â€¹")
        #expect(ru("Routine Ã‚Â· Settings") == "ÃÂ Ã‘Æ’Ã‘â€šÃÂ¸ÃÂ½ÃÂ° Ã‚Â· Settings")
        #expect(ru("Message from Reviewer: Settings") == "ÃÂ¡ÃÂ¾ÃÂ¾ÃÂ±Ã‘â€°ÃÂµÃÂ½ÃÂ¸ÃÂµ ÃÂ¾Ã‘â€š Reviewer: Settings")
    }

@Test func filledTemplatesPreserveNamesAndSubstituteValuesOnlyOnce() {
        for language in AppLanguage.allCases where language != .system && language != .en {
            for name in ["Plan", "connected", "Standby", "$&", "{hint}"] {
                let expected = L10n.tr("Routine Ã‚Â· {name}", preference: language)
                    .replacingOccurrences(of: "{name}", with: name)
                #expect(L10n.tr("Routine Ã‚Â· \(name)", preference: language) == expected)
                let title = L10n.tr("Message about Ã¢â‚¬Å“{title}Ã¢â‚¬Â", preference: language)
                    .replacingOccurrences(of: "{title}", with: name)
                #expect(L10n.tr("Message about Ã¢â‚¬Å“\(name)Ã¢â‚¬Â", preference: language) == title)
            }
            let hint = "Install Claude Code and run `claude auth login`."
            let expected = L10n.tr("{name} isn't set up on this computer. {hint}", preference: language)
                .replacingOccurrences(of: "{hint}", with: L10n.tr(hint, preference: language))
                .replacingOccurrences(of: "{name}", with: "{hint}")
            #expect(L10n.tr("{hint} isn't set up on this computer. \(hint)", preference: language) == expected)
        }
    }

