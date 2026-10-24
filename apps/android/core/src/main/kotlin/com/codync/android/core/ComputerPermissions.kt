package com.codync.android.core

import kotlinx.serialization.Serializable

@Serializable data class ComputerPermissions(val browser: Boolean = false, val files: Boolean = false, val shell: Boolean = false)
