# Prompt único de Android — trabajo entregado, ubicación, integración, compilación y prueba en vivo

Eres el agente que reunirá y validará todo el trabajo de Android descrito en los cuatro handoffs. Trabaja sobre un mismo HEAD integrado, conserva lo de todos los agentes y realiza una sola campaña conjunta de compilación y pruebas. Corrige lo que falle y entrega el reporte en español.

Este prompt reúne el trabajo de Android, sus ubicaciones y las dependencias compartidas del host que necesita el teléfono. Los hashes y estados siguientes son los que entregaron los agentes. Inspecciona el estado actual de las ramas antes de integrar.

## 1. Dónde está cada trabajo de Android

### Paridad OpenBot, Android y cambios compartidos

- Worktree: `C:\Users\Edwin Villeda\AppData\Roaming\Deska\worktrees\android-openbot-parity`
- Rama: `feat/android-openbot-parity`
- HEAD de entrega: `5d25ca7d`.
- Base de la rama: `09cd6c10`, 2026-11-07.
- Entrega: 41 commits desde la creación de la rama.
- Remoto: `origin` = `https://github.com/Thndrr00/Codync.git`.
- La rama se entregó empujada y sincronizada, con `0` commits de diferencia en ambos sentidos.
- Versión: **2.39.0**, ya en `apps/project.yml`, `host/Cargo.toml` y `apps/desktop/package.json`.
- Registro vivo: `docs/plans/openbot-mobile-parity.md`, con archivo por archivo.
- Inventario original: `docs/plans/openbot-android-inventory.md`. Es una foto previa, todavía dice "M5 is next" y conserva líneas "Missing". El registro actual del trabajo es `openbot-mobile-parity.md`.
- Todo el código de esta entrega se indicó commiteado y empujado. En su worktree hay archivos de scratch en `tools/` que nunca se commitean.

### n8n y ComfyUI para Android

- Worktree que contiene la cadena completa: `C:\Users\Edwin Villeda\Documents\Codync-n8n-android`.
- Rama: `feat/n8n-android`.
- Punta de entrega: `867d5569`.
- Base: `43c59885`.
- Contiene `feat/comfy-sidebar`, punta `3dcc60d6`, worktree `C:\Users\Edwin Villeda\Documents\Codync-comfy-sidebar`, y cinco commits más.
- El agente entregó una cadena de 24 commits que el principal no tenía. Indicó que `bc39f467` y `867d5569` todavía no estaban en origin. Comprueba su estado y respáldalos con push de la rama antes de integrar si siguen pendientes.
- La versión antigua de ComfyUI está en `feat/comfy-bundled`, punta `298cad5b`. Sus dos commits distintos, `64a5c993` y `3d4fea6a`, fueron reemplazados por los de `comfy-sidebar`. No los traigas.

### Hermes y sus cambios en Android

- Worktree: `C:\Users\Edwin Villeda\Documents\hermes-integration`.
- Rama: `feat/hermes-integration`.
- Rama remota: `origin/feat/hermes-integration`.
- HEAD entregado: `99a65e948491ae08f7d0be84589b9783ceff977a`.
- Base: `43c59885362748879654888be5afc7d460a0811f`.
- El worktree de Hermes estaba limpio y sincronizado con su rama remota.
- Hay diez commits por H1–H10 y un commit final de entrega.
- Versión de la entrega: 2.39.0.
- Referencia Hermes: `C:\Users\Edwin Villeda\Documents\ref-repos\hermes-agent`, fijada en `10d25f8ad65fdf71304e9272959421a0c22e5d23`.
- Home Assistant procede de `NousResearch/hermes-official-plugins`, commit `e7a218f26db88426ab3e7376f0e000b6e356f5e7`, subdirectorio `homeassistant/`.

Lee los documentos de Hermes que afectan al teléfono y sus servicios.

- `docs/integrations/hermes-entrega.md`.
- `docs/architecture/hermes-integration.md`.
- `docs/integrations/hermes-h4-native-channels.md`.
- `docs/integrations/hermes-h8-voice.md`.
- Las páginas `agents.mdx`, `memory.mdx`, `channels.mdx`, `routines.mdx`, `teamwork.mdx`, `voice.mdx`, `whatsapp.mdx` y `connectors-and-skills.mdx` en `web/content/docs/`.

### Checkout compartido y validación conjunta

- Proyecto principal: `C:\Users\Edwin Villeda\Documents\Codync`, rama `feat/openbot-marketplace`.
- Los handoffs registraron puntas distintas del principal: `717e6909` y `d9acc256`. Comprueba la actual.
- En el principal había cambios ajenos sin commit en `docs/features/routine-flows.md`, archivos `*.html`, `docs/plans/bundled-tools-production.md`, `docs/plans/design-inside-codync.md` y `pwsh.cmd`. No los toques ni reformatees.
- Puede existir un `.git/MERGE_RR.lock` de otro proceso de Git. No lo borres a ciegas.
- Existe `C:\Users\Edwin Villeda\Documents\Codync-windows-validation`, rama `integrate/windows-all-additions`, observado en `2420c77d`. Inspecciónalo para saber qué contiene.
- Otras ramas candidatas del inventario: `feat/openbot-windows`, `feat/android-widgets-v2`, `feat/routine-flows-host`, `feat/routine-flows-ui`, `deska/Implementar-Motor-de-Flujos-de-Rutina-5fa5448a` y `deska/Implementar-Pendientes-Mercury-Agent-8a65c136`. Verifica sus entregas y su relación con Android antes de incorporarlas.

## 2. Qué se trabajó en Android

### Paridad OpenBot

- **M9/M3.** Marcar no leído con línea NEW, leer en voz alta con velocidades, pila de imágenes y visor paginado, SVG, MP4, reconexión con jitter, foreground y timeouts de 15 segundos, parpadeo sin frames y espejo cifrado AES-GCM borrado al salir. Commits `e1f41424`, `75662f40`, `7bcab1d8`, `44bdea3c`, `65610594`.
- **M4, soporte.** Página Support con Discord/GitHub y log de eventos redactado, límite de 5.000 eventos y acciones de guardar/limpiar. Commits `65610594`, `44bdea3c`, `2e8be151`.
- **M2, flujos y WhatsApp.** Rutinas desde WhatsApp y salida a WhatsApp, correr en un hilo, pareo por código, cancelar tarea, modelo por enlace del flujo, canales desde el teléfono y configuración de WhatsApp/Telegram/Discord/Slack/GitHub. Commits `1f69202b`, `9f4a8ebe`, `50b37817`, `f283a04e`, `72825c7e`, `fbad89d8`, `5253ee59`, `a957af52`, `2bff9b66`.
- **M2, hilos.** Hilos por mensaje y por lista, chip "N respuestas", hoja del hilo, renombrar, archivar, borrar, no leídos y "Branch from here". Commit compartido `1b99386d`.
- **M5/M6.** Pausa de voz con slider, velocidad de lectura 0,5×–2×, avatar Plush/Flat en widgets, caras en reposo más tranquilas y flotación de grupo. Commit `f9abdf4a`.
- **M1/M7.** Coinbase, RevenueCat, TickTick, Proton Pass, Higgsfield, lista Apps con estado y paginación y ventana de uso gastado bajo el modelo. Commit `cb6a93b5`.
- **M8.** Panel de cola con Edit/Steer/Delete, pines, dirección por párrafo y nombres en hilos de tarea. Commits `1c0d6242`, `69ed3b18`, `b44563be`.
- **X1–X7.** Árabe/RTL real, compartir desde otras apps, bloqueo con PIN/biometría, espejo cifrado, live updates/push, split plegable y cara en notificaciones. Commits `f4834994`, `2ab25260`, `af145aef`, `47d32ec8`, `3b5bb3e3`, `c49a688a`.
- **M4, voz ElevenLabs.** Clave en la computadora, voces de la cuenta mediante `voiceVoices`, Scribe escucha, Flash 2.5 lee, siempre modo Speech y Test dice "OK". Refactor del host a `host/src/voice/elevenlabs.rs` y correcciones compartidas de ajustes. Commits `da4b4a59`, `e0fa238a`, `eb446f52`, `dd38e168`, `31c474ef`, `5d25ca7d`.
- **Documentación.** Inventario y plan OpenBot, `docs/features/voice-call.md`, privacidad/SEO y notas Play 2.39.0 en `apps/android/WhatsNew.md`. Commits `3f3df22e`, `fd41e980`, `779fc67f`, `31c474ef`, `5d25ca7d`.

### Pantallas y recorridos Android del checkout principal

- Extensiones → Canales, con estado, hojas de configuración, notas y rutinas por canal y miembros de empresa.
- Ajustes → General → Canales, con los controles y navegación a Extensiones.
- Bot → Details/Overview, con agente, modelo, carpeta, permisos, skills, conectores y tokens de hoy.
- Tareas del bot, con aprobaciones, rutinas e hilos.
- Archivos del bot, con navegación y copiar ruta.
- Foto del bot visible en roster y chat y acción Duplicar en el menú.
- Guía de primera vez tras emparejar, que no debe reaparecer al reabrir.
- Ajustes: Diagnóstico no existe en Android, según ese handoff.

### ComfyUI en Android

- Sección en Ajustes de Android para el ejecutor compartido de herramientas.
- ComfyUI v0.44.0 fijado en `packaging/comfy/version.json`.
- Descarga en el primer uso mediante `comfy_install.rs` y recursos compartidos del host en `host/src/tools/`.
- Servidor MCP `codync-comfyui` y 7-Zip empaquetado.
- El estado mostrado por Android debe coincidir con el ejecutor del equipo.
- La cadena de ComfyUI embebido corresponde a los 19 commits `1f7ada84`…`3dcc60d6`.

### n8n en Android

- Lista de workflows, cambio de estado activo y últimas ejecuciones.
- El teléfono nunca recibe la cookie de sesión de n8n.
- Host con `n8nWorkflows`, `n8nSetWorkflowActive` y `n8nExecutions`, en `host/src/n8n_remote.rs`.
- Android con `N8nSection.kt`, `AppStoreN8n.kt` y `N8nStatusTest.kt`.
- Commits `fc4280e0`, `83cc9725`, `bdcdd85b`.
- Documento `web/content/docs/n8n.mdx`.

### Comandos Android `/n8n` y `/comfy`

- Aparecen en la lista `/` del compositor, después de las skills.
- Android usa `SlashCommands.kt`.
- El host usa `host/src/chat/tool_commands.rs`, con nota antes del mensaje. Una skill con el mismo id gana.
- `/n8n lista mis workflows` debe utilizar el conector de n8n.
- `/comfy un zorro rojo` debe usar las herramientas de ComfyUI o comunicar que no está corriendo.
- Commit `867d5569`.
- Las traducciones están en diez catálogos compartidos de `apps/ios/Kit/Sources/CodyncKit/Resources/Languages`, con JSON CRLF. El agente indicó que las redactó y que no habían pasado revisión nativa.

### Hermes que utiliza Android

- **H1.** Selección del proveedor Hermes y conversación mediante el cliente ACP del host, con herramientas, aprobaciones, aclaraciones y cancelación.
- **H2.** Catálogo de 185 habilidades completas y 1.281 archivos auxiliares, mejoras por bot, hasta 50 versiones y restauración con revisión esperada.
- **H3.** Memoria persistente y Dreamer del host, historial FTS con fechas y selección de memoria del perfil, builtin u Honcho conservando las credenciales existentes.
- **H4.** Signal, Email y Home Assistant por los formularios genéricos de Apps, sesiones por conexión, código inicial de pairing, política de remitentes, notas de voz y adjuntos. El host controla autorización, aprobaciones, generaciones e inbox/outbox. Estado, pairing adicional y revocación individual también tienen API.
- **H5.** Los 16 blueprints de rutinas, destinos editables desde Android, entrega persistente incluida WhatsApp y control de ediciones simultáneas.
- **H6.** Tarjeta `HermesRuntimeCard.kt` para aplicar juntos entorno, memoria y delegación. Local, Docker, SSH, Modal, Daytona y Vercel Sandbox. La comprobación explícita `hermesEnvironmentHealth` usa la revisión guardada, puede crear un sandbox y tiene plazo de 60 segundos.
- **H7.** Rutas guardadas de proveedor/modelo por tarea, presupuesto de iteraciones, concurrencia e inactividad y cupos compartidos entre conversaciones y rutinas del bot. Los hijos entregan su resumen al padre.
- **H8.** NeuTTS 1.4.1 offline progresivo, selección TTS independiente de STT, interrupción al hablar y Hey Hermes mediante reconocimiento nativo local, opt-in y primer plano. WAV PCM16 mono 24 kHz, frames de hasta 500 ms y cancelación por solicitante.
- **H9.** API local del host compatible con OpenAI, con bots, claves del vault, herramientas y aprobaciones. Es una dependencia compartida de la entrega.
- **H10.** Catálogo de 66 manifiestos MCP, deduplicación con OpenBot, origen y estado de conexión y reutilización de PKCE/vault. n8n pide su instancia y Asana usa las credenciales de aplicación registradas por el usuario.

Los commits de Hermes que contienen estos cambios son `8ab27514`, `987493c1`, `032caae6`, `be570f37`, `c8a42880`, `6e48b589`, `e1ea819b`, `dfefbc3d`, `7b957766`, `a515e6cb` y el cierre `99a65e94`. Integra la rama completa o la secuencia completa porque H10 contiene cableado compartido y varios commits dependen de otros.

## 3. Archivos que debe revisar el agente de Android

La raíz habitual de la app es `apps/android/app/src/main/kotlin/com/codync/android/`. La del core es `apps/android/core/src/main/kotlin/com/codync/android/core/`.

- `core/VoiceProvider.kt`, `core/Voice.kt`, `core/CloudSpeechEngine.kt` y sus pruebas.
- `VoiceChatSettings.kt`, `AppStoreVoice.kt` y `MainActivity.kt`.
- `ThreadsSheet.kt`, `RepliesView.kt` y `ThreadHeader.kt`.
- `N8nSection.kt`, `AppStoreN8n.kt` y `SlashCommands.kt` en la raíz habitual de la app.
- `ComfySection.kt`, `AppStoreComfy.kt` y `SettingsView.kt` en esa misma raíz.
- Las pruebas `N8nStatusTest.kt`, `ComfyStatusTest.kt` y `SlashCommandsTest.kt` están en `apps/android/app/src/test/kotlin/com/codync/android/`.
- `HermesRuntimeCard.kt`, `ForkBotSettings.kt` y `AppStoreBots.kt`.
- `MarketplaceSheets.kt`, `core/Market.kt` y los formularios genéricos de Apps.
- `RoutinesView.kt`, `core/Routine.kt` y las pruebas de destinos de rutina.
- `apps/android/core/src/main/kotlin/com/codync/android/core/WakePhrase.kt` y `apps/android/core/src/test/kotlin/com/codync/android/core/WakePhraseTest.kt`.
- `apps/android/WhatsNew.md`.

Revisa las dependencias del host que usa el teléfono.

- `host/src/voice.rs`, `voice/elevenlabs.rs` y `api/host.rs`, con `voiceStatus`, `setVoiceKey`, `voiceSession`, `voiceModels`, `voiceVoices`, `voiceTranscribe` y `voiceSpeak`.
- `host/src/api/devices.rs`, con `setVoiceKey` solo loopback y las listas de acciones autorizadas.
- `host/src/n8n_remote.rs`, `chat/tool_commands.rs`, `tools/` y el ejecutor ComfyUI.
- `host/src/native_channels/`, `channels/voice_notes.rs`, las plataformas de mensajería y `voice/transcription/`.
- `host/src/routines/`, `store/routine_delivery.rs` y `market/hermes/blueprints.json`.
- `host/src/agent/hermes/`, especialmente `settings.rs`, `health.rs`, `environment_probe.py`, `acp_owner.py`, los adaptadores SSH y las rutas/cupos de delegación.
- `host/src/market/hermes/`, `hermes_apps.rs`, `oauth.rs`, `registry.rs` y `install.rs`.

### Archivos centrales de Android indicados en el prompt 1

- Android: `apps/android/core/src/main/kotlin/com/codync/android/core/VoiceProvider.kt`,  
`apps/android/app/src/main/kotlin/com/codync/android/VoiceChatSettings.kt`,  
`.../AppStoreVoice.kt`, `.../ThreadsSheet.kt`, `.../RepliesView.kt`, `.../ThreadHeader.kt`.

## 4. Reunir Android antes de compilar

1. Lee AGENTS.md, CLAUDE.md y los documentos de entrega. Muestra `pwd` y `git branch --show-current` del worktree donde trabajarás.
2. Ejecuta `git worktree list`, inspecciona las ramas y usa fetch para comprobar referencias remotas. Conserva los cambios y procesos de otros agentes.
3. Trabaja en un worktree de integración separado. No hagas stash, reset, checkout ni rebase sobre el principal compartido.
4. Comprueba ascendencia y diferencias para incorporar una sola vez OpenBot/paridad, la cadena n8n/ComfyUI y Hermes. Verifica el estado de las otras entregas Android antes de traerlas.
5. En los conflictos conserva las funciones de ambos lados. Los handoffs señalaron posibles conflictos en `host/src/api/host.rs`, `api/devices.rs`, `api/events.rs`, `mcp.rs`, las hojas del TUI, `apps/android/.../SettingsView.kt` y los diez `Languages/*.json`.
6. Para los catálogos de idioma, el handoff de n8n indica conservar la versión del principal y volver a añadir las claves nuevas mediante un script idempotente en Node, manteniendo CRLF.
7. Las claves nuevas de n8n incluyen `Latest runs`, `No workflows yet.`, `{count} more in n8n`, `Start n8n to see its workflows and runs.`, `Succeeded`, `Crashed`, `Waiting`, `Unknown`, `n8n isn't set up on this computer yet. Choose its folder in the desktop app.`, `The workflows n8n runs on the computer. Turning one on publishes it there. Its editor opens in Codync on the computer.`, `Workflows`, `Turning one on publishes it in n8n.`, `n8n: workflows and runs…`, `Do it with the computer's n8n workflows`, `Make an image with ComfyUI` y las de ComfyUI de la rama. La auditoría del host las revisa.
8. Deja un manifiesto de ramas y commits incorporados, conflictos resueltos y SHA integrado que vas a compilar y probar.
9. Conserva la versión 2.39.0 de la entrega y `CURRENT_PROJECT_VERSION` según las reglas de los handoffs y del checkout vigente. Si otras ramas avanzaron, evita rebajar sus versiones.

## 5. Entorno, teléfono y autorización del APK

- Teléfono indicado por el agente de paridad: **M2010J19CG, Android 12**, USB con depuración aceptada. `adb devices` debe listarlo como `67a30f561120`.
- **Construir/instalar un APK solo con el OK del dueño**, porque está usando el teléfono. Una sola instalación, la última.
- Los otros handoffs incluyen pruebas en emulador. Distingue en el reporte el teléfono físico y el emulador que realmente utilices.
- Antes de instalar la versión nueva, cierra la app Android anterior y cualquier `MainActivity` vieja de esa prueba. Coordina también el host Dev para que teléfono y host usen el protocolo correspondiente.
- El host de desarrollo descrito escucha en `19223`, usa `CODYNC_HOME=%USERPROFILE%\.codync-dev` y el token está en `%USERPROFILE%\.codync-dev\token`. En el sandbox de worktree los handoffs indican puerto desde 19300 y datos `~/.codync-dev-<carpeta>`. Comprueba el host y `binaryPath` reales antes de probar.
- Si el teléfono no ve el host, el handoff indica `adb reverse tcp:19223 tcp:19223` y la opción de red local.
- En Windows usa `npm.cmd` y `.\gradlew.bat`. Para scripts `.ps1`, el handoff proporciona `powershell -NoProfile -ExecutionPolicy Bypass -File ...`.
- No cierres Deska ni la app de otros agentes. No uses `taskkill /IM cargo.exe`. Detén únicamente los procesos que lanzaste o los que correspondan al reemplazo Dev coordinado.
- Reutiliza las conexiones, perfiles, cuentas y claves existentes. Las credenciales no se imprimen en logs ni se commitean.

### Condiciones originales del teléfono

- Cierra la app Android en el teléfono y cualquier `MainActivity` vieja.

4. Teléfono: M2010J19CG (Android 12), conectado con depuración USB aceptada. `adb devices` debe  
istarlo (`67a30f561120`). **Construir/instalar un APK solo con el OK del dueño** (él lo está  
sando). Una sola instalación, la última.

## 6. Compilación y pruebas de Android sobre el mismo HEAD

Comprueba herramientas y espacio antes de empezar. El handoff de n8n pide más de 20 GB libres y señala que Cargo necesita `-j 1` y símbolos de depuración desactivados en esta máquina. La compilación final debe contener los cambios reunidos. Evita recompilar por hito. Si corriges un fallo, repite el gate afectado y registra el nuevo HEAD probado.

### Gate Android del handoff de paridad

```powershell
cd apps\android
.\gradlew.bat :core:test :app:testDebugUnitTest :app:compileDebugKotlin
```

- Esperado: `BUILD SUCCESSFUL`.
- Entre los tests de n8n/ComfyUI están `SlashCommandsTest`, `N8nStatusTest` y los tests de ComfyUI.
- El agente n8n indicó que ese código Android todavía no se había compilado.

### APK y lint incluidos en los handoffs

```powershell
.\gradlew.bat :app:lintDebug :app:assembleDebug
```

El gate completo de Android que también figura en la entrega Hermes es el siguiente. Ejecuta desde `apps/android/`, con los requisitos JDK/SDK del repositorio.

```powershell
.\gradlew.bat :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug
```

Instalación proporcionada por los handoffs, cuando esté autorizado el dispositivo correspondiente.

```powershell
adb install -r -d app\build\outputs\apk\debug\app-debug.apk
```

Los handoffs también incluyen la forma con barras `/` para el emulador.

```bash
cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug
adb install -r -d app/build/outputs/apk/debug/app-debug.apk
```

### Host del mismo código integrado

El teléfono utiliza el host. Comprueba que el host de la campaña corresponde al SHA integrado. Los gates entregados son `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`. El handoff de paridad añade `cargo build`; el de n8n especifica `cargo test -j 1 --bin codync-host` y uso de memoria acotado. Mantén los resultados concretos de la campaña.

La prueba `claude_reads_its_own_skills_and_follows_a_symlink` puede fallar en Windows por privilegios de symlink, error 1314. Conserva su fallo y registra la causa ambiental si se confirma. El handoff de paridad proporciona `cargo test -- --skip claude_reads_its_own_skills_and_follows_a_symlink` para comprobar el resto.

Hermes se entregó con revisión de fuente, rustfmt, AST y regresiones escritas. Sus pruebas ejecutables y recorridos reales estaban pendientes. Prueba los adaptadores y APIs utilizados por Android como parte del host integrado, conservando la revisión fijada de Hermes.

### Comandos y condiciones originales de Android

### 3.2 Android

```powershell
cd apps\android
.\gradlew.bat :core:test :app:testDebugUnitTest :app:compileDebugKotlin
```

- Esperado: `BUILD SUCCESSFUL`.
- Opcional si el dueño autoriza APK: `.\gradlew.bat :app:lintDebug :app:assembleDebug` y una sola  
instalación:  
`adb install -r -d app\build\outputs\apk\debug\app-debug.apk`.
- Android: `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug`  
e instala en el emulador (`adb install -r -d app/build/outputs/apk/debug/app-debug.apk`).

Para Android, los cambios afectan a la app y al core. Sigue los requisitos JDK/SDK de AGENTS.md y ejecuta desde apps/android/ los checks correspondientes.

.\\gradlew.bat :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug

## 7. Pruebas en vivo en Android

Cambia el idioma de la app a **Español** y conserva evidencia de cada resultado.

### Hilos

1. Abre un chat con mensajes; mantenimiento largo / menú del agente → **Hilos**.
2. Abre un hilo desde el chip de un mensaje y desde la lista; responde dentro del hilo.
3. Verifica que el chat solo muestra "N respuestas" y que el punto de no leído se limpia al abrir.
4. Renombra, archiva y borra un hilo; vuelve a la lista.
5. "Branch from here" / "Branched from…" navega de vuelta al chat.

### Voz ElevenLabs

1. Ajustes → **Voz** (ajustes de voz del equipo/llamada).
2. Antes de clave: ElevenLabs muestra su formulario; **Test** sin clave muestra el error del host.
3. Con la clave del dueño: pega, **Test** debe **sonar "OK"** (audio real, no solo OK visual).
4. Lista de voces con nombres de la cuenta; sin fila **Mode** (siempre Speech); modelos `scribe_v1`/`eleven_flash_v2_5` y `scribe_v2` si aparece en la lista.
5. Inicia una llamada, habla y confirma que la respuesta se lee en la voz elegida y que el audio mp3 suena; verifica los minutos del mes.
6. Elige un proveedor **sin** clave e inicia llamada: debe caer a **On device** con el aviso.
7. Restaura el motor a **On device**. No borres la clave del dueño si la guardaste.

El handoff describe estas respuestas del host para comprobar el contrato usado por el teléfono.

- `voiceStatus` incluye `openai`, `gemini`, `elevenlabs`. ElevenLabs usa `realtime: ""`, `transcribe: "scribe_v1"`, `speech: "eleven_flash_v2_5"`.
- `voiceVoices` sin clave devuelve `"No ElevenLabs key on this computer. Add one in the call settings."`.
- `voiceSession` para ElevenLabs devuelve `"ElevenLabs doesn't hold live calls. Choose Speech mode."`.
- Con clave válida, `voiceVoices` devuelve `{voices:[{id,name},...]}` de la cuenta y `voiceModels` devuelve modelos TTS y Scribe v1/v2 para transcribir.
- `voiceSpeak`, con `model='eleven_flash_v2_5'`, una voz de la cuenta y `text='OK'`, devuelve `{audio:"<base64 mp3>", format:"mp3"}`. Escucha el resultado real.
- `setVoiceKey` acepta una clave sólo si el proveedor la valida. Una clave inválida no debe guardarse. Ese método es sólo loopback, por lo que la preparación de la clave se hace en la computadora.

### Canales, empresa y rutinas

- Extensiones → Canales: cuatro filas; abrir hoja; notas y rutinas por canal; miembros de empresa.
- Abrir Telegram, Slack y Discord con las cuentas disponibles.
- En un canal de prueba con bot asignado, añadir y borrar una nota, crear una rutina del canal y verla listada y activar/desactivar en Rutinas.
- Ajustes → General → Canales, comprobar controles y navegación a Extensiones.
- Configurar los canales desde el teléfono y comprobar que los tokens se ocultan al escribir.
- Emparejar WhatsApp por código, probar rutinas desde WhatsApp y salida a WhatsApp con el destino de prueba identificado.
- Comprobar cancelación de tarea y modelo seleccionado por el enlace del flujo.

### Details del bot, archivos, fotos y arranque

- Bot → Details: Overview con agente, modelo, carpeta, permisos, skills, conectores y tokens de hoy.
- Tareas con aprobaciones, rutinas e hilos.
- Archivos: navegar y copiar ruta.
- Subir una foto PNG/JPG menor de 300 KB, verla en roster y chat y quitarla.
- Duplicar desde el menú del bot.
- Guía de primera vez tras emparejar, cierre con "Ahora no" y estados de Primeros pasos.
- La guía no reaparece al reabrir.
- Ajustes: Diagnóstico no existe en Android, según el handoff.

### n8n, ComfyUI y comandos

- Ajustes → ComfyUI y n8n, con estado, Start/Stop, lista y ejecuciones.
- ComfyUI descarga aproximadamente 2,5 GB en el primer uso. Comprueba avance, descompresión y estado del ejecutor compartido. Sin modelo, registra que no se probó una generación real.
- n8n requiere la instalación indicada en `C:\Users\Edwin Villeda\Documents\automan`, con `packages/cli/bin/automan`, y Node suficiente.
- Publicar y despublicar un workflow desde el teléfono debe cambiar su estado real en n8n. Las llamadas `/api/v1` con `X-AUTOMAN-API-KEY` estaban pendientes de prueba contra n8n real.
- Prueba el nodo **Codync Bot**, con "Send a message" y espera de respuesta, cuando esté disponible en el host integrado.
- Escribe `/` en el compositor y verifica que `/n8n` y `/comfy` aparecen después de las skills.
- `/n8n lista mis workflows` utiliza el conector. Con n8n parado, el bot debe comunicarlo.
- `/comfy un zorro rojo` utiliza las herramientas o comunica que ComfyUI no está corriendo.
- Un dispositivo remoto `Caller::Device`, como el teléfono emparejado, recibe `n8nStatus` sin `auth` ni cookie y no puede llamar a las acciones incluidas en `LOCAL_ONLY`.

### Hermes utilizado desde el teléfono

1. Conversa con un bot Hermes, ejecuta una herramienta, aprueba y deniega una acción y responde una aclaración con opciones incluso en modo automático. Comprueba Stop y cancelación real.
2. Importa una habilidad del catálogo, úsala, registra una mejora y restaura una versión. Comprueba aislamiento entre bots y rechazo de revisiones obsoletas.
3. Guarda memoria, reabre sesión y consulta lo hecho en un intervalo de fechas. Comprueba persistencia y Honcho con el perfil existente cuando esté disponible.
4. Configura Signal y Email mediante Apps, usa el código inicial de pairing y la política de remitentes y comprueba aprobación por autor, notas de voz y adjuntos. Usa Home Assistant con entidades de prueba seleccionadas. Las acciones adicionales de revocación individual están expuestas por API.
5. Crea una rutina desde un blueprint y edita sus destinos. Comprueba WhatsApp, indisponibilidad temporal, recibos, reconexión, idempotencia y cancelación. Verifica conflicto de cambios simultáneos de destinos.
6. En la tarjeta Hermes guarda conjuntamente entorno, memoria y delegación. Prueba Check saved environment sobre la revisión guardada y la protección de cambios simultáneos. La comprobación puede crear un sandbox.
7. Guarda rutas de proveedor/modelo y comprueba el resumen de los hijos y los límites compartidos del host entre conversaciones y rutinas.
8. Selecciona NeuTTS para salida conservando STT, confirma audio progresivo e interrupción al hablar. Comprueba cancelación anticipada y cierre. Prueba Hey Hermes, su opt-in, permisos, mute y salida a segundo plano.
9. El catálogo MCP debe conservar origen, estado, conexiones existentes y deduplicación con OpenBot. Verifica los flujos de conexión disponibles.

### Regresión Android

- Mensaje con reacción/respuesta, marcar no leído y leer en voz alta a distintas velocidades.
- Adjuntar imagen, pila de imágenes, visor paginado y formatos SVG/MP4.
- Cola con Editar/Steer/Borrar y pines.
- Compartir desde otra app.
- Bloqueo con PIN/biometría y espejo cifrado.
- Support, guardar/limpiar log y comprobar redacción de datos sensibles.
- Reconexión, foreground y timeouts.
- Idioma árabe/RTL y dirección por párrafo.
- Widgets Plush/Flat, caras en reposo, flotación, split plegable, live updates/push y cara en notificaciones, según las capacidades disponibles.
- Apps con estado y paginación, servicios incorporados y ventana de uso gastado bajo el modelo.

### Recorridos originales de Android

### 4.2 Teléfono (Android, en español)

Conecta la app de desarrollo a este equipo; si no ve el host, `adb reverse tcp:19223 tcp:19223`  
y usa la opción de red local. Cambia el idioma de la app a **Español** y recorre:

**Hilos (M2):**

1. Abre un chat con mensajes; mantenimiento largo / menú del agente → **Hilos**.
2. Abre un hilo desde el chip de un mensaje y desde la lista; responde dentro del hilo.
3. Verifica que el chat solo muestra "N respuestas" y que el punto de no leído se limpia al abrir.
4. Renombra, archiva y borra un hilo; vuelve a la lista.
5. "Branch from here" / "Branched from…" navega de vuelta al chat.

**Voz con ElevenLabs (M4):**

1. Ajustes → **Voz** (ajustes de voz del equipo/llamada).
2. Antes de clave: ElevenLabs muestra su formulario; **Test** sin clave muestra el error del host.
3. Con la clave del dueño: pega, **Test** debe **sonar "OK"** (audio real, no solo OK visual).
4. Lista de voces con nombres de la cuenta; sin fila **Mode** (siempre Speech); modelos  
scribe\_v1`/`eleven\_flash\_v2\_5`(y`scribe\_v2\` si aparece en la lista).
5. Inicia una llamada, habla y confirma que la respuesta se lee en la voz elegida y que el audio  
p3 suena; verifica los minutos del mes.
6. Elige un proveedor **sin** clave e inicia llamada: debe caer a **On device** con el aviso.
7. Restaura: motor de vuelta a **On device**; **no borres** la clave del dueño si la guardaste.

**Regresión rápida (5 min):** un mensaje con reacción/respuesta, adjuntar imagen, cola con  
Editar/Steer/Borrar, compartir desde otra app, bloqueo con PIN, y (si el tiempo da) una pantalla  
en árabe/RTL.

ANDROID (mismo orden que puedas)

- Extensiones → Canales: cuatro filas; abrir hoja; notas y rutinas por canal; miembros de empresa.
- Bot → Details: Overview; Tareas (tokens de hoy, aprobaciones, rutinas, hilos); Archivos (navegar,  
copiar ruta); foto visible en roster y chat; Duplicar en el menú.
- Guía de primera vez tras emparejar; no reaparece al reabrir.
- Ajustes: Diagnóstico no existe en Android (no es un fallo).

7. **Android** (emulador con `adb`): Ajustes → ComfyUI y n8n (estado, Start/Stop, lista y ejecuciones) y los comandos `/n8n`/`/comfy` en el compositor.

## 8. Reglas, límites y pendientes de los handoffs

- No añadas funciones nuevas salvo para corregir lo que falle durante la integración o validación. Conserva los cambios de los demás agentes y un cambio lógico por commit.
- No commitees el scratch `tools/*` del worktree de paridad.
- El handoff de paridad señaló un bug en `host/src/store/entries.rs`, `last_health_notice`, que pasa `?3` a una consulta de dos parámetros, y pidió reportarlo como cambio aparte.
- `MIN_APP` seguía en 2.21.0 por una regresión previa del marketplace. El handoff indicó no cambiarlo en esa rama. Un iPhone anterior a 2.39 con enum estricto de proveedor no se arregla retroactivamente.
- Gap deliberado indicado por paridad: sin modelos neuronales on-device Supertonic 3/Parakeet v3. ElevenLabs usa Scribe y TTS, siempre Speech.
- El handoff del principal indica fallback deliberado al inglés para fr/de/pt-BR/ru/ja/ko/zh-Hans/zh-Hant. El de n8n exige sus claves nuevas en diez catálogos. Conserva la procedencia y resultados de ambas comprobaciones.
- No se considera un fallo del canal la ausencia de una cuenta real de Discord/Telegram/Slack. Registra el recorrido pendiente con su motivo.
- NeuTTS requiere Python 3.11–3.13 y sus modelos preparados. Hermes admite Python 3.11–3.14.
- Hey Hermes usa reconocimiento del sistema móvil y primer plano. La adaptación no incluye el runtime móvil TFLite de Hermes.
- La salud de entornos puede crear recursos con costes nativos y comunica limpieza no confirmada al agotar plazo. El máximo configurado de hijos es diez y los límites de inactividad, iteraciones y dinero tienen semánticas diferentes.
- Reutiliza cuentas y destinos de prueba ya identificados del usuario. Si falta un dispositivo, cuenta, modelo o destino, registra el bloqueo y continúa con los recorridos independientes.
- Si el teléfono no aparece o falta una capacidad para probarlo, no inventes el resultado. Márcalo como no ejecutado con la razón exacta.

### Referencia original del inventario Android

- `docs/plans/openbot-android-inventory.md` está desactualizado (foto previa); el registro bueno  
es `openbot-mobile-parity.md`.

## 9. Reporte final de Android

Entrega el informe en español con lo siguiente.

1. Worktree, rama, HEAD integrado y lista de ramas/commits incorporados.
2. Qué trabajo de Android procede de cada entrega y qué conflictos resolviste.
3. Comandos Gradle ejecutados, resultado de cada gate y pruebas pasadas, fallidas y omitidas. Distingue la entrega antigua de los resultados de esta campaña.
4. Ubicación, versión, arquitectura y checksum del APK final, si se autorizó construirlo. Identifica el dispositivo y si se autorizó instalarlo.
5. SHA, ruta y versión del host con el que se probó el teléfono y correspondencia con el código integrado.
6. Resultado de hilos, ElevenLabs, fallback On device, canales, empresa, rutinas, Details, archivos, fotos, onboarding, n8n, ComfyUI, comandos, Hermes y regresiones.
7. Por recorrido, esperado, obtenido, captura y auditoría cuando corresponda. Marca pasado, fallado o no ejecutado y explica el motivo.
8. Correcciones hechas, commits y gate repetido, o "ninguno".
9. Servicios, dispositivos, modelos y pruebas que quedaron pendientes y su impacto concreto.

Si cambias código después de compilar, prueba la versión corregida y registra su HEAD. Conserva logs redactados y evidencia del APK y host realmente usados. El resultado del trabajo debe poder revisarse como una sola entrega Android sobre el mismo código integrado.

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

