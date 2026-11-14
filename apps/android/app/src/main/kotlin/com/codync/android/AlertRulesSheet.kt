package com.codync.android

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.codync.android.design.CardForm
import com.codync.android.design.ModalHeader

/** The computer's alert rules on their own, from its card on the Computers page (iOS `AlertRulesSheet`). */
@Composable
internal fun AlertRulesSheet(store: AppStore) {
    Column(Modifier.fillMaxWidth()) {
        ModalHeader("Alert rules")
        CardForm {
            AlertRulesSection(store)
        }
    }
}
