package com.codync.android

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.lifecycle.viewmodel.compose.viewModel
import org.junit.Rule
import org.junit.Test

class PairingViewTest {
    @get:Rule val compose = createComposeRule()

    @Test fun windowsOffersTheInstallerAndExplainsHowToScanItsCode() {
        compose.setContent { Hosted { PairingView(AppState(loading = false), viewModel()) } }
        compose.onNodeWithText("Windows").performClick()
        compose.onNodeWithText("https://github.com/Thndrr00/Codync/releases/latest/download/codync-windows-x64.exe").assertIsDisplayed()
        compose.onNodeWithText("Continue").performClick()
        compose.onNodeWithText("On Windows, right-click Codync in the system tray", substring = true).assertIsDisplayed()
        compose.onNodeWithText("Use Codync on both devices", substring = true).assertIsDisplayed()
        compose.onNode(hasSetTextAction() and hasText("Or paste a", substring = true)).performScrollTo().assertIsDisplayed()
    }
}
