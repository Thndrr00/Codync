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
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

/** Someone who uses this computer with their own phone or bots (docs/features/people.md). */
internal data class PersonRow(val id: String, val name: String, val email: String)

/** A paired device as the computer lists it for its owner. */
private data class DeviceRow(val key: String, val name: String, val personId: String, val person: String)

internal suspend fun AppStore.people(): List<PersonRow> {
    val list = hostQuery("people", buildJsonObject { }).jsonObject["people"]?.jsonArray ?: return emptyList()
    return list.mapNotNull { item ->
        val row = item.jsonObject
        val id = row["id"]?.jsonPrimitive?.content ?: return@mapNotNull null
        PersonRow(id, row["name"]?.jsonPrimitive?.content.orEmpty(), row["email"]?.jsonPrimitive?.content.orEmpty())
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
    var devices by remember { mutableStateOf<List<DeviceRow>>(emptyList()) }
    var name by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    fun load() = scope.launch {
        runCatching {
            people = store.people()
            devices = store.authorizedDevices()
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
                        SecondaryButton("Remove", enabled = !busy) {
                            scope.launch {
                                try {
                                    store.hostQuery("removePerson", buildJsonObject { put("id", person.id) })
                                    load()
                                } catch (e: Exception) {
                                    error = e.message
                                }
                            }
                        }
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
