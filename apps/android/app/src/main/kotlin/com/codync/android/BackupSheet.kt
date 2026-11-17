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
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/** An encrypted copy of the computer's database in the person's own WebDAV folder (iOS `BackupSettingsView`). */
@Composable
internal fun BackupSheet(store: AppStore) {
    val scope = rememberCoroutineScope()
    var status by remember { mutableStateOf<CloudBackupStatus?>(null) }
    var url by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var backend by remember { mutableStateOf("webdav") }
    var rclone by remember { mutableStateOf(false) }
    var password by remember { mutableStateOf("") }
    var keep by remember { mutableStateOf("14") }
    var enabled by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }

    fun load() {
        scope.launch {
            try {
                val next = store.cloudBackupStatus()
                status = next
                url = next.url
                username = next.username
                backend = next.backend
                rclone = next.rclone
                keep = next.keep.toString()
                enabled = next.enabled
                error = null
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                error = failure.message
            }
        }
    }

    LaunchedEffect(Unit) { load() }

    fun save(runNow: Boolean) {
        val count = keep.toIntOrNull() ?: 14
        if (count < 1 || count > 100) {
            error = "Keep between 1 and 100 backups."
            return
        }
        busy = true
        notice = null
        scope.launch {
            try {
                status = store.saveCloudBackup(url.trim(), username.trim(), password, backend, enabled, count)
                password = ""
                notice = if (runNow) {
                    val upload = store.runCloudBackup()
                    "Uploaded ${upload.name} (${maxOf(1, upload.bytes / 1024)} KB)."
                } else {
                    "Saved."
                }
                load()
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                error = failure.message
            } finally {
                busy = false
            }
        }
    }

    Column(Modifier.fillMaxWidth()) {
        ModalHeader("Backups")
        CardForm {
            CardSection(
                "Cloud backup",
                "Codync seals the database with this computer's vault key before uploading. The server only sees ciphertext.",
            ) {
                row {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        SegmentedChoice(backend, listOf("webdav" to "WebDAV", "rclone" to "rclone"), { backend = it })
                        Text(
                            if (backend == "rclone") {
                                if (rclone) "A remote you configured in rclone on this computer, like gdrive:Codync."
                                else "rclone is not on this computer's PATH. Install it and configure a remote first."
                            } else {
                                "A WebDAV folder, like Nextcloud, ownCloud, Box or pCloud."
                            },
                            style = Type.footnote,
                            color = Palette.secondary,
                        )
                        FieldBox(url, { url = it }, if (backend == "rclone") "rclone path (gdrive:Codync)" else "WebDAV folder (https://…)")
                        if (backend == "webdav") {
                            FieldBox(username, { username = it }, "Username")
                            PlainInput(password, { password = it }, if (status?.hasPassword == true) "Password saved; leave empty to keep it" else "Password", secret = true)
                        }
                        FieldBox(keep, { keep = it }, "Keep (days)")
                        CodyncSwitch(enabled, { enabled = it }) { Text("Back up every day", style = Type.body, color = Palette.text) }
                        Text("A backup is uploaded only while the computer's host runs. The file is sealed before it leaves.", style = Type.footnote, color = Palette.secondary)
                    }
                }
            }
            CardSection {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        SecondaryButton("Save", enabled = !busy && url.isNotBlank()) { save(false) }
                        Box(Modifier.weight(1f))
                        SecondaryButton("Back up now", enabled = !busy && status?.configured == true) { save(true) }
                    }
                }
            }
            status?.let { current ->
                CardSection("Last backup") {
                    row {
                        Text(current.lastAt?.let(::backupWhen) ?: "No backup yet.", style = Type.body, color = Palette.text)
                    }
                    current.lastError?.let { failure -> row { Text(failure, style = Type.footnote, color = Palette.danger) } }
                    for (file in current.files.reversed()) row {
                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            Text(file.name, style = Type.footnote, color = Palette.secondary, maxLines = 1, modifier = Modifier.weight(1f))
                            Text(backupWhen(file.at), style = Type.footnote, color = Palette.tertiary)
                        }
                    }
                }
            }
            notice?.let { row { Text(it, style = Type.footnote, color = Palette.secondary) } }
            error?.let { row { Text(it, style = Type.footnote, color = Palette.danger) } }
        }
    }
}

private fun backupWhen(ms: Long): String =
    SimpleDateFormat("d MMM, HH:mm", Locale.getDefault()).format(Date(ms))
