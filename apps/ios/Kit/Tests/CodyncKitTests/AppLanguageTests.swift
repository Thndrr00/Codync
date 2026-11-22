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
        #expect(L10n.tr("Pair a computer", preference: AppLanguage.preference(in: suite), systemIsChinese: false) == "é…å¯¹ä¸€å°ç”µè„‘")
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
        #expect(L10n.tr("Pair a computer", preference: .zh, systemIsChinese: false) == "é…å¯¹ä¸€å°ç”µè„‘")
        #expect(L10n.tr("Pair a computer", preference: .en, systemIsChinese: true) == "Pair a computer")
        #expect(L10n.tr("Pair a computer", preference: .system, systemIsChinese: true, systemLanguages: ["zh-Hans-CN"]) == "é…å¯¹ä¸€å°ç”µè„‘")
        #expect(L10n.tr("Settings", preference: .system, systemIsChinese: true, systemLanguages: ["zh-Hant-TW"]) == "è¨­å®š")
        #expect(L10n.tr("Not a phrase", preference: .zh, systemIsChinese: false) == "Not a phrase")
        #expect(AppLanguage.menuTitle(.zh, preference: .en, systemIsChinese: true) == "ç®€ä½“ä¸­æ–‡")
        #expect(AppLanguage.menuTitle(.zhHant, preference: .en, systemIsChinese: true) == "ç¹é«”ä¸­æ–‡")
        #expect(AppLanguage.menuTitle(.system, preference: .zh, systemIsChinese: false) == "è·Ÿéšç³»ç»Ÿ")
    }

    @Test func usageCaptionAndScannerHintTranslateAsTheirOwnPhrases() {
        #expect(L10n.tr("Weekly", preference: .zh, systemIsChinese: false) == "æ¯å‘¨")
        #expect(L10n.tr("highest usage", preference: .zh, systemIsChinese: false) == "ç”¨é‡æœ€é«˜")
        #expect(L10n.tr("Weekly Â· highest usage", preference: .zh, systemIsChinese: false) == "Weekly Â· highest usage")
        #expect(L10n.tr("Find nearby barcodes.", preference: .zh, systemIsChinese: false) == "å¯»æ‰¾é™„è¿‘çš„æ¡å½¢ç ã€‚")
        #expect(L10n.tr("Open Codync to connect a computer and check {provider} usage.", preference: .zh, systemIsChinese: false)
            == "æ‰“å¼€ Codyncï¼Œè¿žæŽ¥ä¸€å°ç”µè„‘åŽå³å¯æŸ¥çœ‹ {provider} ç”¨é‡ã€‚")
    }

    @Test func settingsAndBotChromeTranslateAsTheirOwnPhrases() {
        #expect(L10n.tr("Personal", preference: .zh, systemIsChinese: false) == "ä¸ªäºº")
        #expect(L10n.tr("This Mac", preference: .zh, systemIsChinese: false) == "è¿™å° Mac")
        #expect(L10n.tr("Show in notch", preference: .zh, systemIsChinese: false) == "åœ¨å±å¹•é¡¶éƒ¨æ˜¾ç¤º")
        #expect(L10n.tr("Show in Notch", preference: .zh, systemIsChinese: false) == "åœ¨å±å¹•é¡¶éƒ¨æ˜¾ç¤º")
        #expect(L10n.tr("Voice input shortcut", preference: .zh, systemIsChinese: false) == "è¯­éŸ³è¾“å…¥å¿«æ·é”®")
        #expect(L10n.tr("Right âŒ˜ twice", preference: .zh, systemIsChinese: false) == "å³ âŒ˜ ä¸¤æ¬¡")
        #expect(L10n.tr("Claude Code (not installed)", preference: .zh, systemIsChinese: false) == "Claude Codeï¼ˆæœªå®‰è£…ï¼‰")
        #expect(L10n.tr("Copy template", preference: .zh, systemIsChinese: false) == "å¤åˆ¶æ¨¡æ¿")
        #expect(L10n.tr("Siri and Dictation are disabled", preference: .zh, systemIsChinese: false) == "Siri å’Œå¬å†™å·²å…³é—­")
        #expect(L10n.tr("the Codync cloud refused (403 forbidden): Sign in on the computer to use remote screen relay.", preference: .zh, systemIsChinese: false) == "Codync äº‘æ‹’ç»äº†ï¼ˆ403 ç¦æ­¢ï¼‰ï¼šè¯·å…ˆåœ¨ç”µè„‘ä¸Šç™»å½•ï¼Œæ‰èƒ½ä½¿ç”¨è¿œç¨‹å±å¹•ä¸­ç»§ã€‚")
        #expect(L10n.tr("Privacy", preference: .zh, systemIsChinese: false) == "éšç§")
        #expect(L10n.tr("Share usage analytics", preference: .zh, systemIsChinese: false) == "å…±äº«ç”¨é‡åˆ†æž")
        #expect(L10n.tr("Personal workspace", preference: .zh, systemIsChinese: false) == "ä¸ªäººå·¥ä½œåŒº")
        #expect(L10n.tr("Fast mode", preference: .zh, systemIsChinese: false) == "å¿«é€Ÿæ¨¡å¼")
        #expect(L10n.tr("Use the computer", preference: .zh, systemIsChinese: false) == "ä½¿ç”¨ç”µè„‘")
        #expect(L10n.tr("{size} pt (Default)", preference: .zh, systemIsChinese: false) == "{size} ptï¼ˆé»˜è®¤ï¼‰")
        #expect(L10n.tr("Cursor Â· Personal workspace", preference: .zh, systemIsChinese: false) == "Cursor Â· Personal workspace")
    }

    @Test func chatMenuAndMemoryTranslateAsTheirOwnPhrases() {
        #expect(L10n.tr("Full conversation", preference: .zh, systemIsChinese: false) == "å®Œæ•´å¯¹è¯")
        #expect(L10n.tr("Settings", preference: .zh, systemIsChinese: false) == "è®¾ç½®")
        #expect(L10n.tr("Settings", preference: .zhHant, systemIsChinese: false) == "è¨­å®š")
        #expect(L10n.tr("Files", preference: .zhHant, systemIsChinese: false) == "æª”æ¡ˆ")
        #expect(L10n.tr("Delete bot", preference: .zh, systemIsChinese: false) == "åˆ é™¤ Bot")
        #expect(L10n.tr("Connected", preference: .zh, systemIsChinese: false) == "å·²è¿žæŽ¥")
        #expect(L10n.tr("Ask {name}", preference: .zh, systemIsChinese: false) == "é—® {name}")
        #expect(L10n.tr("Said", preference: .zh, systemIsChinese: false) == "è¯´äº†")
        #expect(L10n.tr("Stopped.", preference: .zh, systemIsChinese: false) == "å·²åœæ­¢ã€‚")
        #expect(L10n.tr("Today", preference: .zh, systemIsChinese: false) == "ä»Šå¤©")
        #expect(L10n.tr("Yesterday", preference: .zh, systemIsChinese: false) == "æ˜¨å¤©")
        #expect(L10n.tr("No dreams yet.", preference: .zh, systemIsChinese: false) == "è¿˜æ²¡æœ‰æ¢¦å¢ƒã€‚")
        #expect(L10n.tr("Standing instructions", preference: .zh, systemIsChinese: false) == "å¸¸é©»æŒ‡ä»¤")
        #expect(L10n.tr("Agent", preference: .zh, systemIsChinese: false) == "æ™ºèƒ½ä½“")
        #expect(L10n.tr("Manage the team", preference: .zh, systemIsChinese: false) == "ç®¡ç†å›¢é˜Ÿ")
        #expect(L10n.tr("Skills", preference: .zh, systemIsChinese: false) == "æŠ€èƒ½")
        #expect(L10n.tr("Search agents, connectors and skills", preference: .zh, systemIsChinese: false) == "æœç´¢æ™ºèƒ½ä½“ã€è¿žæŽ¥å™¨å’ŒæŠ€èƒ½")
        #expect(L10n.tr("Apps", preference: .zh, systemIsChinese: false) == "åº”ç”¨")
        #expect(L10n.tr("Agents", preference: .zh, systemIsChinese: false) == "æ™ºèƒ½ä½“")
        #expect(L10n.tr("Installed", preference: .zh, systemIsChinese: false) == "å·²å®‰è£…")
        #expect(L10n.tr("Not installed", preference: .zh, systemIsChinese: false) == "æœªå®‰è£…")
        #expect(L10n.tr("Set up", preference: .zh, systemIsChinese: false) == "è®¾ç½®")
        #expect(L10n.tr("Writing a replyâ€¦", preference: .zh, systemIsChinese: false) == "æ­£åœ¨å†™å›žå¤â€¦")
        #expect(L10n.tr("Downloading Codebuddy Codeâ€¦", preference: .zh, systemIsChinese: false) == "æ­£åœ¨ä¸‹è½½ Codebuddy Codeâ€¦")
        #expect(L10n.tr("Needs approval: Bash", preference: .zh, systemIsChinese: false) == "éœ€è¦æ‰¹å‡†ï¼šBash")
        #expect(L10n.tr("Storage", preference: .zh, systemIsChinese: false) == "å­˜å‚¨")
        #expect(L10n.tr("New skill", preference: .zh, systemIsChinese: false) == "æ–°å»ºæŠ€èƒ½")
        #expect(L10n.tr("Composio API key", preference: .zh, systemIsChinese: false) == "Composio API å¯†é’¥")
        #expect(L10n.tr("When to use it", preference: .zh, systemIsChinese: false) == "ä½•æ—¶ä½¿ç”¨")
        #expect(L10n.tr("Add connector", preference: .zh, systemIsChinese: false) == "æ·»åŠ è¿žæŽ¥å™¨")
        #expect(L10n.tr("Name your group", preference: .zh, systemIsChinese: false) == "ç»™ç¾¤èŠèµ·ä¸ªåå­—")
        #expect(L10n.tr("Nothing installed", preference: .zh, systemIsChinese: false) == "è¿˜æ²¡æœ‰å®‰è£…")
        #expect(L10n.tr("No setup needed.", preference: .zh, systemIsChinese: false) == "æ— éœ€è®¾ç½®ã€‚")
        #expect(L10n.tr("FORWARDXU-MC3 Â· Connected", preference: .zh, systemIsChinese: false) == "FORWARDXU-MC3 Â· Connected")
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
    #expect(AppLanguage.menuTitle(.ru, preference: .en, systemIsChinese: true) == "Ð ÑƒÑÑÐºÐ¸Ð¹")
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
        #expect(Array(AppLanguage.allCases.suffix(4)) == [.es, .ptBR, .ru, .it])
        #expect(AppLanguage.menuTitle(.ptBR, preference: .en, systemIsChinese: true) == "PortuguÃªs (Brasil)")
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
        #expect(AppLanguage.menuTitle(.ar, preference: .en, systemIsChinese: true) == "Ø§Ù„Ø¹Ø±Ø¨ÙŠØ©")
        #expect(!AppLanguage.usesChinese(preference: .ar, systemIsChinese: true))
        #expect(AppLanguage.readsRightToLeft(preference: .ar, languages: ["en-US"]))
        #expect(AppLanguage.readsRightToLeft(preference: .system, languages: ["ar-EG"]))
        #expect(!AppLanguage.readsRightToLeft(preference: .es, languages: ["ar-EG"]))
        #expect(AppLanguage.locale(preference: .ar, systemIsChinese: true).language.languageCode?.identifier == "ar")
        #expect(L10n.tr("Settings", preference: .ar, systemIsChinese: false) != "Settings")
        #expect(L10n.tr("An unknown phrase", preference: .ar) == "An unknown phrase")
    @Test func botRoutineAndMessageSlotsKeepWhatTheUserWrote() {
        func ru(_ text: String) -> String { L10n.tr(text, preference: .ru, systemIsChinese: false) }
        #expect(ru("Created bot connected") == "Ð¡Ð¾Ð·Ð´Ð°Ð½ Ð±Ð¾Ñ‚ connected")
        #expect(ru("Created bot Reviewer").hasSuffix("Reviewer"))
        #expect(ru("Routine Â· ÐŸÑ€Ð¾Ð²ÐµÑ€ÐºÐ° Ð¿Ð¾Ñ‡Ñ‚Ñ‹") == "Ð ÑƒÑ‚Ð¸Ð½Ð° Â· ÐŸÑ€Ð¾Ð²ÐµÑ€ÐºÐ° Ð¿Ð¾Ñ‡Ñ‚Ñ‹")
        #expect(ru("Routine Â· Settings") == "Ð ÑƒÑ‚Ð¸Ð½Ð° Â· Settings")
        #expect(ru("Message from Reviewer: Settings") == "Ð¡Ð¾Ð¾Ð±Ñ‰ÐµÐ½Ð¸Ðµ Ð¾Ñ‚ Reviewer: Settings")
    }
}

@Test func filledTemplatesPreserveNamesAndSubstituteValuesOnlyOnce() {
        for language in AppLanguage.allCases where language != .system && language != .en {
            for name in ["Plan", "connected", "Standby", "$&", "{hint}"] {
                let expected = L10n.tr("Routine Â· {name}", preference: language)
                    .replacingOccurrences(of: "{name}", with: name)
                #expect(L10n.tr("Routine Â· \(name)", preference: language) == expected)
                let title = L10n.tr("Message about â€œ{title}â€", preference: language)
                    .replacingOccurrences(of: "{title}", with: name)
                #expect(L10n.tr("Message about â€œ\(name)â€", preference: language) == title)
            }
            let hint = "Install Claude Code and run `claude auth login`."
            let expected = L10n.tr("{name} isn't set up on this computer. {hint}", preference: language)
                .replacingOccurrences(of: "{hint}", with: L10n.tr(hint, preference: language))
                .replacingOccurrences(of: "{name}", with: "{hint}")
            #expect(L10n.tr("{hint} isn't set up on this computer. \(hint)", preference: language) == expected)
        }
    }

}
