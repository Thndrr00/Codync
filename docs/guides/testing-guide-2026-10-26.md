# Guía de pruebas: lo implementado (2026-10-26)

Rama: `integrate/windows-all-additions` (fork `Thndrr00/Codync`). Versión de los builds: 2.26.0.
Estado de cada prueba: ✅ ya probado en dispositivo real · 🧪 cubierto solo por pruebas automáticas · ⏳ pendiente.

## 0. Preparación

| Qué | Dónde |
|---|---|
| App de Windows (producción) | `%LOCALAPPDATA%\Programs\codync-desktop\Codync.exe` |
| Host de producción | puerto 19222, token en `~\.codync\token` |
| APK de Android firmado | `Documentos\Codync-2.26.0-android.apk` (también en `Download/Codync-2.26.0-prod.apk` del teléfono) |
| Clave de firma | `~\.codync-signing\codync-release.jks` (hacer copia fuera de la PC) |

1. Windows: abre Codync y comprueba `http://127.0.0.1:19222/health` → `version 2.26.0`, `environment main`.
2. Android: desbloquea el teléfono y toca **Instalar** en el aviso de MIUI. La versión del teléfono hoy es la anterior a la última corrección (ver 1.4).
3. Empareja el teléfono con el enlace `codync://pair?...` que devuelve `POST /api/pairing` del host.

## 1. Miniapps remotas en el teléfono

Qué es: el teléfono ya no necesita la dirección local de la PC; el host atiende cada petición por el canal cifrado (`appRequest`).

| # | Paso | Resultado esperado | Estado |
|---|---|---|---|
| 1.1 | Publica una miniapp (`chatCall save_app` con `slug`, `title`, `entry`) | La tarjeta «Phone check» aparece en el chat con **Open App** | ✅ |
| 1.2 | Toca **Open App** en el teléfono | Se ve la página HTML de la miniapp | ✅ |
| 1.3 | Toca el botón que llama `codync.put` | El dato queda en el host y la lista lo muestra al reabrir | ✅ |
| 1.4 | Con la miniapp abierta, añade datos desde otro cliente | La lista se actualiza sola (sondeo de colecciones). Corregido que una colección nueva no refrescara; **requiere instalar el APK nuevo** | ⏳ |
| 1.5 | `appRequest` con token inválido | HTTP 403 | ✅ |
| 1.6 | `appRequest` con ruta `../x` | HTTP 400 `invalid path` | ✅ |
| 1.7 | Cierra y abre la miniapp | Datos persistentes | ✅ |

Prueba manual de bajo nivel (host local): `POST /api/appRequest` con `{botId, slug, token, method, path}`.

## 2. Check-ins proactivos

Qué es: el host despierta a cada bot con check-ins activos tras su intervalo, fuera de horas de silencio y dentro del límite diario.

| # | Paso | Resultado esperado | Estado |
|---|---|---|---|
| 2.1 | Android → bot → Edit profile → **Proactive check-ins** | Interruptor y 4 ajustes (cada N horas, silencio desde/hasta, máximo por día) | ✅ |
| 2.2 | Activar y guardar (✓) | El host guarda `proactive.enabled = true` | ✅ |
| 2.3 | Esperar un ciclo (1 min) | Aparece el registro `proactive:<botId>` en la tabla `kv` (última ejecución, día, contador) | ✅ |
| 2.4 | Esperar el intervalo configurado (por defecto 4 h) | El bot recibe un turno de check-in y solo te escribe si tiene algo que decir | ⏳ |
| 2.5 | Escritorio → editor del bot → mismos controles | Igual que en Android | 🧪 |
| 2.6 | Probar con intervalo 1 h y silencio que cubra la hora actual | No se dispara en horas de silencio | 🧪 (módulo `proactive`, 3 pruebas) |

## 3. Decisiones pendientes

Qué es: la tarjeta «Pending Decisions» ahora lee lo que el host realmente escribe.

| # | Paso | Resultado esperado | Estado |
|---|---|---|---|
| 3.1 | Un bot pide permiso (modo «Ask me») | Aparece en Pending Decisions con **Allow** y **Deny** | 🧪 |
| 3.2 | Toca **Allow** / **Deny** | El permiso se resuelve; la entrada desaparece | 🧪 |
| 3.3 | Un bot pide «human takeover» (`request_handoff`) | Aparece «Human takeover requested» con el motivo | 🧪 |
| 3.4 | Un error de agente sin respuesta posterior | Aparece «Agent error»; desaparece si el bot o tú escribís de nuevo | 🧪 |

Pendiente real: provocar 3.1 en el teléfono con un bot en modo «Ask me».

## 4. Windows de producción

| # | Paso | Estado |
|---|---|---|
| 4.1 | Instalar `codync-windows-x64.exe` (`/S /currentuser`) y abrir | ✅ |
| 4.2 | `/health` responde 2.26.0 `main` | ✅ |
| 4.3 | Editor del bot muestra «Proactive check-ins» | 🧪 |
| 4.4 | Panel de decisiones muestra Deny/Approve según las opciones del host | 🧪 |

## 5. Hooks de Codex (revisión)

| # | Paso | Estado |
|---|---|---|
| 5.1 | `~/.codex/hooks.json` tiene `PostToolUse` (Checking UI changes + aviso de Orca) | ✅ |
| 5.2 | Ejecutar ambos hooks a mano: código 0, ≈6 s y <1 s | ✅ |
| 5.3 | Abrir sesión nueva en Codex y editar un archivo: sin «El hook … terminó: failed» | ⏳ |

Si falla de nuevo, sube el `timeout` del hook de impeccable de 15 a 30 s en `hooks.json`.

## 6. Pruebas automáticas

| Capa | Comando | Resultado |
|---|---|---|
| Host | `cargo test -j 1 proactive -- --test-threads=1` | 4 pasan |
| Escritorio | `npm run typecheck` y `npm test` | 334 pasan, 0 fallan, 19 omitidas |
| Android | `./gradlew.bat assembleRelease testDebugUnitTest :core:test lintDebug` | Pasa |

No corrido: suite completa del host ni clippy (poca memoria). Pruebas instrumentadas de Android: no corridas en este teléfono.

## 7. Pendiente por hacer o decidir

- Reglas de acciones (`actionRules`): guardadas pero sin aplicar en el host.
- `appsInBackground` de los check-ins: guardado, aún sin consultar.
- iOS y terminal: sin miniapps remotas, check-ins ni decisiones pendientes.
- Widget «todo en uno» de Android (plan acordado, sin implementar).
- Fusión con el repositorio original v2.29.0 (rama nueva `integrate/upstream-2.29`, por etapas).
- La versión sigue en 2.26.0 y nada se ha subido de versión.

## 8. Problemas conocidos del equipo de pruebas

- La PC tiene 8 GB de RAM: compila el host con `CARGO_BUILD_JOBS=1` y Gradle con `--no-daemon -Dorg.gradle.jvmargs=-Xmx2g`.
- electron-builder necesita `dist/icon.ico` previo y `-c.win.icon=dist/icon.ico`.
- Instalar por USB en el Xiaomi exige aceptar el aviso de MIUI (`INSTALL_FAILED_USER_RESTRICTED` si no).
- Firma de Android: usar siempre la clave de `~\.codync-signing`; la de depuración antigua ya no coincide con el teléfono.
