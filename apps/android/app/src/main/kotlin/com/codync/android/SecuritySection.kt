package com.codync.android

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import com.codync.android.design.CardSection

/** Device-scoped lock for revealing or replacing secrets; the computer's own permissions are unaffected. */
@Composable internal fun SecuritySection() {
    val context = LocalContext.current
    var required by remember { mutableStateOf(SensitiveAccess.required(context)) }
    val available = remember { SensitiveAccess.available(context) }
    CardSection(
        title = "Security",
        footer = when {
            !available && required ->
                "This phone has no biometric or screen lock, so protected actions stay locked. Set one in Android settings, or turn this off."
            !available -> "Set a screen lock or enroll a fingerprint or face in Android settings to protect webhook keys and saved credentials on this phone."
            else -> "Ask for your fingerprint, face or screen lock before showing or replacing webhook keys and saved credentials. Only this phone is locked; the computer decides what is allowed."
        },
    ) {
        row {
            // Still switchable when the lock disappeared: turning it off is the way out of a phone that lost its lock.
            SwitchRow("Require unlock for secrets", isOn = required, enabled = available || required,
                onChange = { required = it; SensitiveAccess.require(context, it) })
        }
    }
}
