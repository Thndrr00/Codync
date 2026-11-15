package com.codync.android

import com.codync.android.core.HostModule
import com.codync.android.core.ModuleCalls
import com.codync.android.core.ModuleExport
import com.codync.android.core.ModulePermissions
import com.codync.android.core.ModuleScreen
import com.codync.android.core.ModuleViewReply
import com.codync.android.core.ModulesReply
import com.codync.android.core.WireJson
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.decodeFromJsonElement

// Modules on the computer: install from a file, approve what one asks for, turn it on or off, read its screen.

suspend fun AppStore.modules(): List<HostModule> =
    WireJson.decodeFromJsonElement<ModulesReply>(hostCall("modules", ModuleCalls.none())).modules

suspend fun AppStore.installModule(bundle: JsonElement): List<HostModule> =
    WireJson.decodeFromJsonElement<ModulesReply>(hostCall("installModule", ModuleCalls.install(bundle = bundle), 60_000)).modules

suspend fun AppStore.setModule(id: String, enabled: Boolean): List<HostModule> =
    WireJson.decodeFromJsonElement<ModulesReply>(hostCall("setModule", ModuleCalls.set(id, enabled), 30_000)).modules

suspend fun AppStore.approveModule(id: String, permissions: ModulePermissions? = null): List<HostModule> =
    WireJson.decodeFromJsonElement<ModulesReply>(hostCall("approveModule", ModuleCalls.approve(id, permissions), 30_000)).modules

suspend fun AppStore.uninstallModule(id: String): List<HostModule> =
    WireJson.decodeFromJsonElement<ModulesReply>(hostCall("uninstallModule", ModuleCalls.id(id), 30_000)).modules

suspend fun AppStore.exportModule(id: String): ModuleExport =
    WireJson.decodeFromJsonElement(hostCall("exportModule", ModuleCalls.id(id), 30_000))

suspend fun AppStore.moduleView(id: String, view: String): ModuleScreen =
    WireJson.decodeFromJsonElement<ModuleViewReply>(hostCall("moduleView", ModuleCalls.view(id, view), 120_000)).view
