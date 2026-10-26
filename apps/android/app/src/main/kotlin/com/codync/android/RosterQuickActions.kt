package com.codync.android

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.snap
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import com.codync.android.design.IconButton
import com.codync.android.design.Motion
import com.codync.android.design.Palette
import com.codync.android.design.SecondaryButton
import com.codync.android.design.Type
import com.codync.android.design.medium
import com.codync.android.design.reduceMotion

private data class RosterQuickAction(val label: String, val invoke: () -> Unit)

/** Navigation only: the roster supplies the same callbacks used by its existing controls. */
@Composable internal fun RosterQuickActions(
    query: String,
    online: Boolean,
    account: () -> Unit,
    manage: () -> Unit,
    newBot: () -> Unit,
    newGroup: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember(query.isBlank()) { mutableStateOf(false) }
    val reduced = reduceMotion()
    val terms = query.trim().split(Regex("\\s+")).filter(String::isNotEmpty)
    val actions = buildList {
        if (online) {
            add(RosterQuickAction("New bot", newBot))
            add(RosterQuickAction("New group", newGroup))
        }
        add(RosterQuickAction("Computers", manage))
        add(RosterQuickAction("Account", account))
    }.filter { action -> terms.all { action.label.contains(it, ignoreCase = true) } }
    if (actions.isEmpty()) return
    val visible = terms.isNotEmpty() || expanded

    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("Actions", Modifier.weight(1f), style = Type.caption.medium, color = Palette.secondary)
            if (terms.isEmpty()) {
                IconButton(
                    if (expanded) "Hide actions" else "Show actions",
                    if (expanded) "chevron.up" else "chevron.down",
                    modifier = Modifier.semantics { stateDescription = if (expanded) "Expanded" else "Collapsed" },
                ) { expanded = !expanded }
            }
        }
        AnimatedVisibility(
            visible,
            enter = fadeIn(if (reduced) snap() else Motion.fade()) +
                expandVertically(animationSpec = if (reduced) snap() else Motion.layoutSize),
            exit = fadeOut(if (reduced) snap() else Motion.fade()) +
                shrinkVertically(animationSpec = if (reduced) snap() else Motion.layoutSize),
        ) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                actions.forEach { action -> SecondaryButton(action.label, action = action.invoke) }
            }
        }
    }
}
