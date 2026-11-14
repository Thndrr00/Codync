package com.codync.android

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.codync.android.design.*

/**
 * The Settings destination: what belongs to this phone alone (appearance, language, navigation,
 * security, notification permission, analytics, Start over; iOS `SettingsView`). Each computer's
 * own settings (voice chat, push, connection, hidden bots) are in Computer management
 * (`ComputersView`), which `computers` opens.
 */
@Composable internal fun SettingsView(
    state: AppState,
    store: AppStore,
    bottomInset: Dp = 0.dp,
    computers: (() -> Unit)? = null,
    active: Boolean = true,
    reselectToken: Int = 0,
) {
    val context = LocalContext.current
    val uri = LocalUriHandler.current
    val scroll = rememberScrollState()
    LaunchedEffect(context) { NavigationPreferences.ensureLoaded(context) }
    val navStyle by NavigationPreferences.style.collectAsState()
    val navAutoHide by NavigationPreferences.autoHide.collectAsState()
    var confirmStartOver by remember { mutableStateOf(false) }
    var aboutAnalytics by remember { mutableStateOf(false) }
    var channels by remember { mutableStateOf(false) }
    LaunchedEffect(reselectToken) { if (reselectToken > 0) scroll.animateScrollTo(0) }
    LaunchedEffect(active) { if (active) store.refreshCloud() }
    CardForm(Modifier.background(Palette.background).statusBarsPadding(), PaddingValues(start = 20.dp, top = 12.dp, end = 20.dp, bottom = bottomInset), scroll) {
        Text(tr("Settings"), Modifier.semantics { heading() }, style = Type.largeTitle.bold, color = Palette.text)
        if (computers != null) CardSection {
            row { RowLabel("Computer management", "desktopcomputer", chevron = true, modifier = Modifier.plainButton(action = computers)) }
            row { RowLabel("Channels", "paperplane", chevron = true, modifier = Modifier.plainButton { channels = true }) }
        }
        AppearanceSection()
        val appLanguage = LocalAppLanguage.current
        val setAppLanguage = LocalSetAppLanguage.current
        val systemChinese = LocalSystemChinese.current
        val phrases = LocalPhrases.current
        CardSection("Language", footer = "Choose the language for this device. Unsupported system languages use English.") {
            row {
                ValueRow("App language") {
                    ChoicePicker(appLanguage, AppLanguage.entries.map {
                        it to AppLanguage.menuTitle(it, appLanguage, systemChinese, phrases)
                    }, setAppLanguage)
                }
            }
        }
        ThinkingDetailsSettings()
        CardSection("Navigation", "Pick how the tab bar sits at the bottom of the screen on phones.") {
            row {
                ValueRow("Bar style") {
                    ChoicePicker(
                        navStyle,
                        NavBarStyle.entries.map { it to it.title },
                        { NavigationPreferences.setStyle(context, it) },
                        label = "Bar style",
                    )
                }
            }
            row {
                SwitchRow("Hide bar on scroll", isOn = navAutoHide, onChange = { NavigationPreferences.setAutoHide(context, it) })
            }
        }
        SecuritySection()
        NotificationsSection()
        AlertRulesSection(store)
        GettingStartedSection(state)
        EnterpriseSection(state, store)
        BrowserSection(state, store)
        AuditLogSection(state, store)
        CardSection(footer = "Which features you use, never your messages, code or files. Helps us spot bugs sooner and make Codync better for you.") {
            row {
                val analyticsLabel = tr("Share usage analytics")
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(analyticsLabel, style = Type.body, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    IconButton("About usage analytics", "info.circle") { aboutAnalytics = true }
                    Box(Modifier.weight(1f))
                    CodyncSwitch(Analytics.enabled, Analytics::setEnabled, Modifier.width(34.dp)
                        .semantics { contentDescription = analyticsLabel }) {}
                }
            }
        }
        CardSection(footer = "Look around with sample bots, a session, an approval and a routine. Nothing is saved or sent, and leaving brings back exactly what you had.") {
            row {
                val demo by DemoMode.active.collectAsState()
                SecondaryButton(if (demo) "Exit demo mode" else "Try demo mode") { if (demo) DemoMode.exit() else DemoMode.enter() }
            }
        }
        CardSection {
            row { ValueRow("App version", BuildConfig.VERSION_NAME) }
            row { LinkButton("Source code") { uri.open("https://github.com/leepokai/Codync", OutboundLink.Github) } }
        }
        CardSection {
            row {
                Row(Modifier.fillMaxWidth().plainButton(enabled = "session" !in state.busy) { confirmStartOver = true },
                    verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Symbol("arrow.counterclockwise", size = 20.dp, tint = Palette.danger)
                    Text(tr("Start over"), style = Type.body, color = Palette.danger)
                }
            }
        }
    }
    CodyncSheet(aboutAnalytics, { aboutAnalytics = false }, detents = setOf(SheetDetent.Medium, SheetDetent.Expanded)) { AnalyticsSheet() }
    CodyncSheet(channels, { channels = false }, detents = setOf(SheetDetent.Medium, SheetDetent.Expanded)) { ChannelsSheet(state, store) }
    CodyncDialog("Start over?", confirmStartOver, { confirmStartOver = false },
        "Signs out of every account and forgets every computer on this phone, then shows the welcome again. Your computers keep their bots and chats; pair again to use them.") {
        listOf(DialogAction("Start over", destructive = true, action = store::startOver))
    }
}

/** The host's alert rules: which kinds reach the phone. Quiet hours stay in the desktop's Diagnostics for now. */
@Composable internal fun AlertRulesSection(store: AppStore) {
    var rules by remember { mutableStateOf<NotificationRules?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) {
        try { rules = store.notificationRules() }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message }
    }
    fun change(next: NotificationRules) {
        rules = next
        scope.launch {
            try { rules = store.setNotificationRules(next); error = null }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message }
        }
    }
    CardSection("Push alert rules", "Which of the computer's alerts reach this phone. An approval always arrives, even in quiet hours.") {
        val current = rules
        if (current == null) {
            row { Text(error ?: "Loading…", style = Type.body, color = Palette.secondary) }
        } else {
            row { SwitchRow("Task finished", isOn = current.done, onChange = { change(current.copy(done = it)) }) }
            row { SwitchRow("Needs you", "An approval or a question; always arrives.", isOn = current.needsInput, onChange = { change(current.copy(needsInput = it)) }) }
            row { SwitchRow("Task failed", isOn = current.failed, onChange = { change(current.copy(failed = it)) }) }
            row { SwitchRow("Task paused", isOn = current.paused, onChange = { change(current.copy(paused = it)) }) }
            row {
                SwitchRow("Quiet hours", "Finished, failed and paused tasks wait until it ends.",
                    isOn = current.quietStart != current.quietEnd,
                    onChange = { change(current.copy(quietStart = if (it) 22 else 0, quietEnd = if (it) 8 else 0)) })
            }
            if (current.quietStart != current.quietEnd) {
                row { HourRow("From", current.quietStart) { change(current.copy(quietStart = it)) } }
                row { HourRow("Until", current.quietEnd) { change(current.copy(quietEnd = it)) } }
            }
        }
        if (error != null && current != null) row { Text(error ?: "", style = Type.footnote, color = Palette.danger) }
    }
}

/** One quiet-hour bound: a dropdown of the day's hours, in 12-hour labels like the phones show. */
@Composable private fun HourRow(title: String, hour: Int, onChange: (Int) -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Text(title, style = Type.body, color = Palette.text)
        Box(Modifier.weight(1f))
        DropdownMenu({ (0..23).map { value -> MenuItem(hourLabel(value), selected = value == hour) { onChange(value) } } }) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(hourLabel(hour), style = Type.body, color = Palette.text)
                Symbol("chevron.up.chevron.down", size = 11.dp, tint = Palette.tertiary)
            }
        }
    }
}

private fun hourLabel(hour: Int): String {
    val shown = when {
        hour == 0 -> 12
        hour > 12 -> hour - 12
        else -> hour
    }
    return "$shown ${if (hour < 12) "AM" else "PM"}"
}

/** Whether Android lets Codync show notifications at all; each computer's push switch is on its card. */
@Composable private fun NotificationsSection() {
    val context = LocalContext.current
    var asked by remember { mutableStateOf(false) }
    val request = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        notificationsAnswered(context, granted, PromptSource.Settings); asked = true
    }
    val allowed = rememberOnResume { notificationsAllowed(context) } || asked && notificationsAllowed(context)
    CardSection("Notifications", "Get a result summary, a request for input, or a failure notice. Notification previews follow your Android settings.") {
        if (!BuildConfig.PUSH_CONFIGURED) row { Text("Push isn't set up in this build. Bots and chats still update while Codync is open.", style = Type.subheadline, color = Palette.secondary) }
        row {
            ValueRow("Notifications", if (allowed) "On" else if (asked) "Off in Settings" else "Turn on", Modifier.plainButton {
                if (!allowed && !asked && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) request.launch(Manifest.permission.POST_NOTIFICATIONS)
                else openNotificationSettings(context)
            })
        }
    }
}

