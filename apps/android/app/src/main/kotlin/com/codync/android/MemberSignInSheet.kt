package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.design.*

/**
 * Signing in as a member of a computer (docs/features/people.md): the owner or an admin invites the
 * email, and the invitation is a QR code. Scanning it pairs this phone as that person, with only
 * their own bots, chats and connectors. There is no password: the code is single-use and the session
 * it opens can be ended from the computer's People screen (iOS `MemberSignInView`).
 */
@Composable
internal fun MemberSignInSheet(onScan: () -> Unit) {
    Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState())) {
        ModalHeader(tr("Sign in as a member"))
        Column(Modifier.padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Text(
                tr("A member of a computer uses it with their own bots, chats and connectors. The owner invites your email in Settings, People, and shows you the invitation."),
                style = Type.callout, color = Palette.secondary,
            )
            Text(
                tr("Scan the invitation code from that computer. It works once, and this phone then acts as you."),
                style = Type.callout, color = Palette.secondary,
            )
            PrimaryButton(tr("Scan the invitation")) { onScan() }
        }
    }
}
