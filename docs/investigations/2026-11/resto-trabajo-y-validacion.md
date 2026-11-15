# Prompt completo — resto y contenido compartido

## Prompt 1

### Fragmento 1 del prompt 1

promt 1:# Handoff: una sola compilación y prueba en vivo — paridad OpenBot (Android/iOS/desktop/TUI/host)

Fecha: 2026-11-10. Para: el agente que hará la compilación y las pruebas finales de esta rama.
Objetivo: correr **una sola vez** todas las compilaciones y pruebas sobre el mismo HEAD, probar en
vivo en el teléfono y el escritorio, y dejar un reporte con lo que pasó y lo que no se pudo correr.

---

## 0. Dónde está el trabajo

- Worktree: `C:\Users\Edwin Villeda\AppData\Roaming\Deska\worktrees\android-openbot-parity`
- Rama: `feat/android-openbot-parity`
- HEAD de entrega: `5d25ca7d` (41 commits desde que se creó la rama en `09cd6c10`, 2026-11-07)
- Remoto: `origin` = `https://github.com/Thndrr00/Codync.git`; la rama está empujada y en sync
  (`0` commits de diferencia en ambos sentidos).
- Versión: **2.39.0** ya en `apps/project.yml`, `host/Cargo.toml` y `apps/desktop/package.json`.
  No la toques; el bump se hace al fusionar a `main`, no aquí.
- Documentos de referencia:
  - `docs/plans/openbot-mobile-parity.md` — registro vivo de lo hecho y validado.
  - `docs/plans/openbot-android-inventory.md` — inventario original; ojo: es una foto previa y
    todavía dice "M5 is next" y conserva líneas "Missing". No lo tomes como estado actual.

**Muy importante:** el checkout principal `C:\Users\Edwin Villeda\Documents\Codync` está en otra
rama y otros agentes tienen trabajo sin commitear ahí. No entres a ese checkout, no hagas
`stash`/`reset`/`checkout`/`rebase` sobre él, y no reformatees hunk ajenos. Todo este trabajo vive
en el worktree de arriba.

Todo el código está commiteado y empujado. En este worktree solo hay archivos sueltos de scratch en
`tools/` que nunca se commitean.

---

## 1. Qué se trabajó (mapa)

Los hitos son los del inventario del dueño, en su orden. Todos están en
`docs/plans/openbot-mobile-parity.md` con archivo por archivo.

| Hito | Qué se hizo | Commits clave |
| --- | --- | --- |
| M9/M3 | Marcar no leído con línea NEW, leer en voz alta con velocidades, pila de imágenes + visor paginado, SVG, MP4, reconexión con jitter/foreground/timeouts de 15 s, parpadeo sin frames, espejo cifrado AES-GCM borrado al salir | `e1f41424`, `75662f40`, `7bcab1d8`, `44bdea3c`, `65610594` |
| M4 (soporte) | Página **Support** con Discord/GitHub y log de eventos redactado, 5.000 eventos, guardar/limpiar | `65610594`, `44bdea3c`, `2e8be151` |
| M2 (flujos/WhatsApp) | Rutinas desde WhatsApp y salida a WhatsApp, correr en un hilo, pareo por código, cancelar tarea, modelo por enlace del flujo, canales desde el teléfono, WhatsApp/Telegram/Discord/Slack/GitHub en ambas apps | `1f69202b`, `9f4a8ebe`, `50b37817`, `f283a04e`, `72825c7e`, `fbad89d8`, `5253ee59`, `a957af52`, `2bff9b66` |
| M2 (hilos) | Hilos por mensaje y por lista: chip "N respuestas", hoja del hilo, rename/archivar/borrar, no leídos, "Branch from here", en Android/iOS/desktop/TUI | `1b99386d` |
| M5/M6 | Pausa de voz con slider, velocidad de lectura 0,5×–2×, avatar Plush/Flat en widgets, caras en reposo más tranquilas y flotación de grupo | `f9abdf4a` |
| M1/M7 | Coinbase, RevenueCat, TickTick, Proton Pass, Higgsfield; lista Apps con estado y paginación; ventana de uso gastado bajo el modelo | `cb6a93b5` |
| M8 | Panel de cola (Edit/Steer/Delete) compartido con la terminal, pines, dirección por párrafo, nombres en hilos de tarea | `1c0d6242`, `69ed3b18`, `b44563be` |
| X1–X7 | Árabe/RTL real, compartir desde otras apps, bloqueo con PIN/biometría, espejo cifrado, live updates/push, split plegable, cara en notificaciones | `f4834994`, `2ab25260`, `af145aef`, `47d32ec8`, `3b5bb3e3`, `c49a688a` |
| M4 (voz) | **ElevenLabs**: clave en la computadora, voces de la cuenta (`voiceVoices`), Scribe escucha y Flash 2.5 lee, siempre modo Speech, Test dice "OK"; refactor a `host/src/voice/elevenlabs.rs`; correcciones de revisión en iOS/Android/desktop | `da4b4a59`, `e0fa238a`, `eb446f52`, `dd38e168`, `31c474ef`, `5d25ca7d` |
| iOS | Pantallas de canales/WhatsApp, Support, salidas de rutinas, correr en hilo, catálogo árabe, notas App Store 2.39.0 | `b596d872`, `2e8be151`, `f02a7198`, `f1b17cfd`, `fa30f778` |
| Docs/web | Inventario y plan, `docs/features/voice-call.md`, privacidad, SEO, notas Play/App Store 2.39.0 | `3f3df22e`, `fd41e980`, `779fc67f`, `31c474ef`, `5d25ca7d` |

Archivos centrales de lo más reciente (para ubicar rápido):

- Host: `host/src/voice.rs`, `host/src/voice/elevenlabs.rs`, `host/src/api/host.rs` (métodos
  `voiceStatus`, `setVoiceKey`, `voiceSession`, `voiceModels`, `voiceVoices`, `voiceTranscribe`,
  `voiceSpeak`), `host/src/api/devices.rs` (`setVoiceKey` solo loopback).

### Fragmento 2 del prompt 1

- iOS: `apps/ios/Kit/Sources/CodyncKit/Models/Voice.swift`,
  `apps/ios/Kit/Sources/CodyncUI/Call/VoiceChatSettingsView.swift`,
  `.../Call/VoiceEngine.swift`, `.../Call/ThreadSheet.swift`, `.../Call/ThreadsView.swift`,
  tests en `apps/ios/Kit/Tests/CodyncUITests/VoiceSettingsTests.swift`.
- Desktop: `apps/desktop/src/renderer/views/settings/VoiceChatSettingsView.tsx`,
  `.../settings/voice-settings.ts`, `.../call/voice.ts`, `.../call/CloudSpeechEngine.ts`,
  `.../call/CallView.tsx`, `.../thread/dictation.ts` (+ test), `.../thread/RepliesView` y
  `ThreadsCard` de hilos.
- Docs/web: `docs/features/voice-call.md`, `docs/plans/openbot-*.md`, `CLAUDE.md`,
  `web/app/privacy/page.tsx`, `web/app/docs/pages.ts`, `apps/ios/WhatsNew.md`,
  `apps/android/WhatsNew.md`.

---

## 2. Precondiciones (hazlas una vez)

1. `cd` al worktree y muestra `pwd` + `git branch --show-current`.
2. `git fetch origin && git status` — confirma HEAD `5d25ca7d` y que no haya cambios tracked.
   Si `origin/feat/android-openbot-parity` avanzó, haz `git pull --rebase origin
   feat/android-openbot-parity`; si hay conflicto, no descartes hunks ajenos: párate y reporta.
3. **Detén los procesos viejos** antes de compilar/probar (host viejo = protocolo viejo, app
   vieja = UI vieja):
   - Cierra la app de escritorio Codync y sus `codync-host.exe` (incluye los `mcp ...` hijos).

### Fragmento 3 del prompt 1

- El host de desarrollo actual escucha en `19223` con `CODYNC_HOME=%USERPROFILE%\.codync-dev`;
     su token está en `%USERPROFILE%\.codync-dev\token`.

### Fragmento 4 del prompt 1

5. Windows: usa `npm.cmd` y `.\gradlew.bat` (los `.ps1` de npm están bloqueados). Para scripts
   `.ps1`, `powershell -NoProfile -ExecutionPolicy Bypass -File ...`. Varios comandos (cargo, npm)
   escriben a stderr y salen con código 1 aunque hayan funcionado: **lee la salida, no solo el
   exit code**.

---

## 3. La compilación y pruebas (una sola pasada, en este orden)

Corre todo sobre el mismo HEAD y no recompiles por hito. Si algo falla y lo arreglas, vuelve a
correr **solo** el gate afectado; el reporte final debe decir qué HEAD pasó todo.

### 3.1 Host (primero: el escritorio y el teléfono lo usan)

```powershell
cd host
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

- Esperado: `fmt` limpio, `clippy` sin warnings, tests verdes y binario en
  `host\target\debug\codync-host.exe`.
- Baseline: ~1.119 tests. En Windows el test
  `claude_reads_its_own_skills_and_follows_a_symlink` puede fallar por permisos de symlink
  (Developer Mode); si falla solo con error de privilegios (1314), repórtalo como ambiental y
  corre `cargo test -- --skip claude_reads_its_own_skills_and_follows_a_symlink`.
- Atajo opcional: `powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\fasttest.ps1`
  (deja `host/target/test-summary.txt`; mata procesos cargo, así que el host de dev debe estar
  detenido). No es obligatorio.

### Fragmento 5 del prompt 1

### 3.3 Escritorio

```powershell
cd apps\desktop
npm.cmd ci
npm.cmd run typecheck
npm.cmd test
```

- Baseline: 557 tests, **533 pasan, 24 saltados, 0 fallan**; `typecheck` limpio.

### 3.4 iOS (solo en un Mac; en Windows se reporta como no ejecutado)

```bash
cd apps/ios/Kit
xcodebuild test -scheme CodyncKit-Package -destination 'platform=iOS Simulator,name=<un iPhone instalado>'
```

- Si `apps/project.yml` cambió en esta rama: `cd repo && xcodegen generate --spec apps/project.yml`.
- Para la app completa: abrir `apps/Codync.xcodeproj`, esquema `iOS Dev`, Simulator vía Device Hub.

### 3.5 Web/docs (solo si se tocó la web; en esta rama sí: privacidad/SEO)

```powershell
cd web
npm.cmd ci
npm.cmd run lint
npm.cmd run check:docs
```

### 3.6 No toques `cloud/` ni `relay/` — esta rama no los cambia.

---

## 4. Pruebas en vivo

### 4.1 Host de desarrollo + humo de API

```powershell
cd host
$env:CODYNC_HOME = "$env:USERPROFILE\.codync-dev"
Start-Process -FilePath .\target\debug\codync-host.exe -ArgumentList 'serve' -WindowStyle Hidden
Start-Sleep -Seconds 3
$t = Get-Content "$env:USERPROFILE\.codync-dev\token"
$h = @{ Authorization = "Bearer $t" }
function Api($m, $b) { Invoke-RestMethod -Uri "http://127.0.0.1:19223/api/$m" -Method Post -Headers $h -ContentType 'application/json' -Body ($b | ConvertTo-Json -Compress) -TimeoutSec 30 }
```

Comprobaciones (mismo resultado que ya se validó el 2026-11-09):

- `Api voiceStatus @{}` → proveedores `openai`, `gemini`, `elevenlabs`; ElevenLabs con
  `realtime: ""`, `transcribe: "scribe_v1"`, `speech: "eleven_flash_v2_5"`.
- `Api voiceVoices @{provider='elevenlabs'}` sin clave → error
  `"No ElevenLabs key on this computer. Add one in the call settings."`
- `Api voiceSession @{provider='elevenlabs'; model=''; voice='21m00Tcm4TlvDq8ikWAM'}` → error
  `"ElevenLabs doesn't hold live calls. Choose Speech mode."`
- Con una clave válida (pídesela al dueño; no borres claves existentes):
  - `Api setVoiceKey @{provider='elevenlabs'; key='<clave>'}` → guarda solo si ElevenLabs la acepta.
  - `Api voiceVoices @{provider='elevenlabs'}` → `{voices:[{id,name},...]}` de la cuenta.
  - `Api voiceModels @{provider='elevenlabs'}` → modelos TTS + Scribe v1/v2 para transcribir.
  - `Api voiceSpeak @{provider='elevenlabs'; model='eleven_flash_v2_5'; voice='<id>'; text='OK'}` →
    `{audio:"<base64 mp3>", format:"mp3"}`; decodifica y escucha el "OK".
  - Clave inválida: `setVoiceKey` devuelve el error del proveedor y **no** guarda.

### Fragmento 6 del prompt 1

### 4.3 Escritorio

```powershell
cd apps\desktop
npm.cmd run dev
```

- Con el host de desarrollo ya corriendo (3.1/4.1). Si la app no toma el host del worktree,
  revisa que `host\target\debug\codync-host.exe` sea el recién compilado.
- Ajustes → **Voz**: ElevenLabs con su formulario de clave; **Test** suena "OK"; voces de la
  cuenta; sin fila Mode; minutos.
- Hilos: abre un chat → `RepliesView`/`ThreadsCard`; responde, renombra, archiva, borra; chip en
  el chat.
- Si tienes Computer Use, recorre la UI con él; si no, hazlo a mano y reporta cómo.
- Consola: sin errores; al cambiar de muestra, no deben quedar `blob:` URL colgadas.

### 4.4 iPhone (solo Mac)

- Compila `iOS Dev` y abre el Simulator (Device Hub). Ajustes → Voz: ElevenLabs, Test "OK",
  voces de la cuenta, Mode oculto; hilos desde cualquier mensaje y desde la lista.
- Verifica que una versión vieja de la app (si la tienes) no rompe la lista de proveedores: el
  cliente ahora lee el id como texto.

### 4.5 TUI

- `/steer`, `/unqueue`, `/edit` de la cola; `t` sobre un mensaje y `T` para la lista de hilos.
- Sin micrófono: la voz no aplica a la TUI (documentado como no afectada).

---

## 5. Reglas y cosas conocidas

- **No cambies código** salvo que una prueba falle. Si falla: arreglo mínimo en este worktree,
  commit convencional (`fix(...)`), push a `origin feat/android-openbot-parity`, y vuelve a correr
  el gate afectado. Un cambio lógico por commit.
- No toques la versión 2.39.0 ni `CURRENT_PROJECT_VERSION`.
- No commitees `tools/*` (scratch).
- Riesgos conocidos, para reportar y **no** arreglar aquí:
  - `host/src/store/entries.rs` (`last_health_notice` pasa `?3` a una consulta de 2 parámetros):
    bug real, cambio aparte.
  - `MIN_APP` sigue en 2.21.0 por una regresión previa del marketplace; no se cambia en esta rama.
    Un iPhone pre-2.39 con enum estricto de `provider` no se puede arreglar retroactivamente.
  - Gap deliberado: sin modelos neuronales on-device (Supertonic 3/Parakeet v3); ElevenLabs es
    solo voz (Scribe escucha, sus voces leen), nunca conversación en vivo.

### Fragmento 7 del prompt 1

- Si el teléfono no aparece, si no hay Mac o si falta Computer Use: **no lo inventes**; márcalo
  como "no ejecutado" con la razón exacta.

---

## 6. Reporte final (formato)

1. HEAD probado (hash) y rama.
2. Tabla de gates: host (`fmt`, `clippy`, `test`, `build`), Android (Gradle), desktop
   (`typecheck`, `test`), iOS, web — con resultado y notas.
3. Checklist en vivo: teléfono (hilos, voz, fallback, regresión), escritorio, iPhone, TUI —
   pasado / fallado / no ejecutado + por qué.
4. Arreglos hechos (commits y gate re-corrido), o "ninguno".
5. Bloqueos y pendientes externos.

---

## Apéndice: los 41 commits de la rama (`09cd6c10..HEAD`)

```
5d25ca7d docs: name ElevenLabs' model list in voice docs
31c474ef docs: cover ElevenLabs in voice docs and 2.39.0 notes
dd38e168 fix(android,ios,desktop): keep voice settings steady across providers
eb446f52 refactor(host): move ElevenLabs into its own voice module
e0fa238a docs: record the ElevenLabs host API check
da4b4a59 feat(android,ios,desktop,host): add ElevenLabs voice
1b99386d feat(android,ios,desktop,host): add threads to chats
2bff9b66 fix(android,ios): hide the channel tokens as they are typed
c7514578 style(android): line up the Discord form
5253ee59 fix(host): restore the rule tester and always-allow endpoints
fbad89d8 fix(host): let a phone set up the messaging channels
72825c7e feat(android,ios): set channels up from the phone
a957af52 fix(android,ios): localize the channel screens and decode empty Discord
3b5bb3e3 feat(android): prove the account when pairing
c49a688a test(android): repair the stale instrumented suite
779fc67f docs(plans): the iPhone has the channel screens now
b596d872 feat(ios,android): bring channels and WhatsApp to the iPhone
47d32ec8 feat(android): seal the mirror database at rest
f283a04e feat(host,android,desktop): run a flow link on the model it names
fa30f778 docs(ios): record the iPhone Support page and 2.39.0 notes
f02a7198 feat(ios): edit routine outputs and run in a thread
2e8be151 feat(ios,android): add a Support page with a redacted event log
f1b17cfd fix(ios): bundle the Arabic catalog
fd41e980 docs(android): describe the 2.39.0 changes in the Play notes
f4834994 feat(android,ios,desktop): use the app in Arabic
2ab25260 feat(android): send to an agent from the share sheet
af145aef feat(android): lock the app behind a PIN or the phone unlock
b44563be feat(android,desktop,ios): read each paragraph in its own direction
69ed3b18 feat(android,desktop,ios): queue panel above the composer
1c0d6242 feat(host,tui): edit, steer and delete queued messages
cb6a93b5 feat(host,android,ios,desktop): add Proton Pass and spend-aware models
f9abdf4a feat(android,ios,desktop): tune the voice pause and pick a calm avatar
50b37817 feat(host,tui,desktop,ios,android): cancel a stopped or failed task
9f4a8ebe feat(host): link WhatsApp with a pairing code
1f69202b feat(host): start routines from WhatsApp and send to it
44bdea3c feat(android): translate mark unread, read aloud and Support
65610594 feat(android): add a Support page with a redacted event log
3f3df22e docs: track the OpenBot Mobile inventory on Android
7bcab1d8 perf(android): reconnect at once and blink on a timer
75662f40 feat(android): mark unread, read aloud and open media
e1f41424 feat(host): mark messages unread from a message
```

## Prompt 2

### Fragmento 1 del prompt 2

promt 2:
Trabajas en C:\Users\Edwin Villeda\Documents\Codync, rama `feat/openbot-marketplace`,
HEAD 717e6909, 731 commits por delante de origin/main. Varios agentes han trabajado aquí:
NO hagas rebase, reset, checkout, stash ni merge. No descartes ni reformatees hunks ajenos.
Hay cambios sin commitear de otro agente en docs/features/routine-flows.md: no los toques.
Puede existir un .git/MERGE_RR.lock de otro proceso de git: no lo borres a ciegas.

OBJETIVO: UNA sola pasada de compilación y pruebas en vivo de todo lo integrado.
No añadas funciones nuevas salvo para arreglar lo que falle. Deja la rama verde y subida.
Versión 2.39.0: no la cambies.

ENTORNO Y ARRANQUE
- Mata antes de instalar: Codync Dev, codync-host, codync-dev-hostw.
- Host: `cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
  (la prueba de symlink de Windows puede fallar por privilegios, error 1314: anótala como conocida).
- Escritorio: `cd apps/desktop && npm ci && npm run typecheck && npm test`.
- Paquete: `npm run dist:win:dev`; instala `.\dist\codync-dev-windows-x64.exe /S`;
  abre con un puerto nuevo `--remote-debugging-port=NNNN`.
- Si el host no arranca: `resources\codync-host.exe install --port 19223`.
- Datos: %USERPROFILE%\.codync-dev\codync.db y token en %USERPROFILE%\.codync-dev\token.

### Fragmento 2 del prompt 2

- Linux: solo si la VM `codync-linux` existe; si no, se omite y se reporta.
- No compiles iOS.
- Memoria: si el host frena turnos, sube umbrales con `setHealthThresholds` y devuélvelos a 70/85 al acabar.
  Puedes matar rustc, cargo, java y claude; NO cierres Deska.

PRUEBAS AUTOMÁTICAS NUEVAS
- `node tools/host-smoke.mjs` (host real con carpeta temporal; salud, diagnóstico redactado, auditoría,
  empresa, auth y SCIM). Debe dar 6/6.
- `node tools/app-smoke.mjs <puerto>` con la app abierta (roster, Ajustes, Canales, Ctrl+K, y descartar
  el asistente si sale).
- `node tools/memory-budget.mjs` y `node tools/i18n-audit.mjs es --strict` (0 claves pendientes).

PRUEBAS EN VIVO EN ESCRITORIO, en este orden
1. Primer arranque: sale la guía de 4 pasos; "Ahora no" la cierra; Ajustes → Primeros pasos muestra estados.
2. Extensiones → Canales: cuatro tarjetas con estado; abrir Telegram/Slack/Discord.
3. En un canal de prueba con bot asignado: añadir y borrar una nota; crear una rutina del canal y verla
   listada; activar/desactivar en Rutinas.
4. Ajustes → General → Canales: mismos controles y botón a Extensiones.
5. Editar un bot: Overview (agente, modelo, carpeta, permisos, skills, conectores, tokens de hoy),
   subir una foto PNG/JPG (<300 KB), verla en roster y chat, quitarla; Duplicar desde el menú del bot;
   Tareas (aprobaciones/rutinas/hilos) y Archivos (navegar, subir, copiar ruta).
6. Ctrl/Cmd+K: abrir, buscar "Canales", saltar a Extensiones/Canales.
7. Panel Navegador: abrir, comprobar que Chrome queda a la derecha y al frente, navegar a example.com,
   abrir segunda pestaña, cambiar, modo compacto (recordar al reabrir), cerrar todas.
8. Ajustes → Sistema → Copiar diagnóstico: pegar y comprobar que no hay tokens ni claves.
9. Empresa: OIDC con un emisor https real y discovery; token SCIM; GET/POST/DELETE /scim/v2/Users
   con Bearer; comprobar miembros en Ajustes; auditoría de cada acción.
10. Rutinas en paralelo: crear dos rutinas manuales triviales, lanzarlas con <3 s de diferencia y
    comprobar dos procesos de agente y dos resultados.
11. Regresión: enviar mensaje, aprobación (aprobar/rechazar), pestañas de archivos, secciones anidadas,
    tema y RTL, canales existentes.
12. Apagar la app y reabrir: borrador sin enviar pregunta antes de salir; el onboarding no reaparece.

### Fragmento 3 del prompt 2

QUÉ NO EXISTE (no lo reportes como fallo)
- Navegador fase 2 (página incrustada), publicar bots, login por miembro de empresa.
- Traducciones de fr/de/pt-BR/ru/ja/ko/zh-Hans/zh-Hant: caen a inglés a propósito.
- Discord/Telegram/Slack sin cuentas reales: si no hay cuenta, anótalo como no probado, no como fallo.

REPORTA EN ESPAÑOL
- Tabla por prueba: esperado, obtenido, captura, y si deja línea de auditoría.
- Comandos ejecutados y su resultado; fallos con reproducción mínima y archivo/línea.
- Qué no se pudo probar y por qué.
- Deja la app abierta, el árbol limpio salvo archivos ajenos, y di el SHA final.

## Prompt 3

### Fragmento 1 del prompt 3

promt 3:# Prompt: una sola integración, una sola compilación y una sola prueba en vivo

Eres el agente que **integra, compila y prueba una vez** todo lo que otros agentes dejaron en ramas y worktrees separados. No añadas funciones nuevas: junta, compila, prueba, corrige lo que falle y reporta. Responde en español.

Reglas del proyecto que mandan (`CLAUDE.md`): construir encima y nunca deshacer lo de otros, archivos de menos de ~500 líneas, paridad entre clientes, traducciones en los 10 catálogos, nada de código iOS/Mac (no escribas ni compiles iOS), el APK de Android solo se compila en el paso que dice abajo. **No cierres Deska ni la app de nadie. No uses `taskkill /IM cargo.exe`** (mata las compilaciones de otros agentes); mata solo los procesos que tú lanzaste.

## 1. Qué hay que integrar y dónde está

Proyecto principal: `C:\Users\Edwin Villeda\Documents\Codync`, rama `feat/openbot-marketplace` (punta `d9acc256` al escribir esto). En su carpeta hay cambios sin commit de **otro agente** (`docs/features/routine-flows.md` y varios `*.html`, `docs/plans/bundled-tools-production.md`, `docs/plans/design-inside-codync.md`, `pwsh.cmd`): **no los toques**. Integra en un worktree aparte y haz fast-forward al final solo si todo está verde.

Todo mi trabajo vive en una cadena de ramas. **La rama que lo contiene todo es `feat/n8n-android`** (worktree `C:\Users\Edwin Villeda\Documents\Codync-n8n-android`, punta `867d5569`, base `43c59885`). Contiene a `feat/comfy-sidebar` (punta `3dcc60d6`, worktree `Codync-comfy-sidebar`) y 5 commits más. Son 24 commits que el principal no tiene y **ninguno de los dos últimos está en origin** (`bc39f467`, `867d5569`); respáldalos con push de la rama antes de integrar.

| Bloque | Commits (rama `feat/n8n-android`) | Qué es |
| --- | --- | --- |
| ComfyUI embebido | `1f7ada84`…`3dcc60d6` (19) | Ejecutor compartido de herramientas (`host/src/tools/`), ComfyUI v0.44.0 fijado en `packaging/comfy/version.json`, descarga en el primer uso (`comfy_install.rs`), ventana/tarjeta/botón en la barra lateral del desktop, sección en Ajustes de Android, frase en el encabezado del TUI, 7-Zip empaquetado, servidor MCP `codync-comfyui` |
| n8n en el teléfono | `fc4280e0`, `83cc9725`, `bdcdd85b` | Host: `n8nWorkflows`, `n8nSetWorkflowActive`, `n8nExecutions` (`host/src/n8n_remote.rs`); el teléfono nunca recibe la cookie de sesión de n8n. Android: `N8nSection.kt`, `AppStoreN8n.kt`, `N8nStatusTest.kt`. Doc `web/content/docs/n8n.mdx` |
| n8n en desktop y terminal | `bc39f467` | Desktop: Ajustes → n8n lista los workflows con interruptor y las últimas ejecuciones (`views/settings/N8nWorkflows.tsx`, `shared/n8n-workflows.ts`, +3 tests, demo en `fake-host.ts`). TUI: hoja "n8n: workflows and runs…" en la paleta (`host/src/tui/manage/n8n.rs`, `sheets/n8n.rs`) |
| Comandos `/n8n` y `/comfy` | `867d5569` | Host: `host/src/chat/tool_commands.rs` (nota antes del mensaje; una skill con el mismo id gana). Aparecen en la lista `/` de desktop (`slash-commands.ts`), Android (`SlashCommands.kt`) y TUI (`tui/app/slash.rs`). iOS **no** se tocó |
| Traducciones | varios | 10 catálogos en `apps/ios/Kit/Sources/CodyncKit/Resources/Languages` (JSON con CRLF). Las traducciones nuevas las redacté yo; nadie nativo las revisó |

Ya están en el principal y solo hay que **verlos funcionar**: la isla dinámica (paginación de uso `pageOf`, arrastrar archivos) y el nodo **Codync Bot** de n8n (`host/n8n-nodes-codync`).

Ramas y worktrees ajenos con trabajo **no integrado y no probado** (otros agentes; solo anótalo en el reporte, no lo mezcles aquí): `feat/android-openbot-parity` (37 commits, ya en origin), `feat/hermes-integration` (worktree `Documents\hermes-integration`, ~122 archivos sin commit), Mercury Agent (worktree de Deska, ~108 archivos sin commit), `feat/island-restore` (contiene los commits de la isla ya integrados). `feat/comfy-bundled` (`298cad5b`) es la versión antigua de ComfyUI: sus dos commits distintos (`64a5c993`, `3d4fea6a`) están reemplazados por los de `comfy-sidebar`; no los traigas.

## 2. Integración

1. `git fetch`. Crea un worktree nuevo, por ejemplo `Codync-integrate-final`, con una rama nueva desde `feat/openbot-marketplace`. Enlaza `apps/desktop/node_modules` al del principal con una junction (para borrarla después usa `[System.IO.Directory]::Delete(ruta, $false)` en PowerShell o `rmdir` en cmd, nunca `Remove-Item -Recurse`).
2. `git merge feat/n8n-android`. Puntos de conflicto probables: los 10 `Languages/*.json` (toma la versión del principal y vuelve a añadir las claves nuevas con un script idempotente en Node; escribe con CRLF), `host/src/api/host.rs`, `host/src/api/devices.rs` (lista `LOCAL_ONLY`), `host/src/api/events.rs` (`LOCAL_EVENTS`), `host/src/mcp.rs`, `host/src/tui/app/ui.rs` y `overlay_keys.rs`, `apps/android/.../SettingsView.kt`. Conserva siempre lo de ambos lados; nunca borres un cambio ajeno.
3. Claves de traducción nuevas que deben existir en los 10 idiomas tras el merge: las del bloque n8n del teléfono (`Latest runs`, `No workflows yet.`, `{count} more in n8n`, `Start n8n to see its workflows and runs.`, `Succeeded`, `Crashed`, `Waiting`, `Unknown`, `n8n isn't set up on this computer yet. Choose its folder in the desktop app.`, `The workflows n8n runs on the computer. Turning one on publishes it there. Its editor opens in Codync on the computer.`), `Workflows`, `Turning one on publishes it in n8n.`, `n8n: workflows and runs…`, `Do it with the computer's n8n workflows`, `Make an image with ComfyUI` y las de ComfyUI de la rama. El test de auditoría de traducciones del host las revisa.
4. No subas la versión salvo que vayas a llevar esto a `main` (regla de `CLAUDE.md`: parche/menor en `apps/project.yml`, `host/Cargo.toml` + `Cargo.lock`, `apps/desktop/package.json` + lock, sin tocar `CURRENT_PROJECT_VERSION`; Android además `WhatsNew.md`).

## 3. Compilación (una sola vez)

Entorno de esta máquina: `-j 1` y sin símbolos de depuración, o cargo muere por memoria; lanza lo largo desacoplado y escribe a un log (las tareas en segundo plano mueren al cerrarse la sesión):

```
Start-Process cmd -ArgumentList '/c','cd /d <worktree>\host && set "CARGO_PROFILE_DEV_DEBUG=0" && cargo clippy -j 1 --all-targets -- -D warnings > %TEMP%\final-clippy.log 2>&1 & echo FINISHED >> %TEMP%\final-clippy.log' -WindowStyle Hidden
```

Orden: `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test -j 1 --bin codync-host` (el paquete no tiene lib; antes de mi trabajo hay un test conocido que falla por symlinks en Windows, anótalo sin tocarlo) → desktop: `node node_modules/typescript/bin/tsc --noEmit -p tsconfig.json` y `npm test` (esperado: 535 pasan, 24 omitidos, 0 fallos) → Android: compilar y pasar los tests unitarios de `apps/android` (entre ellos `SlashCommandsTest`, `N8nStatusTest` y los de ComfyUI) **solo aquí**; ese código nunca se compiló. Disco: comprueba que quedan más de 20 GB libres antes (la compilación fresca de cargo pesa varios GB; solo borra lo regenerable).

## 4. Pruebas en vivo (una sola pasada, en Windows)

Usa **solo** el entorno aislado de Codync Dev, nunca la app de producción. En el worktree de integración: `node tools/native-host.mjs dev` (host de desarrollo) y luego `npm run dev -- --cdp` en `apps/desktop` (sandbox propio: datos en `~/.codync-dev-<carpeta>`, puerto desde 19300, CDP en puerto+10000). Antes de cada paso comprueba con `/health` que el host que responde es el tuyo (`binaryPath`): otro agente puede desplazar el host compartido del puerto 19223. Maneja el desktop con CDP o Computer Use y toma capturas.

1. **Isla dinámica**: la pestaña Uso pagina de dos en dos con las flechas; arrastrar un archivo sobre la isla muestra "Drop to send to {bot}" y lo envía.
2. **ComfyUI** (descarga ~2,5 GB la primera vez; hazlo una vez, el sandbox la conserva en `~/.codync-dev-<carpeta>`): el botón de la barra lateral lleva la marca; "Descargar" muestra el avance y luego descomprime; el editor abre sin login; el encabezado del TUI y la sección de Ajustes en Android dicen lo mismo. Sin modelo no se genera imagen: no lo cuentes como fallo, pero registra que **no se probó una generación real**.
3. **n8n** (requiere la carpeta de la instalación: `C:\Users\Edwin Villeda\Documents\automan`, con `packages/cli/bin/automan`, y Node suficiente): Ajustes → n8n arranca y para; con n8n corriendo aparece la lista de workflows con interruptor y las ejecuciones; publicar y despublicar un workflow cambia el estado **en n8n** (las llamadas a `/api/v1` con `X-AUTOMAN-API-KEY` nunca se probaron contra un n8n real: es lo más probable que falle). Crea un workflow mínimo con el nodo **Codync Bot** ("Send a message", esperar respuesta) y comprueba que el bot contesta.
4. **Comandos `/n8n` y `/comfy`**: escribe `/` en un chat de desktop y de TUI (y en Android): salen después de las skills; `/n8n lista mis workflows` hace que el bot use el conector de n8n; `/comfy un zorro rojo` hace que use las herramientas de ComfyUI (o diga que no está corriendo). Con n8n parado el bot debe decirlo, no inventar.
5. **TUI** (`codync-host tui`): paleta de comandos → "n8n: workflows and runs…": flechas, Enter publica/despublica, `s` arranca/para, `r` refresca, Esc cierra.
6. **Seguridad**: un dispositivo remoto (Caller::Device, p. ej. el teléfono emparejado o una llamada por `/channel`) recibe `n8nStatus` **sin** `auth`/cookie de sesión y no puede llamar a las acciones que están en `LOCAL_ONLY`.

### Fragmento 2 del prompt 3

Los pasos de Android e iOS que necesiten dispositivo y no puedas hacer, anótalos como "no se pudo ejecutar" con el motivo.

## 5. Si todo está verde

1. Haz un solo commit de arreglos por área (mensajes tipo Conventional Commit; termina cada commit con `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`).
2. Fast-forward de la rama de integración a `feat/openbot-marketplace` en el checkout principal **solo si** el árbol de ese checkout no pisa los cambios sin commit del otro agente (comprueba con `git status`). Push solo si el usuario lo pide.
3. Limpieza: detén tu `npm run dev` y tu host de desarrollo, borra la junction de `node_modules` con `rmdir`/`Directory.Delete`, y luego el worktree. Elimina artefactos regenerables que hayas creado.

## 6. Reporte final (obligatorio)

Una tabla por bloque con: compilado (sí/no), tests (cuántos pasan), probado en vivo (sí/no/motivo) y capturas. Lista aparte lo que **no pudiste probar** (generación real de imagen, n8n real, Android, iOS), los conflictos que resolviste y cómo, y cualquier fallo conocido que encontraste sin arreglar.

## Prompt 4

### Fragmento 1 del prompt 4

promt 4:
Necesito que actúes como coordinador de integración y validación de Codync Dev. Varios agentes trabajaron en funcionalidades distintas. Reúne los trabajos terminados en un worktree de validación y realiza una campaña conjunta de compilación, pruebas y uso real sobre el mismo código integrado. Corrige los errores que encuentres y termina la campaña sin pedirme confirmación entre bloques.

Para este encargo queda autorizada la compilación y las pruebas de desarrollo que estaban pendientes por la instrucción anterior de continuar sin compilar. El resultado debe ser un único instalador final de Codync Dev que contenga los trabajos reunidos y un informe de sus pruebas. Reutiliza compilación incremental y artefactos compatibles. Si una corrección exige recompilar, registra la causa y repite las comprobaciones afectadas. Cada plataforma requiere su propia herramienta de compilación.

1. Ubicaciones y estado comprobado

El checkout compartido es C:\Users\Edwin Villeda\Documents\Codync. En la inspección de este traspaso estaba en feat/openbot-marketplace, commit corto 717e6909, con cambios ajenos sin commit y archivos de otros encargos. Vuelve a inspeccionarlo porque otros agentes pueden seguir trabajando.

La entrega de Hermes está en C:\Users\Edwin Villeda\Documents\hermes-integration, rama feat/hermes-integration, y está subida a origin/feat/hermes-integration.





HEAD entregado 99a65e948491ae08f7d0be84589b9783ceff977a.



Base de esta integración 43c59885362748879654888be5afc7d460a0811f.



El worktree de Hermes estaba limpio y sincronizado con su rama remota.



Hay diez commits por H1–H10 y un commit final de entrega.



La versión de la entrega es 2.39.0. Al integrar, comprueba la versión vigente y evita rebajar versiones aportadas por otros agentes.

El repositorio de referencia de Hermes es C:\Users\Edwin Villeda\Documents\ref-repos\hermes-agent, fijado en 10d25f8ad65fdf71304e9272959421a0c22e5d23. Conserva esa revisión y verifica que esté limpio. El plugin oficial Home Assistant se tomó de NousResearch/hermes-official-plugins, commit e7a218f26db88426ab3e7376f0e000b6e356f5e7, subdirectorio homeassistant/.

Existe además C:\Users\Edwin Villeda\Documents\Codync-windows-validation, rama integrate/windows-all-additions, observado en 2420c77d. Inspecciónalo para identificar una posible integración conjunta ya iniciada. Su existencia no acredita que incluya Hermes ni que haya sido probado.

2. Reunir los trabajos de otros agentes

Lee AGENTS.md, CLAUDE.md y las guías de desarrollo y aislamiento del checkout vigente. Ejecuta git worktree list, revisa estados, ramas, commits y documentos de entrega. Muestra la carpeta y rama donde trabajarás.

Usa un worktree propio para la integración conjunta. Puedes reutilizar el de validación si está disponible y conoces su estado. Conserva los cambios ajenos del checkout compartido y de cada worktree. Evita stash, reset, cambios de rama y limpiezas en el principal. Si necesitas actualizar referencias remotas, usa fetch y trabaja en el checkout aislado.

Estas ramas aparecían en el inventario y son candidatas que debes contrastar con sus entregas y diferencias reales.





feat/openbot-marketplace.



integrate/windows-all-additions.



feat/hermes-integration.



feat/openbot-windows.



feat/android-openbot-parity.



deska/Implementar-Pendientes-Mercury-Agent-8a65c136.



feat/routine-flows-host y feat/routine-flows-ui.



deska/Implementar-Motor-de-Flujos-de-Rutina-5fa5448a.



feat/n8n-android.



feat/comfy-bundled y feat/comfy-sidebar.



feat/android-widgets-v2.

Este inventario indica ubicación, no aprobación automática para incorporar cualquier prototipo. Determina qué entregas están terminadas y pertenecen a la campaña solicitada. Comprueba la ascendencia de commits para evitar traer dos veces el mismo trabajo. Lee los diffs y resuelve los conflictos conservando las funciones de todos los agentes. Los documentos de planificación no demuestran que una función esté implementada.

Antes de la compilación final deja un manifiesto con cada rama o commit incorporado, su procedencia, conflictos resueltos y el SHA conjunto que vas a probar. Si un trabajo sigue activo, coordina su entrega concreta y continúa con las partes independientes. Un estado ajeno desconocido debe aparecer en el informe con su impacto.

La incorporación de Hermes en el worktree conjunto de validación está autorizada por este encargo. La publicación en ramas compartidas de integración, dev o main conserva la condición anterior del usuario de indicar «llévalo a dev». Conserva el resultado revisable en tu rama de validación.

3. Documentos de Hermes que debes leer

En el worktree de Hermes están los informes siguientes. Después de integrar, usa sus copias del worktree conjunto.





docs/integrations/hermes-entrega.md contiene el resumen en español, procedencia, límites y validaciones pendientes.



docs/architecture/hermes-integration.md contiene los contratos, regeneración de inventarios y criterios de aceptación.



docs/integrations/hermes-h4-native-channels.md describe Signal, Email, Home Assistant, pairing, firmas, adjuntos y autorizaciones.



docs/integrations/hermes-h8-voice.md describe NeuTTS, streaming, cancelación y wake móvil.



web/content/docs/agents.mdx, memory.mdx, channels.mdx, routines.mdx, teamwork.mdx, voice.mdx, whatsapp.mdx y connectors-and-skills.mdx contienen los recorridos documentados del usuario.

La entrega anterior fue revisada por fuente. Se aplicó rustfmt a 141 archivos Rust, se analizaron 14 fuentes TypeScript con Babel y se analizaron fuentes Python y scripts remotos mediante AST. Hay regresiones escritas. La compilación, clippy, typecheck, las pruebas ejecutables y los recorridos reales quedaron pendientes. Usa los resultados nuevos para determinar si los criterios originales se cumplen y repara cualquier integración incompleta que descubras.

4. Trabajo entregado y dónde encontrarlo

H1. Proveedor Hermes y ACP

El instalador fija la revisión. El cliente ACP conserva herramientas, aprobaciones y aclaraciones manuales incluso cuando el bot aprueba herramientas automáticamente. La cancelación llega al ejecutor. El proceso tiene propietario y cierre acotado.

Revisa host/src/agent/backends/hermes.rs, backends/catalog.rs, agent/bot/updates.rs, updates_tests.rs, user_input.rs, session.rs, request_control.rs y agent/hermes/.

H2. Habilidades y mejoras

Se incluyen 185 habilidades completas con 1.281 archivos auxiliares. Se reutiliza el índice y los bundles de Codync. Las mejoras por bot conservan hasta 50 versiones y permiten restaurar mediante revisión esperada. Las propuestas de nuevas habilidades usan revisión del usuario.

Revisa host/src/market/hermes/skills.json, skills.tar.gz, LICENSE, market/bundled_skills.rs, skill_history.rs, chat/skills.rs, agent/gear/skills.rs, hub/skill_updates.rs y tools/import-hermes-skills.py.

H3. Memoria, FTS y Honcho

Se mantiene la memoria persistente y Dreamer de Codync. El historial tiene índice FTS y filtros de fechas. El ejecutor Hermes permite memoria del perfil, builtin u Honcho con sus credenciales existentes. Un fallo al iniciar Honcho debe comunicarse.

Revisa host/src/store/session_search.rs, store/entries.rs, store/schema.rs, chat/memory/tools.rs y agent/hermes/{settings.rs,acp_owner.py,state_probe.py}.

H4. Canales y notas de voz

Signal y Email reutilizan adaptadores y perfiles nativos. Home Assistant usa el plugin oficial fijado. El host controla conexiones, conversaciones, pairing, generaciones de autorización, aprobaciones, colas, adjuntos y webhook Email firmado. Telegram, WhatsApp, Slack y Discord reutilizan la transcripción del host. Los archivos entrantes usan uploads estándar y los salientes se resuelven desde snapshots del bot con UUID, tamaño y SHA256.

Revisa host/src/native_channels/, store/native_channels.rs, native_channels.sql, agent/bot/channel_input.rs, native_revocation.rs, channels/voice_notes.rs, las carpetas de cada plataforma y voice/transcription/. Los campos genéricos de Apps permiten configuración y código inicial. Las acciones adicionales de estado, pairing y revocación individual están expuestas por API.

H5. Rutinas con destino

Se importan 16 blueprints. El scheduler usa un outbox persistente por corrida y destino, captura la autorización original y espera recibos. WhatsApp usa la sesión enlazada existente. Los cuatro clientes editan destinos con control de cambios simultáneos.

Revisa host/src/routines/{blueprints,delivery,bindings,output_text,outputs,retry,runner}.rs, store/routine_delivery.rs, market/hermes/blueprints.json, los editores Routine de desktop/iOS/Android y tui/manage/routines.rs.

H6. Entornos por bot y SSH

El ejecutor aplica la política nativa en construcción, turnos y cierre. Incluye local, Docker, SSH, Modal, Daytona y Vercel Sandbox. SSH sincroniza el proyecto en ambos sentidos y controla trabajos de fondo. hermesEnvironmentHealth comprueba explícitamente la revisión guardada y usa un recurso de prueba propio con plazo máximo de 60 segundos.

Revisa host/src/agent/hermes/{installation.rs,settings.rs,acp_owner.py,ssh_owner.py,ssh_workspace.py,ssh_processes.py,health.rs,environment_probe.py}, sus regresiones y los cuatro editores HermesRuntimeCard o runtime TUI.

H7. Subagentes y RPC

Las rutas guardadas exponen identificadores autorizados de proveedor/modelo. Cada hijo tiene presupuesto de iteraciones, concurrencia e inactividad. Un cupo del sistema operativo se comparte entre conversaciones y rutinas del bot. Los hijos entregan el resumen a su padre. La CLI permite llamar herramientas mediante la conexión del host.

Revisa host/src/agent/hermes/delegation_routes.py, delegation_quota.py, chat/team/, cli/tool.rs, tests/tool_cli_e2e.rs y tests/fixtures/team_agent.py.

H8. Voz local y activación móvil

NeuTTS 1.4.1 usa Jo, caché offline y infer_stream. El host entrega WAV PCM16 mono de 24 kHz en frames de hasta 500 ms, con cola acotada y cancelación por solicitante. Desktop, iOS y Android conservan sus conexiones STT y eligen TTS por separado. Hey Hermes usa reconocimiento nativo local y opt-in durante una conversación en primer plano.

Revisa host/src/voice/{neutts.rs,streaming.rs,synthesis.rs,neutts_stream.py}, api/host.rs, los CloudSpeechEngine y ajustes de voz de los tres clientes, VoiceSpeechStream.swift, WakePhrase y sus pruebas. TUI mantiene conversación de texto.

H9. API local OpenAI compatible

Los endpoints /v1/models, /v1/chat/completions, /v1/health y /v1/pricing exponen bots con claves del vault. La solicitud usa la cola, herramientas y aprobaciones del bot. La desconexión cancela su trabajo. El streaming de esta API entrega el resultado final tras mantener viva la conexión.

Revisa host/src/api/openai.rs, openai_keys.rs, chat/team/openai.rs, api/http.rs y la documentación de agentes. Los precios desconocidos se expresan como null.

H10. Catálogo MCP y conexiones

Se importan 66 manifiestos, se evitan duplicados con OpenBot y se conservan los servidores existentes. OAuth reutiliza PKCE y vault. n8n pide su instancia y Asana usa las credenciales de aplicación registradas por el usuario.

Revisa host/src/market/{hermes.rs,hermes_apps.rs,hermes_apps/,hermes/apps.json,oauth.rs,registry.rs,install.rs}, los modelos y pantallas Marketplace de los clientes y tools/import-hermes-mcps.py.

5. Commits de la entrega Hermes





H1 8ab27514, proveedor fijado y preguntas ACP.



H2 987493c1, habilidades y versiones.



H3 032caae6, FTS e historial fechado.



H4 be570f37, canales nativos y transcripción.



H5 c8a42880, blueprints y entrega de rutinas.



H6 6e48b589, ejecutor, ajustes, entornos y SSH.



H7 e1ea819b, rutas, cupos y CLI RPC.



H8 dfefbc3d, voz progresiva y wake móvil.



H9 7b957766, endpoints OpenAI locales.



H10 a515e6cb, catálogo y cableado compartido de H1–H10.



Cierre 99a65e94, informe español y limpieza de fuente.

Integra la rama completa o su secuencia completa. H10 contiene cableado compartido entre fichas y varios commits dependen de otros. Evita seleccionar una ficha aislada y concluir que el conjunto fue incorporado.

6. Compilación y comprobaciones conjuntas

Primero fija el SHA conjunto y comprueba dependencias, herramientas y espacio disponible. Usa una instalación coherente con los lockfiles del checkout integrado. Conserva los recursos ya preparados cuando sean compatibles. Registra los comandos y sus salidas en logs con datos sensibles excluidos.

Desde host/, ejecuta los checks del repositorio.

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

Si la guía vigente prescribe nextest en Windows, emplea la configuración del repositorio y registra esa elección. Corrige las pruebas que fallen intermitentemente. Una repetición exitosa debe conservar el historial del fallo.

Desde apps/desktop/, prepara dependencias con el lockfile integrado y ejecuta lo siguiente.

npm ci
npm run typecheck
npm test

Los adaptadores Python tienen regresiones propias en host/src/agent/hermes/. Usa Python compatible y ejecútalas en procesos separados cuando sus dobles de módulos puedan interferir.

python -B -m unittest discover -s host/src/agent/hermes -p test_owner_overlay.py
python -B -m unittest discover -s host/src/agent/hermes -p test_delegation_quota.py
python -B -m unittest discover -s host/src/agent/hermes -p test_environment_probe.py
python -B -m unittest discover -s host/src/agent/hermes -p test_ssh_owner.py
python -B tools/verify-hermes-imports.py ..\ref-repos\hermes-agent

Estos últimos comandos se ejecutan desde la raíz del worktree conjunto. Ajusta sólo la ubicación del clon de referencia si tu worktree está en otra carpeta. Distingue skips por capacidades POSIX de pruebas realmente ejecutadas.

### Fragmento 2 del prompt 4

Para iOS usa un Mac disponible y la configuración Dev del repositorio. Ejecuta las pruebas de CodyncKit/CodyncUI y el recorrido en el simulador propio del worktree. Windows no proporciona Xcode. Registra esa limitación si falta un Mac y continúa las pruebas disponibles. Para otros módulos incorporados, como cloud o relay, ejecuta sus comprobaciones aplicables cuando sus fuentes hayan cambiado.

Haz la compilación final de Windows desde apps/desktop/ con el comando existente.

npm run dist:win:dev

Ese script ya incluye la preparación del host de desarrollo y el empaquetado Electron. Comprueba que incorpora el host del mismo SHA y arquitectura. Evita generar antes otro instalador equivalente para cada rama.

Identifica el instalador realmente generado en apps/desktop/dist/. El encargo original esperaba codync-dev-windows-x64.exe, pero verifica el nombre y arquitectura actuales. Antes de instalar y abrir la nueva versión, coordina la parada de la variante Dev anterior y de codync-dev-hostw, verifica sus rutas y protege los datos existentes con el procedimiento de backup del proyecto. Usa el instalador Dev y el argumento /S si sigue siendo el mecanismo vigente.

7. Pruebas en vivo de la misma versión integrada

Usa Codync Dev instalada para los flujos que requieren host empaquetado, sidecars, permisos e integración Windows. Comprueba la versión y el ejecutable que realmente atiende las llamadas. Un host antiguo debe detectarse antes de interpretar un error de protocolo como defecto de la función nueva.

Reutiliza las conexiones, perfiles, cuentas y claves existentes de Codync Dev. Las credenciales y backups sensibles permanecen fuera de Git y de los logs. Los servicios o motores ausentes se preparan mediante el mecanismo de instalación correspondiente cuando estén dentro del encargo. Respeta los pins de Hermes y los extras bajo demanda.

Recorre H1–H10 en esa misma instalación y conserva evidencia de cada resultado.





Hermes inicia una conversación, usa una herramienta, presenta una aprobación y permite denegarla. Prueba una aclaración con opciones en modo automático y comprueba su selección manual. Stop y cancelación deben alcanzar el trabajo real.



Importa una habilidad del catálogo, úsala, registra una mejora y restaura una versión. Comprueba aislamiento entre bots y rechazo de una revisión obsoleta.



Guarda memoria, cierra y reabre sesión y consulta lo hecho en un intervalo de fechas. Comprueba que el nudge produce persistencia efectiva. Prueba Honcho con el perfil existente si está disponible y conserva errores de autenticación como tales.



Instala/configura Signal y Email mediante Apps usando las sesiones existentes. Empareja una DM, revoca autorización y prueba una aprobación vinculada al autor. Recibe una nota de voz y adjuntos y entrega un archivo. Prueba el webhook Email con firma válida, inválida y repetida. Comprueba Home Assistant con entidades de prueba seleccionadas.



Crea una rutina desde un blueprint y entrega su informe a un destino de prueba ya identificado del usuario. Prueba indisponibilidad temporal, recibos, reconexión, idempotencia y cancelación. Una entrega ambigua debe conservar su estado y evitar repetición automática. Comprueba los destinos en los editores afectados.



Selecciona entorno por bot y usa Check saved environment. Prueba local, Docker y una conexión SSH disponible. En SSH verifica archivos en ambos sentidos, edición local concurrente, trabajo de fondo, Stop, caída de conexión y ausencia de procesos huérfanos tras el lease. En los backends serverless disponibles comprueba salud y lifecycle con sus costes y recursos registrados.



Guarda rutas distintas de proveedor/modelo, delega y comprueba la ruta efectiva y el resumen recibido por el padre. Satura el cupo compartido entre chat y rutina, cancela un hijo y verifica que libera capacidad. Ejecuta un script con varias llamadas RPC al host y comprueba permisos, plazos y efectos.



Prepara NeuTTS mediante el mecanismo nativo bajo demanda, valida streaming antes de terminar la inferencia e interrupción al hablar. Comprueba cierre, cancelación anticipada y conservación de la conexión STT. Prueba Hey Hermes en iOS/Android disponibles, su opt-in, permisos, mute y salida a segundo plano.



Crea una clave local limitada a un bot, consulta modelos y realiza una conversación con un cliente OpenAI compatible. Prueba herramientas, aprobación, desconexión, timeout, revocación y rechazo de acceso a otro bot. Comprueba la semántica de streaming final documentada y precios desconocidos.



Comprueba catálogo MCP, orígenes, deduplicación, conexiones anteriores y estado. Recorre los flujos de conexión aplicables con las cuentas existentes y comprueba que los canales nativos no se lanzan como servidores MCP ficticios.

Para envíos externos usa los chats, destinatarios y entidades de prueba del propio usuario ya identificados para la campaña. Si falta un destino concreto, documenta ese bloqueo y continúa los demás recorridos. Las pruebas dependientes de servicios ausentes deben quedar diferenciadas de los fallos de código.

Añade a esta campaña los recorridos de las otras entregas realmente integradas. Para OpenBot/Mercury, flujos de rutinas, n8n, ComfyUI, widgets y paridad Android/Windows, usa sus informes y criterios efectivos. Comprueba también regresiones básicas de arranque, bots anteriores, historial, conexiones, marketplace, rutinas, voz, ventanas y cierre de sidecars.

8. Límites conocidos que debes comprobar





Hermes admite Python 3.11–3.14. El extra NeuTTS de esta adaptación requiere 3.11–3.13 y modelos preparados. Verifica qué instalación estás ejecutando antes de correr hermes doctor y guarda su log con datos sensibles excluidos.



SSH requiere Python 3 y POSIX waitid(WNOWAIT) en el destino. Tiene límites de 10.000 archivos, 64 MiB por archivo y 128 MiB por proyecto. Excluye enlaces y credenciales. Una comparación previa al replace conserva una ventana breve del filesystem.



La pérdida de conexión SSH usa un lease de 20 segundos. La recuperación de archivos depende del destino. Los escapes deliberados de grupos de procesos requieren contención del sistema.



La salud del backend puede crear un sandbox. El plazo del RPC es 60 segundos y el timeout comunica limpieza no confirmada.



El máximo configurado de hijos es diez. Inactividad e iteraciones tienen semánticas distintas a un límite monetario o plazo total.



La frase Hey Hermes usa reconocimiento del sistema móvil y primer plano. La integración no incluye un runtime móvil TFLite de Hermes. TUI conserva texto.



Revisa en los informes los formatos o modos nativos que se rechazan expresamente. Si un criterio requerido depende de uno, completa esa integración antes de marcarlo aprobado.

9. Correcciones, entrega y criterio de cierre

Corrige los fallos de integración, compilación, pruebas y recorridos reales que encuentres. Conserva la intención de los cambios de los distintos agentes. Ante archivos compartidos, revisa el estado de todos los worktrees implicados y coordina las modificaciones. Mantén la lógica en el host y la paridad de clientes aplicable.

Los cambios corregidos deben probarse en la versión final correspondiente. Si cambian el código después de empaquetar, regenera el artefacto afectado y registra el nuevo SHA. Evita presentar pruebas de un binario anterior como evidencia del código corregido.

Entrega un informe en español que incluya lo siguiente.





Ramas y commits reunidos y SHA conjunto final.



Ubicación, versión, arquitectura y checksum del instalador Dev que se usó.



Ruta del host realmente ejecutado y su correspondencia con el build.



Comandos ejecutados y resultados, con enlaces a logs y evidencia.



Cada criterio H1–H10 y cada entrega adicional, marcado aprobado, fallido, bloqueado o sin ejecutar, con motivo concreto.



Errores encontrados, correcciones y comprobaciones repetidas por necesidad.



Pruebas de iOS, Android, Windows y otras plataformas disponibles, con sus limitaciones.



Recursos remotos creados y su limpieza, procesos que queden activos y tareas pendientes reales.

Deja abierto el build Dev final usado para la prueba, con Hermes disponible y una habilidad de ejemplo importada. Mantén el worktree conjunto y sus logs para revisión. El cierre completo exige compilación y criterios ejecutados con evidencia. Si un requisito depende de un recurso ausente, describe exactamente cuál y qué quedó sin demostrar. Continúa las acciones independientes autorizadas antes de terminar.

