package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * The board, the saved teams and the read-only links this computer keeps (iOS `WorkspaceSheet`).
 * Every change is a host call, so every client sees the same board.
 */
@Composable
internal fun WorkspaceSheet(store: AppStore) {
    var tab by remember { mutableStateOf(WorkspaceTab.Board) }
    var tasks by remember { mutableStateOf<List<BoardTask>>(emptyList()) }
    var waiting by remember { mutableStateOf<List<WaitingItem>>(emptyList()) }
    var templates by remember { mutableStateOf<List<TeamTemplate>>(emptyList()) }
    var links by remember { mutableStateOf<List<SharedLink>>(emptyList()) }
    var feeds by remember { mutableStateOf<List<CalendarFeed>>(emptyList()) }
    var events by remember { mutableStateOf<List<CalendarEvent>>(emptyList()) }
    var emails by remember { mutableStateOf<List<EmailAccount>>(emptyList()) }
    var messages by remember { mutableStateOf<List<EmailMessage>>(emptyList()) }
    var mailAccount by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(true) }
    val scope = rememberCoroutineScope()

    suspend fun load() {
        loading = true
        try {
            tasks = store.boardTasks()
            waiting = store.waitingItems()
            templates = store.teamTemplates()
            links = store.sharedLinks()
            feeds = store.calendarFeeds()
            events = store.calendarAgenda()
            emails = store.emailAccounts()
            if (mailAccount.isEmpty()) mailAccount = emails.firstOrNull()?.id ?: ""
            error = null
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            error = failure.message
        } finally {
            loading = false
        }
    }

    fun run(block: suspend () -> Unit) {
        scope.launch {
            try {
                block()
                error = null
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                error = failure.message
            }
        }
    }

    LaunchedEffect(Unit) { load() }

    Column(Modifier.fillMaxWidth()) {
        ModalHeader("Workspace") { if (loading) Spinner() }
        CardForm {
            SegmentedChoice(tab, WorkspaceTab.entries.map { it to it.label }, select = { tab = it })
            when (tab) {
                WorkspaceTab.Board -> {
                    TodaySection(store) { error = it }
                    var title by remember { mutableStateOf("") }
                    var detail by remember { mutableStateOf("") }
                    var more by remember { mutableStateOf(false) }
                    var waitTitle by remember { mutableStateOf("") }
                    var waitWho by remember { mutableStateOf("") }
                    var waitDue by remember { mutableStateOf("") }
                    CardSection("Add a card", "A card can name the bot that owns it; moving it is yours to do.") {
                        row {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                FieldBox(title, { title = it }, "What needs doing")
                                if (more) FieldBox(detail, { detail = it }, "Detail (optional)", minLines = 2, maxLines = 4)
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                    LinkButton(if (more) "Fewer options" else "More options") { more = !more }
                                    Box(Modifier.weight(1f))
                                    SecondaryButton("Add", enabled = title.isNotBlank()) {
                                        val text = title.trim()
                                        run {
                                            tasks = store.boardTasks()
                                            store.saveBoardTask(text, detail.trim())
                                            tasks = store.boardTasks()
                                            title = ""
                                            detail = ""
                                            more = false
                                        }
                                    }
                                }
                            }
                        }
                    }
                    for (status in BoardTask.statuses) {
                        val column = tasks.filter { it.status == status }
                        if (column.isNotEmpty()) CardSection(statusTitle(status)) {
                            for (task in column) row {
                                BoardTaskRow(task,
                                    onMove = { next -> run {
                                        store.moveBoardTask(task.id, next)
                                        tasks = store.boardTasks()
                                    } },
                                    onDelete = { run {
                                        store.deleteBoardTask(task.id)
                                        tasks = store.boardTasks()
                                    } })
                            }
                        }
                    }
                    if (tasks.isEmpty() && !loading) CardSection {
                        row { Text("Nothing on the board yet.", style = Type.body, color = Palette.secondary) }
                    }
                    CardSection("Waiting on", "Replies, reviews and deliveries other people owe you. They show in the daily digest and a bot can list them.") {
                        row {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                FieldBox(waitTitle, { waitTitle = it }, "What are you waiting on")
                                FieldBox(waitWho, { waitWho = it }, "From (optional)")
                                FieldBox(waitDue, { waitDue = it }, "Due (2026-11-20, optional)")
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                    SecondaryButton("Add", enabled = waitTitle.isNotBlank()) {
                                        val due = waitDue.trim()
                                        val dueAt = if (due.isEmpty()) null else runCatching {
                                            SimpleDateFormat("yyyy-MM-dd", Locale.US).parse(due)?.time
                                        }.getOrNull()
                                        if (due.isNotEmpty() && dueAt == null) error = "The due date is a day, like 2026-11-20."
                                        else run {
                                            store.saveWaitingItem(waitTitle.trim(), waitWho.trim(), dueAt)
                                            waiting = store.waitingItems()
                                            waitTitle = ""; waitWho = ""; waitDue = ""
                                        }
                                    }
                                }
                            }
                        }
                        for (item in waiting.filter { !it.done }) row {
                            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                    Text(item.title, style = Type.body, color = Palette.text)
                                    val sub = listOf(item.counterpart, waitingDue(item)).filter { it.isNotEmpty() }.joinToString(" · ")
                                    if (sub.isNotEmpty()) Text(sub, style = Type.footnote, color = Palette.secondary)
                                }
                                LinkButton("Done") { run {
                                    store.setWaitingDone(item.id, true)
                                    waiting = store.waitingItems()
                                } }
                                LinkButton("Delete") { run {
                                    store.deleteWaitingItem(item.id)
                                    waiting = store.waitingItems()
                                } }
                            }
                        }
                        if (waiting.all { it.done } && !loading) row {
                            Text("Nothing is waiting on anyone.", style = Type.footnote, color = Palette.secondary)
                        }
                    }
                    MeetingsSection(store) { error = it }
                }
                WorkspaceTab.Teams -> {
                    if (templates.isEmpty() && !loading) CardSection {
                        row { Text("No saved teams yet. Save a group as a team on the computer.", style = Type.body, color = Palette.secondary) }
                    }
                    for (template in templates) CardSection(template.name, template.description.ifEmpty { null }) {
                        if (template.members.isNotEmpty()) row {
                            Text(template.members.joinToString(" · "), style = Type.footnote, color = Palette.secondary)
                        }
                        row {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                SecondaryButton("Add to this computer") { run { store.installTeamTemplate(template.id) } }
                            }
                        }
                    }
                }
                WorkspaceTab.Links -> {
                    val clipboard = LocalClipboardManager.current
                    val uri = LocalUriHandler.current
                    if (links.isEmpty() && !loading) CardSection {
                        row { Text("No read-only links. Publishing a document or artifact creates one.", style = Type.body, color = Palette.secondary) }
                    }
                    for (link in links) CardSection(link.title, "Anyone with the link can read it until it expires or is revoked.") {
                        row { Text(link.url, style = Type.footnote, color = Palette.secondary, maxLines = 1) }
                        row {
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                SecondaryButton("Copy link") { clipboard.setText(AnnotatedString(link.url)) }
                                SecondaryButton("Open") { if (link.url.isNotEmpty()) uri.openUri(link.url) }
                                Box(Modifier.weight(1f))
                                SecondaryButton("Revoke") { run {
                                    store.revokeSharedLink(link.token)
                                    links = store.sharedLinks()
                                } }
                            }
                        }
                    }
                }
                WorkspaceTab.Calendar -> {
                    var feedName by remember { mutableStateOf("") }
                    var feedUrl by remember { mutableStateOf("") }
                    CardSection("Add a calendar", "Paste the .ics address from Google, Apple or Outlook. It is read only.") {
                        row {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                FieldBox(feedName, { feedName = it }, "Name (optional)")
                                FieldBox(feedUrl, { feedUrl = it }, "https://…/calendar.ics")
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                    SecondaryButton("Add calendar", enabled = feedUrl.isNotBlank()) { run {
                                        val feed = store.addCalendarFeed(feedName.trim().ifEmpty { "Calendar" }, feedUrl.trim())
                                        feeds = feeds + feed
                                        events = store.calendarAgenda()
                                        feedName = ""
                                        feedUrl = ""
                                    } }
                                }
                            }
                        }
                    }
                    if (feeds.isEmpty()) CardSection {
                        row { Text("No calendars yet.", style = Type.body, color = Palette.secondary) }
                    } else CardSection("Calendars") {
                        for (feed in feeds) row {
                            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                    Text(feed.name, style = Type.body, color = Palette.text)
                                    Text(feed.url, style = Type.footnote, color = Palette.secondary, maxLines = 1)
                                }
                                SecondaryButton("Remove") { run {
                                    store.removeCalendarFeed(feed.id)
                                    feeds = store.calendarFeeds()
                                    events = store.calendarAgenda()
                                } }
                            }
                        }
                    }
                    if (events.isEmpty()) CardSection {
                        row { Text("No events in the next two weeks.", style = Type.body, color = Palette.secondary) }
                    } else CardSection("Next two weeks") {
                        for (event in events) row {
                            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(event.title, style = Type.body, color = Palette.text)
                                Text(eventTime(event), style = Type.footnote, color = Palette.secondary)
                                event.location?.takeIf { it.isNotBlank() }?.let {
                                    Text(it, style = Type.footnote, color = Palette.secondary)
                                }
                            }
                        }
                    }
                }
                WorkspaceTab.Email -> {
                    var mailName by remember { mutableStateOf("") }
                    var mailAddress by remember { mutableStateOf("") }
                    var mailPassword by remember { mutableStateOf("") }
                    var mailImap by remember { mutableStateOf("") }
                    var mailSmtp by remember { mutableStateOf("") }
                    var mailUser by remember { mutableStateOf("") }
                    var mailTo by remember { mutableStateOf("") }
                    var mailSubject by remember { mutableStateOf("") }
                    var mailBody by remember { mutableStateOf("") }
                    if (emails.isEmpty()) CardSection {
                        row { Text("No email account yet. Add one below.", style = Type.body, color = Palette.secondary) }
                    } else CardSection("Accounts") {
                        for (account in emails) row {
                            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Column(Modifier.weight(1f).clickable { mailAccount = account.id }, verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                    Text(account.name, style = Type.body, color = Palette.text)
                                    Text(account.address, style = Type.footnote, color = Palette.secondary)
                                }
                                if (mailAccount == account.id) Text("Reading", style = Type.footnote, color = Palette.secondary)
                                SecondaryButton("Remove") { run {
                                    store.deleteEmailAccount(account.id)
                                    emails = store.emailAccounts()
                                    if (mailAccount == account.id) {
                                        mailAccount = emails.firstOrNull()?.id ?: ""
                                        messages = emptyList()
                                    }
                                } }
                            }
                        }
                        row {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                SecondaryButton("Read inbox", enabled = mailAccount.isNotEmpty()) { run {
                                    messages = store.emailInbox(mailAccount)
                                } }
                            }
                        }
                    }
                    if (messages.isNotEmpty()) CardSection("Inbox") {
                        for (message in messages) row {
                            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                Text(message.subject.ifEmpty { "(no subject)" }, style = Type.body, color = Palette.text)
                                Text("${message.from} · ${message.date}", style = Type.footnote, color = Palette.secondary, maxLines = 2)
                            }
                        }
                    }
                    CardSection("Add an account", "IMAP uses TLS on 993; SMTP uses STARTTLS on 587. An app password works where the provider requires one.") {
                        row {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                FieldBox(mailName, { mailName = it }, "Name (optional)")
                                FieldBox(mailAddress, { mailAddress = it }, "me@example.com")
                                FieldBox(mailPassword, { mailPassword = it }, "Password or app password")
                                FieldBox(mailImap, { mailImap = it }, "IMAP server (imap.example.com)")
                                FieldBox(mailSmtp, { mailSmtp = it }, "SMTP server (smtp.example.com)")
                                FieldBox(mailUser, { mailUser = it }, "Username (defaults to the address)")
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                    SecondaryButton("Save account",
                                        enabled = mailAddress.isNotBlank() && mailImap.isNotBlank() && mailSmtp.isNotBlank() && mailPassword.isNotEmpty()) { run {
                                        val saved = store.saveEmailAccount(
                                            mailName.ifBlank { mailAddress },
                                            mailAddress.trim(),
                                            mailPassword,
                                            mailImap.trim(),
                                            mailSmtp.trim(),
                                            mailUser.trim().ifBlank { mailAddress.trim() },
                                        )
                                        emails = store.emailAccounts()
                                        mailAccount = saved.id
                                        mailName = ""; mailAddress = ""; mailPassword = ""; mailImap = ""; mailSmtp = ""; mailUser = ""
                                    } }
                                }
                            }
                        }
                    }
                    if (emails.isNotEmpty()) CardSection("Send an email", "It is sent at once from the selected account.") {
                        row {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                FieldBox(mailTo, { mailTo = it }, "To")
                                FieldBox(mailSubject, { mailSubject = it }, "Subject")
                                FieldBox(mailBody, { mailBody = it }, "Message", minLines = 3, maxLines = 8)
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                    SecondaryButton("Send", enabled = mailTo.isNotBlank() && (mailBody.isNotBlank() || mailSubject.isNotBlank())) { run {
                                        store.sendEmail(mailAccount, mailTo.trim(), mailSubject.trim(), mailBody)
                                        mailTo = ""; mailSubject = ""; mailBody = ""
                                    } }
                                }
                            }
                        }
                    }
                }
            }
            if (error != null) Text(error ?: "", style = Type.footnote, color = Palette.danger)
        }
    }
}

@Composable private fun BoardTaskRow(task: BoardTask, onMove: (String) -> Unit, onDelete: () -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Symbol(if (task.isDone) "checkmark.circle.fill" else "circle", size = 18.dp,
            tint = if (task.isDone) Palette.accent else Palette.tertiary)
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(task.title, style = Type.body, color = Palette.text)
            if (task.detail.isNotEmpty()) Text(task.detail, style = Type.footnote, color = Palette.secondary, maxLines = 2)
        }
        DropdownMenu({ BoardTask.statuses.filter { it != task.status }.map { status ->
            MenuItem("Move to ${statusTitle(status)}", icon = "arrow.right") { onMove(status) }
        } + MenuItem("Delete", icon = "trash", destructive = true, divider = true, action = onDelete) }) {
            Symbol("ellipsis", size = 16.dp, tint = Palette.secondary)
        }
    }
}

private enum class WorkspaceTab(val label: String) {
    Board("Board"),
    Teams("Teams"),
    Links("Links"),
    Calendar("Calendar"),
    Email("Email"),
}

private fun statusTitle(status: String): String = when (status) {
    "doing" -> "Doing"
    "waiting" -> "Waiting"
    "done" -> "Done"
    else -> "To do"
}

private fun eventTime(event: CalendarEvent): String {
    val date = Date(event.start)
    val day = SimpleDateFormat("d MMM", Locale.getDefault()).format(date)
    return if (event.allDay) day else "$day, " + SimpleDateFormat("HH:mm", Locale.getDefault()).format(date)
}

private fun waitingDue(item: WaitingItem): String =
    item.dueAt?.let { SimpleDateFormat("d MMM", Locale.getDefault()).format(Date(it)) } ?: ""

