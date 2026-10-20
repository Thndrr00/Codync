# Revisión de documentación: Android propio y complemento con Windows

Fecha: 2026-10-19. Estado: análisis de fuentes y propuestas, sin cambios de código.

## Resultado

Sí hay oportunidades adicionales. La prioridad es construir una identidad de
interfaz Android sobre los componentes Codync existentes, y después incorporar
funciones compartidas con escritorio mediante el host. Cambiar solo de paleta
conservaría la estructura de iPhone que hoy define buena parte de la experiencia.

La propuesta se divide en tres documentos:

1. Este informe: alcance revisado, hallazgos, solapamientos y orden de trabajo.
2. [Especificación del sistema visual Android](especificacion-identidad-visual-android.md):
   cómo cambiar tema, materiales, componentes y pantallas sin rehacer el producto.
3. [Oportunidades adicionales Android y Windows](oportunidades-adicionales-android-windows.md):
   veinte fichas de ampliaciones, pendientes documentados y propuestas nuevas,
   con viabilidad, integración y aceptación.

No son setenta funciones nuevas: el catálogo anterior contiene cincuenta ideas
y las veinte fichas adicionales incluyen ampliaciones y pendientes que ya
aparecen en otras guías. Cada ficha declara su relación con lo existente.

## Alcance y límites de esta revisión

Se inventariaron los **56 archivos Markdown presentes en `docs/` al iniciar la
revisión**, sus títulos/secciones y referencias a implementación, planes y
límites. Se profundizó en los documentos relevantes para Android, UI, host,
escritorio, archivos, colaboración, conexión y capacidades Windows; no se hizo
una lectura línea por línea de todos los protocolos, investigaciones o archivos
históricos. También se contrastaron puntos concretos con `apps/android/README.md`
y código de Android, Electron y Rust.

Esto es una revisión documental y de código, no una auditoría visual ejecutada:
no se abrió ni instaló una app, no se midieron frames, no se probaron dispositivos
y no se verificaron servicios desplegados. “Existe código” no significa
“funciona en producción”. Una búsqueda sin coincidencias tampoco demuestra
ausencia completa de una capacidad.

Los documentos de `archive/` y las investigaciones fechadas se trataron como
antecedentes, nunca como evidencia de un error todavía reproducible. El código
inspeccionado orienta el plan, pero su aceptación requiere pruebas actuales.

## Hallazgos confirmados en las fuentes

### 1. El parecido con iOS es una decisión explícita

`apps/android/README.md`, `docs/plans/android-app.md` y
`docs/design/ui-conventions.md` describen Android como un port de las pantallas
del iPhone, con componentes propios que reproducen cristal, hojas y menús.
No es solo una impresión estética del usuario.

En el código, `design/Palette.kt` usa pares de colores y `CodyncTheme.kt` obtiene
el modo oscuro del sistema. Además, la paleta propia y MaterialTheme se definen
en sitios distintos, y `botMessageBackground` consulta directamente el tema
del sistema. Introducir modo manual sin resolver esas rutas puede producir
pantallas mezcladas. Cambiar únicamente MaterialTheme no recolorea todos los
controles Codync.

La barra actual de `MainTabs.kt` tiene Bots/State. El cambio deseado requiere
una revisión de tokens, superficies, navegación, formularios y estados; la
especificación enlazada propone una migración por pantallas.

### 2. Hay infraestructura que conviene ampliar

- Android ya tiene Lista/Constelación, creación de bots y grupos, permisos,
  rutinas, archivos, voz y widgets; no hace falta recrear sus stores.
- `AppStoreChat.kt` tiene envíos persistidos, reintentos y nonce. Una bandeja
  de pendientes debe presentar esa cola, no mantener una cola paralela.
- `AlertNotification.kt` publica atajos de conversación con
  `ShortcutManagerCompat.pushDynamicShortcut`. La idea 38 del catálogo debe
  completar fijados/gestión, no afirmar que Android carece de atajos.
- Memoria Android ya contiene capas y versiones en los archivos inspeccionados.
  Hay que validar flujos y compatibilidad antes de decidir qué falta.
- La documentación de archivos incluye snapshots e integridad para resultados.
  Una biblioteca de entregables reutilizaría esas referencias.
- La colaboración entre bots ya tiene datos estructurados de emisor, receptor,
  petición, respuesta y estado. Es una base mejor para un mapa de colaboración
  que interpretar mensajes o conectar todas las estrellas arbitrariamente.

### 3. Hay divergencias documentales que afectan la planificación

- `docs/plans/android-app.md` y el inicio de `docs/design/bot-memory.md` todavía
  describen memoria Android como un port pendiente de métodos retirados; los
  módulos actuales inspeccionados ya incluyen el modelo nuevo. Reconciliar con
  pruebas, sin marcar todo completado solo por existencia de clases.
- `docs/architecture/desktop-app.md` dice que Windows no tiene computer MCP.
  `docs/features/computer-use.md` y `host/src/screen/cua.rs` sí contemplan driver
  en Windows. Separar ese soporte del visor remoto, que requiere otro helper.
- `docs/features/file-attachments.md` conserva exclusiones de UI Android/grupos
  que contrastan con cobertura declarada en el plan y README Android. Antes de
  una ampliación, verificar entrada/salida y contexto por separado.
- Las investigaciones Windows sobre npm o memoria son fotografías de una
  versión. No planificar arreglos como si todos siguieran presentes.
- La guía anterior de cincuenta ideas aconseja tolerar streaming de bloques;
  `docs/features/chat-messages.md` establece que las burbujas muestran entradas
  finales. Optimizar contenido largo no requiere cambiar esa política.

Este trabajo registra divergencias; no modifica retrospectivamente documentos
de aceptación ni inventa resultados para dar por resueltas sus listas.

### 4. Windows necesita una matriz de capacidades

La app Electron es un único renderer para macOS, Linux y Windows. Los cambios
comunes deben entrar allí; integración con SO pasa por main/preload o por el
host, sin bifurcar pantallas completas.

La revisión distingue:

- **Chat, bots y herramientas del host:** base común a extender y validar.
- **Computer use por bots:** driver contemplado en Windows; la guía indica
  límites sobre ventanas elevadas. No equivale a controlar el escritorio seguro.
- **Ver la PC desde Android:** falta helper de pantalla Windows según las
  fuentes actuales. Es una capacidad nueva de plataforma, no un rediseño del visor.
- **SSH en escritorio Windows:** la arquitectura documenta dependencia de rutas
  y comprobaciones Unix, con backend Windows pendiente.
- **Aprobaciones e island:** ya hay superficies de atención documentadas en
  escritorio. Una bandeja nueva debe integrarlas en vez de reemplazarlas todas.

## Qué ya estaba propuesto y no debe duplicarse

- Tema dinámico/manual/OLED, superficies ligeras, cabeceras, navbar y formularios:
  ideas 1–16; consolidarlas en un sistema visual único.
- Búsqueda, filtros, agrupación, atención y stop: 17–24; coordinarlas con
  Constelación sin construir otro roster aislado.
- Código, diff, compositor, hilo e inspector: 25–34; reutilizar renderizadores.
- Memoria, rutinas, compartir, atajos y notificaciones: 35–40; varias bases existen.
- Voz, macros, lupa y PiP: 41–44; dependen de ciclo de vida y capacidades reales.
- Git, handoff, telemetría, web preview, transferencia y prompts: 45–50;
  conservar un contrato compartido por capacidad.
- Constelaciones por grupo, avatares y widget de bloqueo tienen planes propios.
  La constelación representa miembros; un mapa de colaboración representa
  intercambios reales y puede ser una capa distinta, expresamente identificada.
- Memoria propia del harness y reporte de problemas ya están en `docs/plans/`.
  Incorporarlos a la hoja de ruta como pendientes existentes, no como novedades.

## Recomendación de entrega

1. **Primero, identidad Android:** resolver tema y tokens, luego controles y
   navegación, después chat, formularios y extensiones. Prioridad sobre añadir
   veinte botones nuevos. Validar una sección completa antes de migrar el resto.
2. **Después, confianza y recuperación:** cola visible, estados de conexión,
   diagnóstico de capacidades y errores accionables. Ayudan tanto en móvil
   como cuando Windows actúa de host.
3. **Luego, complementar dispositivos:** entregables, marcadores, decisiones
   pendientes y mapa real de colaboración, con los contratos existentes donde
   sea posible y adiciones al host donde hagan falta.
4. **Proyectos de plataforma separados:** helper de pantalla Windows y SSH
   Windows. No bloquear el nuevo tema Android por trabajos de media/input.
5. **Más tarde y previa decisión de producto:** múltiples equipos simultáneos,
   coordinación de espacios y resultados estructurados. La arquitectura actual
   prioriza un equipo; este informe no cambia esa decisión automáticamente.

Cada fase requiere alcance acotado, checks, e2e y la paridad exigida por el
repositorio. Los cambios de ergonomía se adaptan por plataforma; las funciones,
estados y autorizaciones se mantienen coherentes en iOS, escritorio y terminal.

## Evidencia de diseño externa

La dirección propuesta es una inferencia de las fuentes locales y de la necesidad
del usuario, no una conclusión de pruebas de usabilidad. Google trata el tema
como color, tipografía y forma y admite sistemas propios; no exige reemplazar
todos los controles con widgets de una biblioteca.
[Android: Themes](https://developer.android.com/design/ui/mobile/guides/styles/themes).

En Windows conviene tratar contraste y preferencias del sistema como parte del
tema; Electron expone señales de modo oscuro y alto contraste. No se asume
que APIs de WinUI puedan copiarse directamente al renderer web de Electron.
[Electron nativeTheme](https://www.electronjs.org/docs/latest/api/native-theme),
[Microsoft: Contrast themes](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/high-contrast-themes).

## Inventario documental inicial

La siguiente lista permite reproducir el alcance. Sus rutas son relativas a
`docs/`; no indica que todas sus afirmaciones hayan sido probadas.

- Raíz: `README.md`.
- `architecture/`: `conversation-runtime.md`, `desktop-app.md`,
  `file-structure.md`, `overview.md`.
- `archive/`: `README.md`, `bot-computer-ssh-plan-2026-09-25.md`,
  `cloudflare-account-device-plan-2026-09-25.md`,
  `macos-chat-swiftui-2026-10-05.md`,
  `mobile-widgets-verification-2026-09-26.md`, `ui-audit-2026-09-25.md`.
- `design/`: `bot-memory-research.md`, `bot-memory.md`, `mobile-widgets.md`,
  `push-and-live-activity.md`, `ui-conventions.md`.
- `features/`: `analytics.md`, `bot-collaboration.md`, `bot-workspaces.md`,
  `chat-messages.md`, `computer-order.md`, `computer-use.md`,
  `connector-credentials.md`, `context-and-memory.md`, `file-attachments.md`,
  `groups-and-threads.md`, `marketplace.md`, `notch-island.md`,
  `remote-screen.md`, `routines.md`, `voice-call.md`.
- `guides/`: `accounts-and-ssh.md`, `clean-code.md`, `cloudflare-testing.md`,
  `composer.md`, `connector-verification.md`, `desktop-styles.md`, `dev-app.md`,
  `development.md`, `environments-and-deployment.md`, `linux-servers.md`,
  `updates.md`, `windows-testing.md`.
- `investigations/`: `posthog-errors-2026-10-19.md`,
  `windows-npm-agents-2026-10-14.md`.
- `nuevo thndr/`: `guia-implementacion-50-mejoras-android-escritorio.md`,
  `plan de mejora andorid+escritorio.md`, `plan-avatares-bots.md`,
  `plan-constelaciones-interactivas-por-grupo.md`, `plan-widget-bloqueo-android.md`.
- `plans/`: `android-app.md`, `error-tracking-detail.md`, `harness-memory.md`.
- `reference/`: `compatibility.md`, `remote-relay.md`, `fixtures/README.md`.

## Archivos de código contrastados

- Android design: `Palette.kt`, `CodyncTheme.kt`, `Chrome.kt`, `Sheet.kt`,
  `ConstellationOrb.kt`; app: `MainTabs.kt`, `BotListView.kt`,
  `BotConstellation.kt`, `ConstellationLayout.kt`, `AppStoreChat.kt`,
  `AlertNotification.kt`, `MemoryCard.kt`, `MemoryModel.kt`, `RoutinesScreen.kt`,
  `MarkdownText.kt`, `VoiceControls.kt`; core: `Routine.kt`, `Memory.kt`,
  `MarkdownBlocks.kt`, `Screen.kt`.
- Escritorio: `src/renderer/styles/theme.css`, `views/BotRow.tsx`,
  `src/main/screen.ts`; referencias adicionales en la arquitectura y catálogo.
- Host: `src/screen/mod.rs`, `src/screen/cua.rs`, `src/api/files.rs`,
  `src/api/remote.rs`, `src/hub.rs`, `src/service/mod.rs`.

Los nombres cortos Android pertenecen a los paquetes
`com/codync/android` y `com/codync/android/design` de sus módulos respectivos.

## Siguiente tarea concreta recomendada

Implementar solo la base del tema Android: resolución única de apariencia,
preferencia de dispositivo y migración de una pantalla piloto, con fallback y
pruebas. Esta recomendación no inicia la implementación. Consultar el contrato
visual propuesto y decidir sus detalles antes de editar componentes distribuidos.
