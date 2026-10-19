# Plan de Mejora Android + Escritorio (Thndrr00/Codync)

Este documento reúne el catálogo completo de **50 mejoras de UI/UX y utilidades** para el fork de Codync (`origin = Thndrr00/Codync`), enfocado en evolucionar `apps/android` (Kotlin + Jetpack Compose) hacia una experiencia nativa Android moderna y ergonómica, junto con las integraciones que conectan `apps/android`, `apps/desktop` y `host`.

> [!IMPORTANT]
> **Regla del Fork**: Todo el trabajo pertenece exclusivamente al fork `Thndrr00/Codync`. Nunca hacer push ni abrir PR hacia `leepokai/Codync` (`upstream`).

---

## Diagnóstico de la Arquitectura Actual

1. **Barra de navegación inferior estilo iPhone (`GlassTabBar`)**:
   - Actualmente definida en `apps/android/design/src/main/kotlin/com/codync/android/design/Chrome.kt` y montada en `apps/android/app/src/main/kotlin/com/codync/android/MainTabs.kt`.
   - Es una cápsula flotante con efecto Liquid Glass de solo 2 pestañas (`Bots` y `State`, cada una de `90.dp`), obligando a abrir hojas modales (`CodyncSheet`) o menús superiores para acceder a **Extensiones (Marketplace)**, **Computadoras y Ajustes**, **Uso** y **Cuenta**.
2. **Controles y navegación clonados 1:1 de iOS 26**:
   - Uso de `chevron.left` de `32.dp` encerrado en burbuja de cristal, títulos centrados en `ChatScreen.kt`, botón `+` en la esquina superior derecha (`NewMenuButton` en `BotListView.kt`) difícil de alcanzar con una mano, y hojas modales (`Sheet.kt`) siempre a altura completa con radio `40.dp`.
3. **Oportunidades de integración Móvil + Escritorio + Host**:
   - El daemon en Rust (`host/`) y la app de escritorio (`apps/desktop/`) ya gestionan sesiones ACP, archivos, memoria, rutinas, canal encriptado E2E y pantalla remota WebRTC, lo que permite exponer nuevas utilidades cruzadas para controlar y supervisar el flujo de desarrollo desde el móvil.

---

## Catálogo de 50 Ideas de UI/UX y Utilidades

Cada idea especifica si su alcance es **[Solo Móvil]** (`apps/android`) o **[Móvil + Escritorio/Host]** (`apps/android` + `apps/desktop` + `host`).

### Categoría 1: Rediseño de la Barra de Navegación Inferior y Navegación Principal (Ideas 1–8)

1. **[Solo Móvil] Nueva Barra de Navegación Inferior estilo Android Moderno (`CodyncBottomBar`)**
   - **Mejora**: Reemplazar la cápsula flotante de 2 pestañas (`GlassTabBar`) por una barra de navegación inferior de ancho completo o dock ergonómico estilo Material 3 Expressive con **4 destinos directos**: **Chats** (Bots), **Actividad** (Aprobaciones/Rutinas/Uso), **Extensiones** (Marketplace directo sin entrar a Ajustes) y **Ajustes/PC** (Computadoras y Cuenta).
   - **Archivos**: `apps/android/design/.../Chrome.kt`, `apps/android/app/.../MainTabs.kt`, `apps/android/app/.../CodyncApp.kt`.

2. **[Solo Móvil] Indicadores y Badges Dinámicos en la Navbar Inferior**
   - **Mejora**: Mostrar en tiempo real sobre los iconos de la navbar: conteo de mensajes no leídos en *Chats*, punto ámbar pulsante cuando un bot está en `needsInput` (requiere aprobación) o contador de tareas activas (`working`), y estado online/offline en la pestaña *Ajustes/PC*.
   - **Archivos**: `MainTabs.kt`, `Chrome.kt`.

3. **[Solo Móvil] Botón de Acción Flotante (FAB) Contextual en lugar del `+` superior de iOS**
   - **Mejora**: Mover la creación de *Nuevo Bot / Nuevo Grupo* (`NewMenuButton`) a un FAB expandible en la esquina inferior derecha sobre la navbar, que se contrae a solo icono al hacer scroll hacia abajo.
   - **Archivos**: `BotListView.kt`, `Controls.kt`.

4. **[Solo Móvil] Navegación por Gestos Horizontales (`HorizontalPager`) y Re-tap para subir al inicio**
   - **Mejora**: Permitir deslizar lateralmente entre las pestañas principales de `MainTabs.kt` con transición suave de indicador activo, y que al tocar dos veces la pestaña activa la lista haga scroll automático hasta arriba (`scrollToItem(0)`).
   - **Archivos**: `MainTabs.kt`, `BotListView.kt`.

5. **[Solo Móvil] Ocultación Inteligente de la Navbar al hacer Scroll (`Auto-hide on Scroll`)**
   - **Mejora**: Conectar un `NestedScrollConnection` en `MainTabs.kt` para que la barra inferior se deslice suavemente hacia abajo cuando el usuario explora listas largas y reaparezca al deslizar hacia arriba.
   - **Archivos**: `MainTabs.kt`.

6. **[Solo Móvil] Mini-Barra Persistente de Llamada de Voz y Pantalla Remota sobre la Navbar**
   - **Mejora**: Cuando hay una llamada de voz activa (`state.voice != null`) y el usuario sale del chat hacia el roster u otra pestaña, mostrar una mini-barra acoplada justo encima de la navbar inferior con el avatar del bot hablando, onda de audio, botón de silenciar y colgar.
   - **Archivos**: `MainTabs.kt`, `ChatScreen.kt`.

7. **[Solo Móvil] Modo `NavigationRail` Lateral y Vista Dividida (Master-Detail) para Plegables/Tablets/Horizontal**
   - **Mejora**: Si el ancho de ventana supera `600.dp` (teléfonos en horizontal, plegables abiertos o tablets), transformar automáticamente la navbar inferior en un `NavigationRail` vertical a la izquierda y mostrar el roster de bots + la conversación abierta lado a lado (como en escritorio).
   - **Archivos**: `MainTabs.kt`, `CodyncApp.kt`.

8. **[Solo Móvil] Selector de Estilo de Barra Inferior en Ajustes**
   - **Mejora**: Añadir una opción en Ajustes de Apariencia para elegir entre 3 estilos de barra inferior: *Barra Anclada Android (M3 Surface)*, *Dock Flotante Compacto (4 iconos)* o *Clásico Cápsula*.
   - **Archivos**: `SettingsView.kt`, `Chrome.kt`.

---

### Categoría 2: Identidad Visual Android, Temas y Personalización UI (Ideas 9–16)

9. **[Solo Móvil] Soporte de Color Dinámico (Material You / Monet) + Modo AMOLED Pitch Black**
   - **Mejora**: Ampliar `Palette.kt` para soportar extracción de color dinámico de Android 12+ (`dynamicDarkColorScheme` / `dynamicLightColorScheme`), selección de color de acento global y modo **AMOLED Negro Puro (`#000000`)**.
   - **Archivos**: `Palette.kt`, `Theme.kt`.

10. **[Solo Móvil] Tinte Ambiental por Bot en la Conversación (`Bot Accent Theme`)**
    - **Mejora**: Aprovechar el `avatarColor` de cada bot (`blue`, `orange`, `violet`, `green`, `magenta`) para teñir sutilmente el botón de envío, los bordes del compositor, los enlaces y las burbujas del usuario dentro de `ChatScreen.kt`.
    - **Archivos**: `ChatScreen.kt`, `ChatRows.kt`, `MessageComposer.kt`.

11. **[Solo Móvil] Rediseño de Cabeceras (`ThreadHeader` y `ScreenHeader`) sin estilo iOS**
    - **Mejora**: En `ChatScreen.kt`, cambiar el botón redondo de cristal con `chevron.left` gigante y título centrado por una barra superior estilo Android: flecha atrás nativa, avatar + nombre + subtítulo de ruta alineados a la izquierda (tocables para abrir el inspector del bot), y botones de acción rápida a la derecha (*Pantalla remota*, *Routines*, *Menú `⋮`*).
    - **Archivos**: `ChatScreen.kt`, `Chrome.kt`.

12. **[Solo Móvil] Superficies Tonales Ligeras (Modo Rendimiento 120 Hz)**
    - **Mejora**: Ofrecer un estilo de superficie tonal Android con borde sutil de `1.dp` (en lugar del shader `lens` + `blur` de `Glass.kt` que imita iOS 26), reduciendo carga de GPU en scroll rápido.
    - **Archivos**: `Glass.kt`, `Palette.kt`.

13. **[Solo Móvil] Control de Tamaño de Fuente y Densidad de Interfaz en Móvil**
    - **Mejora**: Igualar la opción de tamaño de texto que tiene `apps/desktop` (`stepTextSize`), agregando en Ajustes de Android un control de escala de texto del chat (85% a 130%) y densidad de filas (*Estándar* vs *Compacta*).
    - **Archivos**: `Type.kt`, `SettingsView.kt`.

14. **[Solo Móvil] Retroalimentación Háptica Nativa (`HapticFeedback`) en toda la App**
    - **Mejora**: Integrar micro-vibraciones nativas al cambiar de pestaña en la navbar, arrastrar bots en `RosterReorder`, enviar mensajes, reaccionar con emojis y recibir solicitudes de permiso.
    - **Archivos**: `Controls.kt`, `BotListView.kt`, `MessageComposer.kt`.

15. **[Solo Móvil] Hojas Modales (`CodyncSheet`) con Detents (Media Altura / Pantalla Completa) y Predictive Back**
    - **Mejora**: Ajustar `Sheet.kt` con radios de `28.dp` estilo Android, soporte para abrir a media altura (*Half-expanded*) en vistas cortas (Cuenta, Filtros, Rutinas) y animación *Predictive Back* progresiva al hacer el gesto de volver de Android 14+.
    - **Archivos**: `Sheet.kt`.

16. **[Solo Móvil] Selector Manual de Tema (Sistema / Claro / Oscuro / Negro OLED)**
    - **Mejora**: Permitir elegir el tema directamente dentro de `SettingsView.kt` sin depender solo del ajuste global del sistema.
    - **Archivos**: `SettingsView.kt`, `Palette.kt`.

---

### Categoría 3: Roster de Bots, Búsqueda y Gestión Rápida (Ideas 17–24)

17. **[Solo Móvil] Barra de Búsqueda Global en el Roster (Bots + Mensajes)**
    - **Mejora**: Añadir un buscador en `BotListView.kt` (equivalente móvil de `SearchPalette.tsx` de escritorio) para filtrar bots por nombre, carpeta de proyecto (`cwd`), agente o buscar texto dentro del historial local en `MirrorDatabase`.
    - **Archivos**: `BotListView.kt`, `MirrorDatabase.kt`.

18. **[Solo Móvil] Chips de Filtro Rápido sobre la Lista de Bots**
    - **Mejora**: Añadir una fila horizontal de chips debajo del buscador: **Todos**, **Necesitan tu acción (`needsInput`)**, **Trabajando (`working`)**, **No leídos**, **Grupos** y **Por proyecto**.
    - **Archivos**: `BotListView.kt`.

19. **[Solo Móvil] Gestos Swipe-to-Action en las Filas de Bots (`BotRowItem`)**
    - **Mejora**: Permitir deslizar un bot hacia la derecha para **Fijar / Marcar como leído**, y hacia la izquierda para **Ocultar / Detener ejecución (`stop`)**, con iconos animados y háptica.
    - **Archivos**: `BotListView.kt`.

20. **[Solo Móvil] Tarjeta Resumen Superior "Necesitan tu Atención" (`Action Required Banner`)**
    - **Mejora**: Cuando uno o más bots están en `status == "needsInput"`, mostrar arriba del roster una tarjeta interactiva que permita ver el permiso solicitado y **Aprobar / Rechazar con un toque** sin entrar al chat.
    - **Archivos**: `BotListView.kt`, `ChatRows.kt`.

21. **[Solo Móvil] Badges de Metadatos en `BotRow` (Agente, Carpeta y Modo Rápido)**
    - **Mejora**: Mostrar en cada fila del roster pequeños pills con el icono del agente (`Claude`, `Codex`, `Gemini`), el nombre de la carpeta del proyecto (`cwd`) y un indicador si tiene `fastMode` activo.
    - **Archivos**: `BotListView.kt`.

22. **[Solo Móvil] Agrupación Plegable por Carpeta de Proyecto (`Workspace Grouping`)**
    - **Mejora**: Opción para agrupar los bots en el roster según el directorio de proyecto (`bot.cwd`) en secciones plegables.
    - **Archivos**: `BotListView.kt`.

23. **[Solo Móvil] Cronómetro en Vivo y Botón Rápido de `Stop` en Filas de Bots Trabajando**
    - **Mejora**: Cuando un bot tiene `status == "working"`, mostrar en `Preview` el tiempo transcurrido en vivo (`1:24`) y un botón rápido para cancelar la ejecución desde la lista.
    - **Archivos**: `BotListView.kt`.

24. **[Solo Móvil] Vista Previa Rápida (`Peek Sheet`) al Mantener Presionado un Bot**
    - **Mejora**: En el menú contextual de `BotRowItem`, mostrar una mini-vista previa con los últimos 3 mensajes de ese bot para leer rápido sin salir del roster.
    - **Archivos**: `BotListView.kt`, `Menus.kt`.

---

### Categoría 4: Experiencia de Chat, Compositor y Bloques de Código (Ideas 25–34)

25. **[Solo Móvil] Visor de Código Mejorado con Resaltado de Sintaxis, Copia por Bloque y Envoltura de Línea**
    - **Mejora**: Mejorar los bloques de código en el chat con encabezado de lenguaje (`kotlin`, `rust`, `typescript`, `bash`), botón de copiar siempre visible, toggle de *Word Wrap* y vista en pantalla completa.
    - **Archivos**: `Markdown.kt`, `ChatRows.kt`.

26. **[Solo Móvil] Renderizador Visual de Bloques `diff` / Parches en el Chat**
    - **Mejora**: Renderizar bloques ```diff con líneas añadidas (`+`) en verde (`Palette.added`), eliminadas (`-`) en rojo (`Palette.removed`) y numeración de líneas.
    - **Archivos**: `Markdown.kt`, `ChatRows.kt`.

27. **[Solo Móvil] Barra de Formato Rápido y Atajos sobre `MessageComposer`**
    - **Mejora**: Al enfocar el campo de texto en `MessageComposer.kt`, mostrar una fila compacta de chips: `` `código` ``, ` ```bloque``` `, `@mención`, `/comando` y pegar del portapapeles.
    - **Archivos**: `MessageComposer.kt`.

28. **[Solo Móvil] Deslizar Burbuja para Responder en Hilo (`Swipe-to-Reply`)**
    - **Mejora**: Permitir deslizar cualquier mensaje de `ChatRow` hacia la izquierda para abrir su hilo de respuestas (`RepliesView`).
    - **Archivos**: `ChatRows.kt`.

29. **[Solo Móvil] Captura de Cámara Directa y Anotación/Dibujo sobre Imágenes antes de Adjuntar**
    - **Mejora**: Añadir opción de **Cámara** en el botón `+` de `MessageComposer.kt` y un lienzo de dibujo simple para rodear o señalar errores visuales en una captura antes de enviarla al agente.
    - **Archivos**: `MessageComposer.kt`, `AttachmentFiles.kt`.

30. **[Solo Móvil] Buscador Dentro de la Conversación Actual (`In-Chat Search`)**
    - **Mejora**: Añadir búsqueda dentro de `ChatScreen.kt` que filtre o resalte coincidencias en la transcripción y salte entre ellas con botones `↑` / `↓`.
    - **Archivos**: `ChatScreen.kt`, `ConversationList.kt`.

31. **[Solo Móvil] Botón `JumpToLatest` Mejorado con Contador de Nuevos Mensajes y Estado en Vivo**
    - **Mejora**: Mostrar sobre el botón flotante `JumpToLatest` en `ConversationList.kt` un badge con el número de respuestas nuevas recibidas mientras se lee el historial.
    - **Archivos**: `ConversationList.kt`.

32. **[Solo Móvil] Rediseño de "Full Conversation" (`TraceView`) como Línea de Tiempo Filtrable**
    - **Mejora**: Añadir filtros por chips en `TraceView` (*Todo*, *Comandos/Bash*, *Archivos editados*, *Pensamiento*, *Errores*) con bloques colapsables.
    - **Archivos**: `TraceView.kt`, `ChatScreen.kt`.

33. **[Solo Móvil] Panel de Inspector del Bot Unificado por Pestañas (`Details / Memory / Routines / Settings`)**
    - **Mejora**: Al tocar la cabecera del bot en `ChatScreen.kt`, abrir un inspector con pestañas superiores equivalente al `DetailsPanel.tsx` de escritorio para cambiar modelo, *Fast mode*, *Routines* y *Memory* sin cambiar de pantalla.
    - **Archivos**: `BotEditor.kt`, `ChatScreen.kt`.

34. **[Solo Móvil] Selección Parcial de Texto y Cita Directa (`Quote Reply`)**
    - **Mejora**: Permitir seleccionar una parte del mensaje del agente y pulsar *"Citar"*, insertando `> fragmento` en `MessageComposer.kt`.
    - **Archivos**: `ChatRows.kt`, `MessageComposer.kt`.

---

### Categoría 5: Utilidades Móviles de Productividad, Memoria, Rutinas y Sistema Android (Ideas 35–40)

35. **[Solo Móvil (con API de Host existente)] Port Completo de la Nueva Memoria de 2 Capas y Notas con Versiones**
    - **Mejora**: Actualizar `MemoryCard.kt` y `MemoryModel.kt` a la arquitectura de memoria 2.18/2.19 del host: dos capas (*Tu memoria* y *Memoria del bot*), entradas archivadas, notas largas con historial de versiones, switch de memoria propia del harness y tarjetas de *Handoff*.
    - **Archivos**: `MemoryCard.kt`, `MemoryModel.kt`.

36. **[Solo Móvil] Constructor Visual de Horarios en `RoutinesScreen` (Sin escribir Cron a mano)**
    - **Mejora**: Añadir un selector visual interactivo de frecuencia (*Cada X horas*, *Diario a las HH:MM*, *Días de la semana `L M X J V S D`*) en `RoutinesScreen.kt` que construya la expresión cron automáticamente.
    - **Archivos**: `RoutinesScreen.kt`.

37. **[Solo Móvil] Integración con el Menú Compartir de Android (`Intent.ACTION_SEND` / `ACTION_SEND_MULTIPLE`)**
    - **Mejora**: Permitir compartir enlaces, texto, capturas o archivos desde cualquier app de Android hacia Codync, eligiendo a qué bot enviarlo.
    - **Archivos**: `AndroidManifest.xml`, `MainActivity.kt`.

38. **[Solo Móvil] Atajos Dinámicos del Launcher (`ShortcutManagerCompat`) y Pin to Home Screen**
    - **Mejora**: Publicar los bots fijados como *Dynamic App Shortcuts* al mantener pulsado el icono de Codync en el launcher de Android y permitir fijar accesos directos de bots individuales en el escritorio del teléfono.
    - **Archivos**: `MainActivity.kt`, `AppStore.kt`.

39. **[Solo Móvil] Acciones Directas en Notificaciones de Android (Aprobar / Rechazar / Responder desde la Notificación)**
    - **Mejora**: Añadir `RemoteInput` para responder mensajes desde la notificación y botones de acción directa para aprobar o denegar permisos sin abrir la app.
    - **Archivos**: `PushService.kt`, `AppStoreNotifications.kt`.

40. **[Solo Móvil] Bloqueo Biométrico Opcional (Huella / Rostro) para Credenciales y Aprobaciones Críticas**
    - **Mejora**: Proteger con `BiometricPrompt` la vista de credenciales (`CredentialsView`), revocación de computadoras o aprobaciones sensibles.
    - **Archivos**: `MarketplaceView.kt`, `SettingsView.kt`.

---

### Categoría 6: Pantalla Remota (`ScreenView`) y Llamadas de Voz en Móvil (Ideas 41–44)

41. **[Solo Móvil] Barra de Macros de Desarrollador y D-Pad Táctil en `ScreenView`**
    - **Mejora**: Ampliar `ModifierBar` en `ScreenView.kt` con atajos de un toque (`Ctrl+C`, `Ctrl+V`, `Ctrl+Z`, `Cmd/Ctrl+S`, `Enter`, `Esc`, `Tab`) y repetición automática al mantener presionadas las flechas.
    - **Archivos**: `ScreenView.kt`, `ScreenKeys.kt`.

42. **[Solo Móvil] Modo Picture-in-Picture (PiP) Nativo para `ScreenView`**
    - **Mejora**: Permitir minimizar `ScreenView.kt` a una ventana flotante PiP de Android para seguir viendo cómo trabaja el bot en la pantalla de la PC mientras chateas o usas otra app.
    - **Archivos**: `MainActivity.kt`, `ScreenView.kt`, `AndroidManifest.xml`.

43. **[Solo Móvil] Lupa de Precisión (`Magnifier Loupe`) en Modo Táctil Directo de `ScreenView`**
    - **Mejora**: Mostrar una lupa circular sobre el dedo al arrastrar en modo `TouchMode.Direct` para hacer clic con precisión milimétrica en pantallas de escritorio de alta resolución.
    - **Archivos**: `ScreenView.kt`, `ScreenCanvas.kt`.

44. **[Solo Móvil] Rediseño de `CallBar` con Transcripción en Vivo y Controles de Micrófono / Altavoz**
    - **Mejora**: Añadir a la barra de voz en `ChatScreen.kt` botón de *Mute*, selector de salida de audio (Altavoz / Bluetooth / Auricular) y subtítulos en vivo de lo que se está reconociendo.
    - **Archivos**: `VoiceViews.kt`, `ChatScreen.kt`, `LocalVoiceAudio.kt`.

---

### Categoría 7: Nuevas Funciones Conjuntas (Móvil Android + Escritorio + Host) (Ideas 45–50)

45. **[Móvil + Escritorio/Host] Explorador de Archivos del Proyecto y Visor de `git status` / `git diff` en Vivo**
    - **Qué hace**: Permite explorar desde el móvil el árbol de archivos del workspace del bot (`cwd`), ver qué archivos fueron modificados (`git status`), inspeccionar el `git diff` en tiempo real y autocompletar `@archivo` en el compositor.
    - **Host / Escritorio**: Nuevos métodos RPC en `host/src/api/files.rs` (`workspaceTree`, `workspaceGitStatus`, `workspaceGitDiff`) y pestaña de cambios en `apps/desktop/src/renderer/views/thread/DetailsPanel.tsx`.
    - **Móvil**: Hoja interactiva *Archivos y Cambios Git* desde `ChatScreen.kt`.

46. **[Móvil + Escritorio/Host] "Handoff" / Continuar Conversación entre Escritorio y Móvil en un Toque**
    - **Qué hace**: Sincroniza qué bot/hilo está abierto activamente en `apps/desktop` para mostrar en Android un chip sobre la navbar (*"Continuar con [Bot] abierto en tu PC"*), y viceversa en el escritorio.
    - **Host / Escritorio**: Estado efímero de presencia de cliente en `host/src/hub.rs` y emisión en `apps/desktop/src/renderer/views/ChatWindow.tsx`.
    - **Móvil**: Chip de Handoff rápido en `MainTabs.kt`.

47. **[Móvil + Escritorio/Host] Monitor de Salud de la PC y Control de Energía (`Keep Awake`) desde el Móvil**
    - **Qué hace**: Muestra en el móvil y en el escritorio la telemetría del host (CPU, RAM, batería, agentes en ejecución) e incluye un interruptor remoto para activar **Evitar suspensión de la PC (`Keep Awake`)** mientras corren tareas largas.
    - **Host / Escritorio**: Exponer `host/src/service/` keep-awake y métricas en `host/src/api/host.rs` + vista en `apps/desktop`.
    - **Móvil**: Tarjeta de salud del sistema y switch *Keep Awake* en `SettingsView.kt` y `StateScreen.kt`.

48. **[Móvil + Escritorio/Host] Previsualización Móvil de Servidores Locales (`Localhost Web Preview` por Canal Encriptado)**
    - **Qué hace**: Cuando un bot levanta un servidor de desarrollo en la PC (`localhost:3000`, `5173`, `8080`), permite abrir y probar esa web en tiempo real desde un `WebView` dentro de la app de Android tunelizado por el `EncryptedChannel`.
    - **Host / Escritorio**: Proxy HTTP local sobre RPC en `host/src/api/remote.rs` y botón de compartir puerto en `apps/desktop`.
    - **Móvil**: Visor `LocalPreviewSheet.kt` en `apps/android`.

49. **[Móvil + Escritorio/Host] Portapapeles Universal y Dropzone Rápido PC ↔ Móvil**
    - **Qué hace**: Sincronización rápida de portapapeles y envío directo de archivos/capturas entre `apps/desktop` y `apps/android` en cualquier momento sin necesidad de iniciar una sesión de video WebRTC de `ScreenView`.
    - **Host / Escritorio**: Métodos `clipboardPush` / `quickDrop` en `host/src/api/remote.rs` y menú rápido en `apps/desktop`.
    - **Móvil**: Acción rápida en `MessageComposer.kt` y pestaña de PC.

50. **[Móvil + Escritorio/Host] Biblioteca Sincronizada de Prompts y Plantillas (`Saved Prompts & Snippets`)**
    - **Qué hace**: Biblioteca de prompts recurrentes guardada en el SQLite del `host`, editable en `apps/desktop` y en `apps/android`, e insertable con un toque desde el compositor de mensajes.
    - **Host / Escritorio**: Persistencia en `host/src/store/` y panel de gestión en `apps/desktop`.
    - **Móvil**: Selector de plantillas en `MessageComposer.kt`.

---

## Fases de Ejecución Recomendadas

- **Fase 1**: Rediseño de la barra de navegación inferior estilo Android (4 pestañas con badges), FAB ergonómico, buscador + chips de filtro en el roster, cabeceras nativas Android y tarjeta de aprobaciones rápidas (Ideas 1, 2, 3, 5, 11, 14, 17, 18, 20).
- **Fase 2**: Mejoras visuales de temas (Material You, Negro OLED, tinte por bot), bloques de código/diff, barra de formato en el compositor y *Swipe-to-Reply* (Ideas 9, 10, 13, 25–31).
- **Fase 3**: Port de Memoria 2.19, constructor visual de horarios Cron en Rutinas, menú Compartir de Android (`ACTION_SEND`) y mejoras de `ScreenView` / PiP (Ideas 35–44).
- **Fase 4**: Nuevas funciones conjuntas Móvil + Escritorio + Host (Explorador de archivos y `git diff`, Handoff, Monitor de PC + Keep Awake, Web Preview de localhost, Portapapeles Universal y Biblioteca de Prompts — Ideas 45–50).
