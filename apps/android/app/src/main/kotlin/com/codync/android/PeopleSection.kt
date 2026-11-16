package com.codync.android

import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

/** Someone who uses this computer with their own phone or bots (docs/features/people.md). */
internal data class PersonRow(val id: String, val name: String, val email: String, val role: String)

/** A paired device as the computer lists it for its owner. */
private data class DeviceRow(val key: String, val name: String, val personId: String, val person: String)

/** One member session, paired device or live invitation (`memberSessions`). */
private data class MemberSessionRow(
    val id: String,
    val kind: String,
    val person: String,
    val email: String,
    val name: String,
    val platform: String,
    val expiresAt: Long?,
    val connected: Boolean,
)

internal suspend fun AppStore.people(): List<PersonRow> {
    val list = hostQuery("people", buildJsonObject { }).jsonObject["people"]?.jsonArray ?: return emptyList()
    return list.mapNotNull { item ->
        val row = item.jsonObject
        val id = row["id"]?.jsonPrimitive?.content ?: return@mapNotNull null
        PersonRow(id, row["name"]?.jsonPrimitive?.content.orEmpty(), row["email"]?.jsonPrimitive?.content.orEmpty(),
            row["role"]?.jsonPrimitive?.content.orEmpty().ifBlank { "member" })
    }
}

/** Whether this caller is the owner: an admin's phone sees the people but can't change a role. */
internal suspend fun AppStore.canManageRoles(): Boolean = runCatching {
    hostQuery("people", buildJsonObject { }).jsonObject["canManageRoles"]?.jsonPrimitive?.content?.toBoolean() ?: true
}.getOrDefault(true)

/** Every member session, paired member device and live invitation; empty on a host without them. */
private suspend fun AppStore.memberSessions(): List<MemberSessionRow> {
    val list = runCatching { hostQuery("memberSessions", buildJsonObject { }) }.getOrNull()
        ?.jsonObject?.get("sessions")?.jsonArray ?: return emptyList()
    return list.mapNotNull { item ->
        val row = item.jsonObject
        val id = row["id"]?.jsonPrimitive?.content ?: return@mapNotNull null
        MemberSessionRow(
            id = id,
            kind = row["kind"]?.jsonPrimitive?.content.orEmpty(),
            person = row["person"]?.jsonPrimitive?.content.orEmpty(),
            email = row["email"]?.takeUnless { it is JsonNull }?.jsonPrimitive?.content.orEmpty(),
            name = row["name"]?.takeUnless { it is JsonNull }?.jsonPrimitive?.content.orEmpty(),
            platform = row["platform"]?.takeUnless { it is JsonNull }?.jsonPrimitive?.content.orEmpty(),
            expiresAt = row["expiresAt"]?.takeUnless { it is JsonNull }?.jsonPrimitive?.content?.toLongOrNull(),
            connected = row["connected"]?.jsonPrimitive?.content?.toBoolean() ?: false,
        )
    }
}

private suspend fun AppStore.authorizedDevices(): List<DeviceRow> {
    val list = hostQuery("devices", buildJsonObject { }).jsonObject["devices"]?.jsonArray ?: return emptyList()
    return list.mapNotNull { item ->
        val row = item.jsonObject
        val key = row["key"]?.jsonPrimitive?.content ?: return@mapNotNull null
        DeviceRow(
            key = key,
            name = row["name"]?.jsonPrimitive?.content.orEmpty(),
            personId = row["personId"]?.jsonPrimitive?.content.orEmpty(),
            person = row["person"]?.jsonPrimitive?.content.orEmpty(),
        )
    }
}

/**
 * Who is on this computer besides its owner: add or remove a person, and assign a paired phone to one.
 * The owner does this on the computer or from their own phone; a member's phone is refused by the host, so the
 * section then shows that error. There is no member password: a person is a label the owner puts on a device or a bot.
 */
@Composable
internal fun PeopleSection(state: AppState, store: AppStore) {
    val scope = rememberCoroutineScope()
    var people by remember { mutableStateOf<List<PersonRow>>(emptyList()) }
    var sessions by remember { mutableStateOf<List<MemberSessionRow>>(emptyList()) }
    var devices by remember { mutableStateOf<List<DeviceRow>>(emptyList()) }
    var name by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var inviteName by remember { mutableStateOf("") }
    var inviteEmail by remember { mutableStateOf("") }
    var inviteRole by remember { mutableStateOf("member") }
    var issued by remember { mutableStateOf<Pair<String, String>?>(null) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var removing by remember { mutableStateOf<PersonRow?>(null) }
    var canManageRoles by remember { mutableStateOf(true) }
    fun load() = scope.launch {
        runCatching {
            people = store.people()
            devices = store.authorizedDevices()
            sessions = store.memberSessions()
            canManageRoles = store.canManageRoles()
            error = null
        }.onFailure { error = it.message }
    }
    LaunchedEffect(state.contextId) { load() }

    CardSection(
        "People on this computer",
        "Someone who uses this computer with their own phone or their own bots. Their phone sees only their bots, chats and connectors, plus the ones shared with everyone.",
    ) {
        row {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (people.isEmpty()) {
                    Text("No one else yet", style = Type.caption, color = Palette.secondary)
                }
                for (person in people) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Column(Modifier.weight(1f)) {
                            Text(person.name, style = Type.body, color = Palette.text)
                            if (person.email.isNotBlank()) Text(person.email, style = Type.caption, color = Palette.secondary)
                        }
                        if (canManageRoles) {
                            ChoicePicker(person.role, listOf("member" to tr("Member"), "admin" to tr("Admin")), { role ->
                                scope.launch {
                                    try {
                                        store.hostQuery("updatePerson", buildJsonObject {
                                            put("id", person.id); put("name", person.name); put("email", person.email)
                                            put("role", role)
                                        })
                                        load()
                                    } catch (e: Exception) {
                                        error = e.message
                                    }
                                }
                            }, label = tr("Role"))
                        } else {
                            Text(tr(if (person.role == "admin") "Admin" else "Member"), style = Type.caption, color = Palette.secondary)
                        }
                        SecondaryButton("Remove", enabled = !busy) { removing = person }
                    }
                }
                FieldBox(name, { name = it }, "Name", 1, 1, fill = Palette.bubbleUser)
                FieldBox(email, { email = it }, "Email (optional)", 1, 1, fill = Palette.bubbleUser, capitalize = false)
                SecondaryButton(if (busy) "Adding…" else "Add a person", enabled = !busy && name.isNotBlank()) {
                    busy = true
                    scope.launch {
                        try {
                            store.hostQuery("addPerson", buildJsonObject {
                                put("name", name.trim()); put("email", email.trim())
                            })
                            name = ""
                            email = ""
                            load()
                        } catch (e: Exception) {
                            error = e.message
                        } finally {
                            busy = false
                        }
                    }
                }
                error?.let { Text(it, style = Type.caption, color = Palette.danger) }
            }
        }
    }

    CardSection("Invite a member",
        "An invitation is a one-time code the member scans on their phone, or signs in with at the computer. They act as that person, with only their own bots, chats and connectors.") {
        row {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                FieldBox(inviteName, { inviteName = it }, "Name (optional)", 1, 1, fill = Palette.bubbleUser)
                FieldBox(inviteEmail, { inviteEmail = it }, "Email", 1, 1, fill = Palette.bubbleUser, capitalize = false)
                if (canManageRoles) {
                    ChoicePicker(inviteRole, listOf("member" to tr("Member"), "admin" to tr("Admin")),
                        { inviteRole = it }, label = tr("Role"))
                }
                SecondaryButton(if (busy) "Inviting…" else "Invite", enabled = !busy && inviteEmail.isNotBlank()) {
                    busy = true
                    scope.launch {
                        try {
                            val answer = store.hostQuery("inviteMember", buildJsonObject {
                                put("name", inviteName.trim()); put("email", inviteEmail.trim()); put("role", inviteRole)
                            })
                            val person = answer.jsonObject["person"]?.jsonObject
                            val invite = answer.jsonObject["invite"]?.jsonObject
                            issued = (person?.get("name")?.jsonPrimitive?.content.orEmpty() to
                                (invite?.get("pairingUrl")?.jsonPrimitive?.content
                                    ?: invite?.get("code")?.jsonPrimitive?.content.orEmpty()))
                            inviteName = ""
                            inviteEmail = ""
                            load()
                        } catch (e: Exception) {
                            error = e.message
                        } finally {
                            busy = false
                        }
                    }
                }
                issued?.let { (person, link) ->
                    Text(tr("Invitation for {name}").replace("{name}", person), style = Type.caption, color = Palette.secondary)
                    Text(link, style = Type.caption, color = Palette.text)
                }
                error?.let { Text(it, style = Type.caption, color = Palette.danger) }
            }
        }
    }

    CardSection("Member sessions",
        "A session is a paired phone or a signed-in app. Ending one revokes that device at once; the person and their bots stay.") {
        row {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (sessions.isEmpty()) Text("None yet", style = Type.caption, color = Palette.secondary)
                for (session in sessions) {
                    ValueRow(session.person, detail = when {
                        session.kind == "invite" -> tr("Invitation")
                        session.platform.isNotBlank() -> "${session.name} · ${session.platform}"
                        else -> session.name
                    }) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(
                                when {
                                    session.kind == "invite" -> tr("Pending")
                                    session.connected -> tr("Connected")
                                    else -> tr("Signed out")
                                },
                                style = Type.caption, color = Palette.secondary,
                            )
                            SecondaryButton(tr("End session"), enabled = !busy) {
                                scope.launch {
                                    try {
                                        store.hostQuery("revokeMemberSession", buildJsonObject { put("session", session.id) })
                                        load()
                                    } catch (e: Exception) {
                                        error = e.message
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    CardSection("Assign a device", "A member's phone only sees and changes what is theirs.") {
        row {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (devices.isEmpty()) {
                    Text("None yet", style = Type.caption, color = Palette.secondary)
                }
                for (device in devices) {
                    ValueRow(device.name, detail = device.person.ifBlank { "This computer's owner" }) {
                        DropdownMenu({
                            buildList {
                                add(MenuItem("This computer's owner", selected = device.personId.isBlank()) {
                                    assignPerson(store, scope, device, "", { load() }, { error = it })
                                })
                                for (person in people) {
                                    add(MenuItem(person.name, selected = person.id == device.personId) {
                                        assignPerson(store, scope, device, person.id, { load() }, { error = it })
                                    })
                                }
                            }
                        }, label = "Person") {
                            Row(Modifier.outline().pill(androidx.compose.ui.graphics.Color.Transparent),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                Text(device.person.ifBlank { "This computer's owner" }, style = Type.body, color = Palette.text)
                                Symbol("chevron.down", size = 13.dp, tint = Palette.secondary)
                            }
                        }
                    }
                }
                error?.let { Text(it, style = Type.caption, color = Palette.danger) }
            }
        }
    }

    CodyncDialog(
        tr("Remove {name}?").replace("{name}", removing?.name.orEmpty()),
        removing != null,
        { removing = null },
        message = "Their phone loses access at once. Their bots, chats and connectors become yours. Nothing is deleted.",
        actions = {
            val person = removing
            if (person != null) {
                listOf(DialogAction("Remove", destructive = true) {
                    removing = null
                    scope.launch {
                        try {
                            store.hostQuery("removePerson", buildJsonObject { put("id", person.id) })
                            load()
                        } catch (e: Exception) {
                            error = e.message
                        }
                    }
                })
            } else {
                emptyList()
            }
        },
    )
}

private fun assignPerson(
    store: AppStore,
    scope: kotlinx.coroutines.CoroutineScope,
    device: DeviceRow,
    person: String,
    done: () -> Unit,
    failed: (String?) -> Unit,
) {
    scope.launch {
        try {
            store.hostQuery("setDevicePerson", buildJsonObject {
                put("device", device.key); put("person", person)
            })
            done()
        } catch (e: Exception) {
            failed(e.message)
        }
    }
}

/**
 * The owner picks who a bot belongs to; a member's phone gets no people list, so this stays out of its editor.
 * A bot sees the shared connectors plus its own person's.
 */
@Composable
internal fun PersonPicker(store: AppStore, person: String?, change: (String?) -> Unit) {
    var people by remember { mutableStateOf<List<PersonRow>>(emptyList()) }
    LaunchedEffect(Unit) { people = runCatching { store.people() }.getOrDefault(emptyList()) }
    if (people.isEmpty()) return
    CardSection("This bot belongs to", "A member's phone only sees and changes what is theirs.") {
        row {
            ValueRow("Person", detail = people.firstOrNull { it.id == person }?.name ?: "Anyone on this computer") {
                DropdownMenu({
                    buildList {
                        add(MenuItem("Anyone on this computer", selected = person == null) { change(null) })
                        for (row in people) add(MenuItem(row.name, selected = row.id == person) { change(row.id) })
                    }
                }, label = "Person") {
                    Row(Modifier.outline().pill(androidx.compose.ui.graphics.Color.Transparent),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(people.firstOrNull { it.id == person }?.name ?: "Anyone on this computer",
                            style = Type.body, color = Palette.text, maxLines = 1)
                        Symbol("chevron.down", size = 13.dp, tint = Palette.secondary)
                    }
                }
            }
        }
    }
}
