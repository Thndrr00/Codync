# 50 Mejoras e Implementaciones Extraídas de bdots para Codync

**Repositorio de origen:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots` (`fayazara/bdots`)  
**Licencia:** MIT (Commit `ce94160`)  
**Destino en Codync:** `host/` (Rust daemon), `apps/desktop/` (Electron/React/TS), `apps/android/` (Kotlin/Compose), `apps/ios/` (SwiftUI).  
**Fecha de elaboración:** 2026-10-25

---

## Estructura por Bloques Temáticos

- **Bloque 1: Implementación de Lógica Core, Runtime de Agentes y Micro-Apps (Ideas 1 – 15)**
- **Bloque 2: UI / UX, Micro-Animaciones, Transiciones y Componentes Visuales (Ideas 16 – 30)**
- **Bloque 3: Rendimiento, Salvaguardas, Seguridad y Resiliencia (Ideas 31 – 40)**
- **Bloque 4: Adición de Opciones, Ajustes de Usuario, Automatizaciones y Herramientas (Ideas 41 – 50)**

---

## Bloque 1: Implementación de Lógica Core, Runtime de Agentes y Micro-Apps

### 1. Servidor de Mini-Aplicaciones Web Generadas por Bots con Aislamiento CSP Sandbox
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\apps-runtime.ts` (Líneas 1-45) y `src\agents\app-store.ts`.
- **Qué hace el código:** Expone una ruta `/apps/<userId>/<botId>/<slug>/<token>/<path>` donde el bot sirve archivos HTML/CSS/JS generados en tiempo de ejecución. Inyecta la cabecera `Content-Security-Policy: sandbox allow-scripts allow-forms;` para crear un origen opaco totalmente aislado de cookies, sockets y almacenamiento del host.
- **Qué hacer con ese código en Codync:** Portar a Rust en `host/src/api/apps_runtime.rs`. Exponer una ruta similar en el servidor HTTP axum del host (`/api/apps/:bot_id/:slug/:token/*`), sirviendo los archivos desde el workspace del bot (`~/.codync/workspaces/<bot_id>/apps/<slug>/`) e inyectando las cabeceras CSP de sandbox estricto.

### 2. Motor Reactivo de Sincronización en Tiempo Real para Mini-Apps (`AppLive`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\apps-runtime.ts` (Líneas 60-120).
- **Qué hace el código:** Implementa un WebSocket dedicado bajo `_live` que gestiona suscripciones por app. Emite eventos estructurados `{ type: "change", collection: string, id?: string }` cuando se mutan datos y `{ type: "reload", version: number }` cuando el bot actualiza el código.
- **Qué hacer con ese código en Codync:** Implementar en `host/src/api/app_live_ws.rs` usando canales `tokio::sync::broadcast`. Cuando el agente o el usuario modifica una colección o guarda una nueva versión del HTML/JS, el host difunde el evento a todas las pestañas de visualización abiertas en Desktop o Móvil.

### 3. Bucle de Auto-Reparacin de Frontend (`_error` Reporting)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\apps-runtime.ts` (Líneas 48-58 y 180-210).
- **Qué hace el código:** Provee un endpoint `POST /_error` que recibe `{ message, stack, source, version }`. Al recibirlo, despierta automáticamente al bot con un prompt prioritario para que analice la excepción de JavaScript y republique la app con el bug solucionado.
- **Qué hacer con ese código en Codync:** Agregar la ruta en `host/src/api/apps_runtime.rs`. Al recibir un fallo, invocar `host::agent::enqueue_event()` con tipo `app_error` y el stack trace. En `apps/desktop/src/renderer/components/BotAppPanel.tsx`, inyectar un script base que intercepte `window.addEventListener('error')` y haga el POST automáticamente.

### 4. Micro-Base de Datos REST Embebida para Apps de Bots (`_data`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\app-store.ts` (Líneas 1-150) y `src\agents\bot-agent.ts:1883-1935`.
- **Qué hace el código:** Implementa un almacén documental en SQLite organizado por colecciones (`list`, `get`, `put`, `remove`). Las mini-apps usan `fetch('_data/prospects')` sin necesidad de configurar un backend externo.
- **Qué hacer con ese código en Codync:** Integrar tablas `bot_app_collections` en `host/crates/memory/src/sqlite.rs`. Exponer las operaciones en `host/src/api/apps_data.rs` y dotar al bot de las herramientas `app_data_put`, `app_data_list`, `app_data_delete` en `host/src/tools/apps.rs`.

### 5. Biblioteca Inyectada de Cliente `window.bdots` (`_bdots.js`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\apps-runtime.ts` (Líneas 130-175).
- **Qué hace el código:** Genera un script ligero en memoria que se sirve en `_bdots.js`. Expone `window.bdots.data.list()`, `window.bdots.data.put()`, `window.bdots.tell(text)` y escucha automáticamente el WebSocket `_live`.
- **Qué hacer con ese código en Codync:** Escribir `host/src/apps_runtime/codync_client_script.js` con el namespace `window.codync`. Este script se inyecta automáticamente en cualquier HTML servido por el host para permitir reactividad instantánea con cero configuración.

### 6. Canal de Accin Directa de la App hacia el Bot (`_tell`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\apps-runtime.ts` (Líneas 40-45 y 195-205).
- **Qué hace el código:** Permite a botones dentro de la mini-app hacer `POST /_tell` con `{ text: "Generar reporte Q3 con estos filtros seleccionados" }`. El texto entra al hilo del bot como una orden directa del usuario.
- **Qué hacer con ese código en Codync:** En `host/src/api/apps_runtime.rs`, mapear las peticiones a `_tell` directamente al dispatch de mensajes del host (`host::agent::handle_user_message`). El bot responde en el chat informando qué acción tomó en base a la interacción en la web app.

### 7. Enlaces de Acceso Basados en Tokens de Capacidad (`AppLink`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\lib\types.ts` (Líneas 213-225) y `src\agents\app-store.ts`.
- **Qué hace el código:** Genera enlaces seguros con tokens aleatorios (`token: string`, `label: string`, `mode: "edit" | "view"`, `owner: boolean`). Permite compartir una mini-app con un colega en modo sólo lectura o edición sin requerir autenticación completa.
- **Qué hacer con ese código en Codync:** Modelar `AppLink` en `host/src/apps_runtime/links.rs` y persistirlos en SQLite. En `apps/desktop/src/renderer/components/BotAppPanel.tsx`, agregar un botón "Compartir enlace" que permita generar URLs firmadas con selector de permisos (Ver / Editar).

### 8. Gestin de Elementos Abiertos y Seguimiento de Tareas (`track_item` / `OpenItem`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1591-1665) y `src\lib\types.ts:180-192`.
- **Qué hace el código:** Permite a los bots registrar incidencias pendientes de seguimiento en un tablero (`track_item`, `update_item`, `resolve_item`) con fecha de re-verificación (`checkAfter`).
- **Qué hacer con ese código en Codync:** Agregar la entidad `OpenItem` en el host (`host/src/agent/open_items.rs`). El bot puede registrar elementos en seguimiento (ej. "Esperando aprobación de factura en Gmail") y revisarlos de forma autónoma en chequeos periódicos.

### 9. Deduplicacin de Rutinas para Evitar Notificaciones Repetitivas (`mark_reported`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1666-1712).
- **Qué hace el código:** Provee las herramientas `mark_reported({ key, ttlDays })` y `was_reported({ key })`. Almacena hashes o IDs de correos/alertas para que los escaneos peridicos no notifiquen dos veces el mismo elemento al usuario.
- **Qué hacer con ese código en Codync:** Implementar una tabla de deduplicación con TTL en `host/crates/memory/src/dedup.rs`. Envolver las rutinas del host (`host/src/routines/`) para que utilicen este mecanismo antes de despachar notificaciones a los clientes.

### 10. Conexiones MCP a Nivel de Usuario con Estado Compartido
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\user-hub.ts` (Líneas 1-120) y `src\lib\types.ts:40-75`.
- **Qué hace el código:** Define `Connection` en el UserHub: una única instancia de servidor MCP (ej. Gmail o GitHub) que mantiene la autenticación y estado viva, permitiendo que todos los bots del usuario invoquen sus herramientas sin re-autenticarse individualmente.
- **Qué hacer con ese código en Codync:** Refactorizar el gestor de MCP en `host/src/mcp/manager.rs` para que las conexiones pertenezcan al host/usuario global, exponiendo un catálogo unificado (`AppCatalog`) que se inyecta selectivamente en los bots según sus políticas.

### 11. Ejecución de Tareas de Shell Asíncronas en Segundo Plano (`shell_job`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1172-1204).
- **Qué hace el código:** Permite lanzar comandos pesados (npm install, compilaciones, descargas) como trabajos en segundo plano (`jobId`), devolviendo el control al turno de inmediato y ofreciendo herramientas de sondeo de logs y estado.
- **Qué hacer con ese código en Codync:** Integrar en `host/src/tools/shell_job.rs` utilizando `tokio::process::Command` con canales de salida guardados en archivos temporales para inspección progresiva.

### 12. Asignacin Dinmica de Nombre por Primera Tarea (`set_name`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1205-1214) y `product.md:78`.
- **Qué hace el código:** Cuando un usuario crea un bot genérico y le da su primera tarea ("Quiero que revises facturas en PDF"), el bot ejecuta `set_name({ name: "Gestor de Facturas" })` y actualiza automáticamente su tarjeta y avatar.
- **Qué hacer con ese código en Codync:** Agregar la herramienta `set_name` en `host/src/tools/bot_identity.rs` que actualice el campo `name` en la base de datos de bots y emita un evento SSE `bot_renamed` a la interfaz.

### 13. Pasarela de Solicitud de Toma de Control Humano (`request_handoff`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1215-1232).
- **Qué hace el código:** El bot detecta que requiere un inicio de sesión o un CAPTCHA, llama a `request_handoff({ reason, url })` y suspende su turno dejando la sesión abierta.
- **Qué hacer con ese código en Codync:** Integrar en `host/src/tools/handoff.rs` vinculado con el subsistema de Remote Screen y navegador CDP, mostrando un modal emergente en la app Desktop con las instrucciones del bot.

### 14. Compartición de Hechos Transversal a Toda la Flota (`remember_for_all_bots`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1249-1258).
- **Qué hace el código:** Permite que un bot publique un hallazgo a la memoria global del usuario en el UserHub para que todos los demás bots lo conozcan de inmediato.
- **Qué hacer con ese código en Codync:** Extender `host/crates/memory` con un campo `scope: "bot" | "user"`. Implementar la herramienta `remember_for_all_bots` en el host para enriquecer el contexto compartido.

### 15. Búsqueda y Lectura de Historial Profundo (`search_history` / `read_history`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1259-1337).
- **Qué hace el código:** Herramientas para que el bot consulte turnos y mensajes de días o semanas anteriores mediante búsqueda semántica/FTS sin cargar todo el historial en el prompt activo.
- **Qué hacer con ese código en Codync:** Implementar FTS5 sobre la tabla de transcripciones en `host/crates/memory/src/history.rs` y exponer las herramientas correspondientes.

---

## Bloque 2: UI / UX, Micro-Animaciones, Transiciones y Componentes Visuales

### 16. Texto Shimmer Brillante para Estados de Pensamiento (`shimmer-text.tsx`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\components\shimmer-text.tsx` y `.agents\skills\transitions-dev\15-shimmer-text.md`.
- **Qué hace el código:** Efecto de gradiente animado en CSS que se desplaza de izquierda a derecha sobre el texto de pensamiento del agente ("Analizando dependencias...", "Buscando en la web...").
- **Qué hacer con ese código en Codync:** Portar la clase y animación `@keyframes shimmer` a `apps/desktop/src/renderer/styles/transitions.css` y crear el componente `apps/desktop/src/renderer/components/ShimmerText.tsx`. En Android, crear el modificador Compose `Modifier.shimmerEffect()` en `apps/android/design/src/main/java/app/codync/design/Shimmer.kt`.

### 17. Pestañas Deslizantes con Pldora Dinmica (`sliding-tabs.tsx`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\components\sliding-tabs.tsx` y `.agents\skills\transitions-dev\16-tabs-sliding.md`.
- **Qué hace el código:** Pestañas interactivas donde el indicador de selección (cápsula/pill de fondo) se desliza fluidamente con física de resortes (`framer-motion` o CSS spring) entre las opciones.
- **Qué hacer con ese código en Codync:** Reemplazar los botones de pestañas estáticos en `apps/desktop/src/renderer/components/Controls.tsx` con un componente `SlidingTabs`. En Android, portar a `TabRow` con indicador animado en `apps/android/design/src/main/java/app/codync/design/Controls.kt`.

### 18. Panel Deslizante Lateral con Fsica de Shell (`slide-shell.tsx`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\components\slide-shell.tsx` y `.agents\skills\transitions-dev\07-panel-reveal.md`.
- **Qué hace el código:** Contenedor de panel lateral que se oculta y revela suavemente recalculando el flexbox del layout principal sin causar saltos bruscos en el scroll del chat.
- **Qué hacer con ese código en Codync:** Utilizar en `apps/desktop/src/renderer/components/SidePanel.tsx` para alojar los paneles de la computadora del bot, el inspector de memoria o el visor de mini-apps.

### 19. Tarjeta de Ordenador con Stream de Pestaas en Vivo (`computer-card.tsx`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\components\computer-card.tsx` (Líneas 1-120).
- **Qué hace el código:** Tarjeta incrustada en el chat que muestra la miniatura de la pestaña activa del navegador con un badge de estado (`Working` / `Action needed` / `Done`), botón de refresco y botón de tomar el control ("Take over").
- **Qué hacer con ese código en Codync:** Crear `apps/desktop/src/renderer/components/ComputerCard.tsx` que reciba la URL del stream CDP de Codync Screen y renderice el visor interactivo inline.

### 20. Modal de Toma de Control Humano con Barra de Accin
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\components\computer-card.tsx` (Líneas 130-180).
- **Qué hace el código:** Modal a pantalla completa o vista amplia donde el usuario interactúa con la página directamente, rematado por una barra superior que dice: *"El bot necesita que inicies sesión en Salesforce. Cuando termines, haz clic en Continuar"* con botones `He terminado` y `Cancelar`.
- **Qué hacer con ese código en Codync:** Integrar en `apps/desktop/src/renderer/components/TakeoverModal.tsx` conectándolo con la señal de completitud de handoff hacia el host.

### 21. Catlogo de Avatares Geomtricos de Robots (`robot-heads`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\lib\types.ts` (Líneas 15-35) y `src\components\bot-avatar.tsx`.
- **Qué hace el código:** 12 combinaciones de formas geométricas de cabezas (`rectangle`, `circle`, `hexagon`, `square`) y colores de chasis (`#2b49a3`, `#e0563f`, `#2f8f5b`, etc.) asignadas ordenadamente a los bots creados.
- **Qué hacer con ese código en Codync:** Portar los SVGs y tokens de color a `apps/desktop/src/renderer/components/BotAvatar.tsx` y a `apps/android/design/src/main/java/app/codync/design/BotAvatar.kt`.

### 22. Spinner Animado en el Avatar Durante la Ejecucin
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots` (Commit `ce94160`).
- **Qué hace el código:** Cuando el bot entra en estado `working`, el avatar tradicional se transforma suavemente mediante un crossfade en un spinner circular estilizado con el color temático del bot.
- **Qué hacer con ese código en Codync:** Modificar `BotAvatar.tsx` para aceptar la prop `status: BotStatus`. Si es `working`, animar la transición hacia un spinner de pulso elegante.

### 23. Efecto de Inclinacin 3D de Tarjetas al Hover (`card-tilt.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\19-card-tilt.md`.
- **Qué hace el código:** Efecto de perspectiva CSS (`transform: perspective(1000px) rotateX(...) rotateY(...)`) que sigue la posición del ratón sobre la tarjeta de un bot o una mini-app.
- **Qué hacer con ese código en Codync:** Añadir la directiva o hook `useCardTilt()` en `apps/desktop/src/renderer/hooks/useCardTilt.ts` para aplicarlo en el catálogo de bots y tarjetas de aplicaciones.

### 24. Men Desplegable con Efecto Bloom / Expansin Elstica (`09-dropdown-bloom.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\09-dropdown-bloom.md` y `05-menu-dropdown.md`.
- **Qué hace el código:** Menú contextual que emerge con una ligera escala elástica desde el punto de pulsación (`transform-origin`) y desenfoque progresivo de fondo.
- **Qué hacer con ese código en Codync:** Aplicar en `apps/desktop/src/renderer/components/AnchoredMenu.tsx` y en los menús de contexto de Android en `apps/android/design/src/main/java/app/codync/design/DropdownMenu.kt`.

### 25. Bloque de Razonamiento Plegable con Flujo de Pensamiento (`29-reasoning-stream.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\29-reasoning-stream.md` y `28-thinking-states.md`.
- **Qué hace el código:** Visualizador de la cadena de pensamiento (thinking/reasoning tokens) que se muestra plegado con un indicador pulsante durante la generación y puede expandirse con transición suave de altura.
- **Qué hacer con ese código en Codync:** Integrar en el renderizador de mensajes en `apps/desktop/src/renderer/components/MessageItem.tsx` y en el cliente Android.

### 26. Contador Numrico Giratorio para Consumo de Tokens (`26-spinning-counter.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\26-spinning-counter.md` y `02-number-pop-in.md`.
- **Qué hace el código:** Transición vertical de dígitos estilo cuentakilómetros o tragaperras que anima suavemente el incremento en el contador de tokens utilizados y costo acumulado.
- **Qué hacer con ese código en Codync:** Crear `apps/desktop/src/renderer/components/SpinningCounter.tsx` para mostrar el uso de tokens en la barra de estado y tarjetas de rutinas.

### 27. Transicin de Resorte en Cambio de Tamao de Tarjeta (`01-card-resize.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\01-card-resize.md`.
- **Qué hace el código:** Anima el cambio de dimensiones de una tarjeta (por ejemplo cuando se expande el detalle de una herramienta) usando FLIP animation o `layoutId`.
- **Qué hacer con ese código en Codync:** Aplicar en las tarjetas de aprobación y de herramientas ejecutadas en el hilo del chat de Desktop.

### 28. Disolucin Suave al Limpiar Entradas de Texto (`13-input-clear-dissolve.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\13-input-clear-dissolve.md`.
- **Qué hace el código:** Al pulsar el botón de limpiar o tras enviar un mensaje, el texto se desvanece con una micro-escala hacia abajo en vez de desaparecer bruscamente.
- **Qué hacer con ese código en Codync:** Integrar en el compositor de chat `apps/desktop/src/renderer/components/Composer.tsx`.

### 29. Notificaciones Toast Apilables con Desplazamiento Suave (`22-toast.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\22-toast.md` y `32-banner-stacking.md`.
- **Qué hace el código:** Sistema de banners y toasts que entran desde la esquina inferior, apilándose con desfase de profundidad y escala para mensajes de sistema.
- **Qué hacer con ese código en Codync:** Implementar en `apps/desktop/src/renderer/components/ToastContainer.tsx`.

### 30. Sacudida de Error en Formularios y Tarjetas Rechazadas (`12-error-state-shake.md`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\.agents\skills\transitions-dev\12-error-state-shake.md`.
- **Qué hace el código:** Animación de vibración horizontal (`keyframes shake`) con atenuación de resorte cuando una acción es bloqueada por una política o una validación falla.
- **Qué hacer con ese código en Codync:** Aplicar a las tarjetas de aprobación cuando el usuario pulsa rechazar o cuando un comando es denegado por el host.

---

## Bloque 3: Rendimiento, Salvaguardas, Seguridad y Resiliencia

### 31. Guardin Antibucles con JSON Estable de Argumentos Ordenados (`LoopGuard`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\guards.ts` (Líneas 1-45).
- **Qué hace el código:** Clase `LoopGuard` con función `stableJson()` que serializa objetos con claves ordenadas alfabéticamente. Si un modelo ejecuta 5 llamadas idnticas consecutivas (`MAX_IDENTICAL_CALLS = 5`), corta la llamada e inyecta una advertencia imperativa al modelo para detener el reintento ciego.
- **Qué hacer con ese código en Codync:** Portar a Rust en `host/src/agent/loop_guard.rs`. Integrar la verificación en el ciclo de ejecución de herramientas de `host/src/agent/loop.rs`.

### 32. Poda Adaptativa de Pginas Web Anteriores en el Contexto (`KEEP_PAGE_RESULTS`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\guards.ts` (Líneas 46-75).
- **Qué hace el código:** Durante un turno con mltiples pasos de navegacin, retiene completas slo las ltimas 3 pginas visitadas (`KEEP_PAGE_RESULTS = 3`). Las pginas ms antiguas se recortan automticamente a 1,000 caracteres, ahorrando decenas de miles de tokens de contexto.
- **Qué hacer con ese código en Codync:** Implementar la funcin de poda en `host/src/agent/context_pruner.rs` aplicndola a los resultados de herramientas de lectura web o terminal.

### 33. Poda Estricta de Imgenes en la Ventana de Contexto (`KEEP_IMAGES = 2`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\guards.ts` (Lnea 52).
- **Qué hace el código:** Descarta de forma dinmica capturas de pantalla o imgenes intermedias anteriores, manteniendo nicamente las **2 ms recientes** en el contexto del modelo.
- **Qué hacer con ese código en Codync:** Aplicar en el preparador de mensajes del host (`host/src/agent/prepare_step.rs`) antes de despachar la llamada al LLM, reduciendo drsticamente el costo de tokens de visin.

### 34. Techo Mximo de Salida de Herramientas (`MAX_TOOL_RESULT_CHARS = 12,000`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\guards.ts` (Lnea 48).
- **Qué hace el código:** Trunca cualquier resultado masivo de herramienta a 12,000 caracteres con un aviso explcito: `[Truncated: output was X chars, showing first 12000]`.
- **Qué hacer con ese código en Codync:** Incorporar en la envolvente de ejecucin de herramientas en `host/src/tools/runner.rs`.

### 35. Proxy Zero-Leak de Secretos con Autenticacin Out-of-Band (`secret_request`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\secrets.ts` (Líneas 1-50) y `src\agents\bot-agent.ts:1377-1394`.
- **Qué hace el código:** El bot jams ve ni posee el secreto. Invoca `secret_request({ secret, url, method, body })`. El host valida el origen exacto (`https://api.github.com`), inyecta la cabecera `Authorization: Bearer <token>` directamente y enva la peticin.
- **Qué hacer con ese código en Codync:** Implementar el proxy de peticiones seguras en `host/src/security/secrets_proxy.rs` y la herramienta `secret_request` en `host/src/tools/secrets.rs`.

### 36. Redaccin Triple Anti-Exfiltracin en Respuestas HTTP
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\secrets.ts` (Líneas 45-65).
- **Qué hace el código:** Si el servidor remoto devuelve una respuesta de error reflejando el token, la funcin `redact()` busca y sustituye por `[redacted]` el secreto en formato plano, formato URL-encoded y formato Base64.
- **Qué hacer con ese código en Codync:** Portar la funcin de sanitizacin a `host/src/security/redactor.rs` aplicndola sobre cualquier cuerpo de respuesta devuelto al agente.

### 37. Validacin Estricta de Origen HTTPS para Secretos (`httpsOrigin`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\secrets.ts` (Líneas 15-25).
- **Qué hace el código:** Garantiza que un secreto slo pueda enviarse a su `https://` especfico, rechazando automticamente peticiones con usuario/contraseña embebida en la URL o protocolos inseguros.
- **Qué hacer con ese código en Codync:** Integrar la validacin en el dispatcher de peticiones de red del host.

### 38. Grabador Semntico de Demostraciones en Navegador con Exclusin de Contraseas
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\teach.ts` (Líneas 10-60).
- **Qué hace el código:** Inyecta un script ligero en la pgina durante el Take-Over que registra clics y cambios de campos extrayendo selectores semnticos (etiquetas ARIA, roles, placeholders) y filtrando estrictamente mediante expresiones regulares cualquier campo que coincida con `pass|otp|code|cvv|card|ssn|pin|secret|token`.
- **Qué hacer con ese código en Codync:** Alojar el script de grabacin en `host/src/browser/teach_recorder.js` e inyectarlo va CDP en las sesiones de toma de control.

### 39. Preservacin del Log de Demostracin a Travs de Navegaciones (`window.name`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\teach.ts` (Líneas 20-35).
- **Qué hace el código:** Guarda el log de acciones en `window.name` (que sobrevive navegaciones en el mismo sitio y cambios de dominio) adems de `localStorage`, asegurando que no se pierda la traza al redirigir tras un login.
- **Qué hacer con ese código en Codync:** Utilizar este mismo truco de runtime para garantizar que los flujos demostrados por el usuario se mantengan ntegros de inicio a fin.

### 40. Extraccin de la Ruta Completa de Navegacin (`Page.getNavigationHistory`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\teach.ts` (Líneas 70-110).
- **Qué hace el código:** Al concluir la grabacin, utiliza el comando CDP `Page.getNavigationHistory` para obtener la secuencia cronolgica de pginas visitadas, correlacionndola con los eventos de clic del DOM.
- **Qué hacer con ese código en Codync:** Integrar en el sintetizador de demostraciones de `host/src/browser/teach_analyzer.rs`.

---

## Bloque 4: Adicin de Opciones, Ajustes de Usuario, Automatizaciones y Herramientas

### 41. Panel de Configuracin Proactiva con Horario de Silencio (`ProactiveSettings`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\lib\types.ts` (Líneas 148-178) y `src\components\settings-panel.tsx`.
- **Qué hace el código:** Permite configurar: chequeos cada N horas, hora de inicio de silencio (`quietStart`, ej. "22:00"), hora de fin (`quietEnd`, ej. "08:00"), zona horaria local y tope mximo de mensajes espontneos al da (`maxPerDay: 6`).
- **Qué hacer con ese código en Codync:** Agregar estos ajustes en la configuracin del bot en `host/src/config/bot.rs` y construir la interfaz de configuracin en `apps/desktop/src/renderer/components/BotSettingsModal.tsx` y en Android.

### 42. Disparadores Ligeros de Observacin para Rutinas (`RoutineWatch`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\lib\types.ts` (Líneas 140-146) y `product.md:141`.
- **Qué hace el código:** Define `watch: { url: string } | { command: string }`. Antes de despertar al costoso modelo de lenguaje, la rutina ejecuta una peticin HTTP ligera o un comando de shell; slo si la salida o hash cambi respecto al ltimo chequeo se despierta el LLM.
- **Qué hacer con ese código en Codync:** Incorporar el pre-chequeo en `host/src/routines/watcher.rs`. Si la respuesta HTTP devuelve `304 Not Modified` o el mismo hash SHA256, se descarta el turno ahorrando el 100% de tokens.

### 43. Matriz de Polticas de Accin por Clases (`ACTION_CLASSES`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\lib\types.ts` (Líneas 93-130).
- **Qué hace el código:** Categoriza las herramientas en 7 clases claras para el usuario: Editar archivos (`files`), Ejecutar comandos (`shell`), Leer la web (`browse`), Conducir el navegador (`browser`), Usar apps conectadas (`apps`), Mensajear otros bots (`bots`), Programar rutinas (`routines`), asignando a cada una: `allow` | `ask` | `block`.
- **Qué hacer con ese código en Codync:** Reemplazar las reglas individuales por esta matriz intuitiva en `host/src/security/policy.rs` y crear la UI de seleccin en los ajustes del bot en Desktop y Móvil.

### 44. Conmutador de Ejecucin de Apps en Segundo Plano (`appsInBackground`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\lib\types.ts` (Líneas 160-165).
- **Qué hace el código:** Ajuste explcito de seguridad que impide que un bot ejecutando trabajo nocturno o en segundo plano llame a herramientas con efectos externos (como enviar correos o crear issues) si su poltica est en "Preguntar primero", ya que el usuario no estar presente para aprobar.
- **Qué hacer con ese código en Codync:** Implementar la guarda en `host/src/agent/approval_gate.rs`.

### 45. Herramienta para Solicitar Secretos al Usuario (`request_secret`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1338-1376).
- **Qué hace el código:** Si el bot intenta acceder a una API y no tiene credenciales, ejecuta `request_secret({ name: "stripe_key", origin: "https://api.stripe.com", scheme: "bearer", prompt: "Necesito tu clave de Stripe para conciliar pagos" })`. Esto despliega una tarjeta de entrada enmascarada en la UI del chat.
- **Qué hacer con ese código en Codync:** Crear la herramienta en `host/src/tools/request_secret.rs` y el componente de UI `apps/desktop/src/renderer/components/SecretPromptCard.tsx`.

### 46. Herramienta para Catalogar Secretos Disponibles (`list_secrets`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1395-1401).
- **Qué hace el código:** Devuelve al modelo la lista de nombres de secretos guardados, su esquema y el origen HTTPS permitido, sin revelar jams el valor secreto.
- **Qué hacer con ese código en Codync:** Implementar en `host/src/tools/secrets.rs`.

### 47. Herramienta de Guardado y Publicacin de Mini-Apps (`save_app`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1844-1864).
- **Qué hace el código:** Permite al bot guardar un archivo HTML, JavaScript y metadatos bajo un slug (`save_app({ slug: "kanban", name: "Tablero de Proyecto", html: "..." })`), incrementando la versin automticamente.
- **Qué hacer con ese código en Codync:** Implementar la herramienta en `host/src/tools/apps.rs`, escribiendo los archivos en la carpeta de apps del bot y emitiendo el evento de recarga.

### 48. Herramientas de Administracin de Mini-Apps (`list_apps` y `delete_app`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1865-1882).
- **Qué hace el código:** Permiten al bot y al usuario auditar qu aplicaciones web tiene activas el bot y eliminarlas cuando ya no se necesiten.
- **Qué hacer con ese código en Codync:** Implementar en `host/src/tools/apps.rs` y en la pestaa "Apps" del panel lateral de Codync Desktop.

### 49. Herramienta de Sntesis de Demostracin en Habilidad (`finish_teaching`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\agents\bot-agent.ts` (Líneas 1758-1804).
- **Qué hace el código:** Extrae el registro de la sesin de enseanza y compila un prompt estructurado pidiendo al modelo que genere un script limpio o una habilidad que reproduzca exactamente los pasos demostrados.
- **Qué hacer con ese código en Codync:** Integrar en `host/src/browser/teach_synthesis.rs` para consolidar las demostraciones en habilidades de `~/.codync/skills/`.

### 50. Monitor de Estado y Diagnstico de Conexiones MCP (`lastFailure`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\bdots\src\lib\types.ts` (Líneas 65-75) y `src\lib\connection-status.ts`.
- **Qué hace el código:** Rastrea el ltimo fallo de cada servidor MCP conectado clasificndolo en `auth` (token expirado), `unreachable` (servidor cado) o `tool` (error interno de parmetros), mostrando un indicador visual con URL de re-autenticacin si aplica.
- **Qué hacer con ese código en Codync:** Integrar este enriquecimiento de diagnstico en `host/src/mcp/health.rs` y mostrar el estado detallado en la seccin de Ajustes de Conexiones de Desktop.
