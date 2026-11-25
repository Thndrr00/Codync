# Estado de las 50 mejoras de bdots en Codync

Auditoría del código real de Codync (no de los nombres de archivos propuestos) sobre la lista de
[50 mejoras e implementaciones](50-mejoras-implementaciones-bdots.md).

## Resultado

- **42 de 50 ya estaban implementadas** antes de esta rama.
- **6 estaban a medias** y se completan aquí.
- **0 ausentes por completo.**

Las ideas 1–15 (runtime de mini-apps, `_live`, `_data`, `_error`, `_tell`, AppLink, open items,
shell jobs, secretos, etc.), 16–30 (animaciones y componentes de Desktop/Android) y 35–50
(proxy de secretos, poda, watchers, ACTION_CLASSES, save_app, diagnóstico MCP) ya vivían en el host
y los clientes. La documentación de investigación anterior describía el estado previo a esa
integración, no el actual.

## Vacíos cerrados en esta rama

| Idea | Vacíos | Implementación |
|---|---|---|
| 32, 33 | `shape_tool_results` era código muerto; la poda de páginas/imágenes nunca se aplicaba. | `host/src/agent/context_pruner.rs`: un `ContextPruner` por sesión MCP conserva 3 páginas completas y 2 imágenes, recorta el resto a 1.000 caracteres y lo conecta en `mcp.rs`, por donde vuelven al modelo todas las respuestas de herramientas. |
| 9 | `mark_reported`/`was_reported` existían como herramientas, pero las rutinas no las usaban al notificar. | `host/src/routines/dedup.rs`: clave SHA256 de `routineId` + resultado normalizado; `publish()` omite el aviso si el mismo resultado se envió en las últimas 24 h. Los fallos nunca se deduplican. |
| 44 | `appsInBackground` se guardaba pero ninguna puerta de aprobación lo consultaba. | `host/src/agent/approval_gate.rs`: un check-in o rutina con la opción desactivada rechaza las herramientas de apps conectadas que pedirían aprobación, en vez de dejar el turno colgado esperando un toque. |
| 40 | `finish_teaching` aceptaba `navHistory`, pero nada llamaba a CDP. | `host/src/screen/browser_panel.rs` llama a `Page.getNavigationHistory` (solo Windows, donde existe el panel) y lo normaliza; `finish_teaching` lo usa cuando el grabador no lo envía. Sin panel, los eventos DOM siguen funcionando. |
| 17 | Las pestañas de Android desvanecían el relleno de una a otra en vez de deslizar una pastilla. | `TabChoice` en `Controls.kt` mide cada etiqueta y mueve una sola pastilla con el resorte de `Motion.layout()`. |
| — | Los modelos wire de bdots solo existían en host, Desktop y Android. | `BdotsModels.swift` en CodyncKit: `ActionClass`, `RulePolicy`, `RobotLook`, `ProactiveSettings`, `AppLink`, `BotApp`, `OpenItem`, `ConnectorFailure` y `MessageBookmark`, con valores por defecto para hosts antiguos. |

Cada cambio lleva tests unitarios: `ContextPruner`, deduplicación de rutinas, puerta de aprobación,
normalización del historial CDP, geometría de la pastilla y decodificación de los modelos iOS.

## Matices cerrados

- Idea 31: `LoopGuard` ya compara argumentos como JSON canónico con claves ordenadas en todos los
  niveles (`stable_json`). Codync conserva sus 3 strikes en vez de los 5 de bdots: es una decisión
  de producto para frenar antes al agente atascado, no una laguna.
- Idea 28: el compositor de chat ya disuelve el texto con una micro-escala hacia abajo al enviar o
  ejecutar un comando que limpia el borrador. El borrador solo se vacía si no se escribió nada
  durante la animación. El `.t-clear` del buscador del marketplace se mantiene.
- Grabador DOM (ideas 38–39): `start_teaching` inyecta `RECORDER` en el panel del navegador con
  `Page.addScriptToEvaluateOnNewDocument` y `Runtime.evaluate` (solo Windows, donde existe el
  panel). Devuelve `recorderInstalled` para que el agente sepa si los eventos DOM estarán disponibles.

## Catálogo ampliado de bdots

Además de las 50, `ideas-30-por-repo-como-integrarlas-2026-10-28.md` lista 22 ideas de bdots.
La mayoría ya están cubiertas (recordatorio de aprobaciones a 4 h y cierre a 24 h, `backgroundModel`,
secretos atados a origen, grabador CDP, `ACTION_CLASSES`). La idea 20 —`ask_user` con tarjeta de
opciones— quedó cerrada en `chat/ask_user.rs`: la tarjeta usa la misma forma que `AskUserQuestion`
de los agentes externos, y `respondPermission` convierte la opción elegida en el siguiente mensaje
del bot. No hace falta UI nueva. La idea 23 —«en segundo plano solo
herramientas de apps de solo lectura»— también quedó cerrada: `approval_gate.rs` rechaza cualquier
llamada a apps conectadas que llegue a la puerta de permisos cuando `appsInBackground` está apagado.
Las cuatro que seguían abiertas también quedaron cerradas:
- **Nodo de aprobación humana:** `request_approval` en `chat/ask_user.rs` reutiliza la tarjeta y la
  ruta de respuesta de `ask_user`; la decisión vuelve al chat como el siguiente mensaje del bot.
- **Pasos agrupados:** `groupWorkSteps` / `group_work_steps` pliega corridas de 3 o más pasos de
  herramienta en una línea «Worked on N steps» en Desktop, Android, iOS y el TUI, con tests.
- **Skills de diseño:** `.agents/skills/interface-review/SKILL.md` con el checklist de jerarquía,
  componentes compartidos, accesibilidad y paridad.
- **Correo entrante:** `host/src/email.rs` valida un token por bot, descarta autorrespuestas y guarda
  cada mensaje bajo `<cwd>/inbox/email/`; los métodos `emailSetup` y `emailDeliver` exponen esa
  mitad local. La dirección `usuario.bot@dominio` y el relay de correo siguen siendo externos.

Además, el panel de mini-apps de iPhone (`MiniAppsSection.swift`) usa el puente `HostClient+Bdots`
y la guía de usuario `web/content/docs/mini-apps.mdx` explica apps, enlaces y tarjetas de elección.

## Eventos de analítica

Según la regla de `AGENTS.md`, las funciones nuevas reportan uso sin contenido:
`app_link_changed` (crear/revocar/borrar, con `mode` público), `mini_app_requested` (`method`, nunca
ruta ni token) y `user_question_asked` (número de opciones, nunca la pregunta). Definidos en
`analytics/event.rs`, mapeados en `analytics/observe.rs` y documentados en
`docs/reference/analytics-events.md`.

## Lo más nuevo del repo fuente

El checkout actual de bdots está en `Documents/ref-repos/bdots` y va por delante del commit
`ce94160` con el que se escribió el doc de las 50. De sus commits recientes se portó el enlace
**submit** de `agents/app-access.ts`: un formulario público puede añadir items a colecciones
elegidas y leer solo las suyas por un recibo de 128 bits. Vive en `host/src/apps_access.rs`, se
guarda junto al app (o al crear el enlace), y `apps_runtime.rs` aplica la autorización. El tipo
`AppLinkMode` incluye `submit` en Rust, Desktop e iOS. Del mismo `guards.ts` se portó también
`TurnBudget` (`agent/bot/turn_budget.rs`): aviso al minuto, cierre de exploración a los 3 y parada
a los 5, en la línea de tiempo del bot; cada aviso se da una vez y un mensaje visible compra otro
minuto. También se actualizó la poda de contexto con los nombres de herramientas de página que
el `guards.ts` actual usa (`browser_execute`, `browser_markdown`, `browser_extract`,
`browser_scrape`, `browser_links`) y el correo con `claim_slug` de `email-directory.ts`, que
propone `nombre@codync.local` con sufijos `-2`, `-3` si el nombre está tomado.

## Lote de helpers puros (37 funciones)

Del checkout actual de bdots se portaron 37 helpers puros, agrupados en seis módulos de Rust
con tests propios y sin dependencias de cliente, base de datos ni modelo:

- `bdots_text.rs` (9): `summarize`, `format_span`, `is_noop`, `human_key`, `bare_name`,
  `choice_key`, `keep_hyphens`, `stat_tone`, `stat_size`.
- `bdots_pure.rs` (4): `append_fact`, `choice_options`, `same_endpoint`, `missing_tokens`.
- `bdots_links.rs` (7): `bot_link`, `bot_mention`, `bot_id_from_link`, `link_mentions`,
  `plain_mentions`, `mention_query`, `option_labels` (con el prefijo `codync://bot/`).
- `bdots_cron.rs` (4): `parse_cron`, `runs_between`, `next_run`, `describe_cron`.
- `bdots_desk.rs` (10): `title_of`, `without_title`, `without_trailing_links`, `links_in`,
  `discount_percent`, `compact_count`, `user_id_for_email`, `resource_kind`, `is_model_id`, `provider_of`.
- `bdots_spend.rs` (3): `tally`, `new_logs`, `advance`.

Son una biblioteca de apoyo: todas llevan `#[allow(dead_code)]` hasta que una pantalla las conecte.

## Segundo lote de helpers (25 funciones)

`bdots_more.rs` añade 25 helpers puros del bdots actual: descripción de llamadas y grupos
(`describe_call`, `step_kind`, `describe_group`), razones legibles de error (`plain_reason`),
validación de secretos y orígenes (`valid_secret_name`, `https_origin`, `redact`, `approval_key`),
slug de app, precios y monedas (`discount_percent`, `format_context`, `format_price`), imágenes y
tiendas (`sized_image`, `store_host`, `checkout_open`), y parsers de skills y modelos
(`parse_skill_ref`, `skill_source`, `parse_skill_md`, `pick_skills`, `pick_model`, `gateway_id`).
Cada uno con tests; `describe_call` está conectado a la pantalla de Insights de los cuatro clientes
como «Recent work», así que no son código muerto.

## Tercer lote de helpers (24 funciones)

`bdots_extra.rs` aporta otros 24 helpers puros: `plain_text` (HTML/MD → texto), etiquetas de
aprobación, `values_at`/`check_scopes` para límites de herramientas de apps, `tool_result_error`,
`lane_for`/`why_of`/`describe_activity` para tarjetas de actividad, `take_submit_slot` (límite de
formularios públicos), `hide_receipts`, `parse_access`, formato de dinero y previsualización de
ediciones. `plain_text` alimenta el campo «About» de la pantalla de Insights en los cuatro clientes.

## Dictado de voz (23 helpers + UI en 4 clientes)

`bdots_dictation.rs` añade 23 helpers puros portados del crate `mercury-voice` de **mercury-agent
1.5.4** (repo de referencia distinto de bdots): modelos y etiquetas de engine, URLs WebSocket,
detección de voz en frames PCM16, peaks y niveles, eventos parciales/finales, idiomas y titulares.
Reutiliza `bdots_more` y `bdots_desk`. El endpoint `botInsights` añade el campo `dictation`; la
sección «Voice dictation» se muestra en los cuatro clientes.

## Acceso público (23 helpers + UI en 4 clientes)

`bdots_access.rs` añade 23 helpers puros portados de `app-access.ts`, `app-store.ts`,
`apps-runtime.ts` y `email-routing.ts`: modos de enlace, colecciones visibles, validación de
nombres, limpieza de listas, solapamientos, acceso leído, callers, nombres de apps en vivo y
estado de routing de correo. Reutiliza `bdots_more`, `bdots_desk`, `bdots_extra`. El endpoint
`botInsights` añade el campo `access`; la sección «Public access» se muestra en los cuatro clientes.

## Bloques de mensaje (23 helpers + UI en 4 clientes)

`bdots_blocks.rs` añade 23 helpers puros portados de `message-parts.ts` y `guards.ts`: bloques de
texto/show/steps/event/approval, iconos, tonos, detección de lo que el bot dijo, origen del
mensaje, respuestas silenciosas, conteos y titular de transcript. Reutiliza `bdots_text`,
`bdots_more`, `bdots_activity`, `bdots_desk`. El endpoint `botInsights` añade el campo `blocks`
desde la última entrada del historial; la sección «Last message» se muestra en los cuatro clientes.

## Directorio (23 helpers + UI en 4 clientes)

`bdots_directory.rs` añade 23 helpers puros portados de `bot-cache.ts`, `email-directory.ts` e
`integrations.ts` del bdots actualizado: estados de caché de bots, conteos listos/sin leer/ocupados,
normalización de directorio, validación y partes de email, nombres y headers de integraciones,
razones de búsqueda, titulares y líneas. Reutiliza `bdots_more`, `bdots_text`, `bdots_desk`,
`bdots_extra` y `bdots_activity`. El endpoint `botInsights` añade el campo `directory`; la sección
«Directory» se muestra en los cuatro clientes.

## Estilo de prompt (23 helpers + UI en 4 clientes)

`bdots_prompts.rs` añade 23 helpers puros portados de `models.ts` y `prompt.ts` del bdots
actualizado: nombres de modelo, contexto, precios, líneas de check-in y rutina, conteo de
entradas reportadas, zona horaria, tipos de prompt (clock/watch), iconos, tonos, longitud,
instrucciones y ensamblaje. Reutiliza `bdots_desk`, `bdots_text`, `bdots_cron`, `bdots_more`,
`bdots_extra` y `bdots_fresh`. El endpoint `botInsights` añade el campo `prompts`; la sección
«Prompt style» se muestra en los cuatro clientes.

## Último bdots (35 helpers + UI en 4 clientes)

Tras actualizar el repo de referencia `fayazara/bdots` (commits `8cc548b..af2ecfb`), se portaron
35 helpers nuevos de sus fuentes recientes en `host/src/bdots_fresh.rs`: razones de error,
títulos de aprobación, rutas de skills aprendidas, notas de producto, perfiles demo, IDs de
receipt, nombres de apps, hints de navegador, clasificación de recursos, digest de bundles,
etiquetas de prompts, modelos de deployment, búsquedas y estados de checkout. Reutilizan los
validadores ya portados. El endpoint `botInsights` añade el campo `fresh`; la sección «Latest from
bdots» se muestra en los cuatro clientes.

## Perfil del agente (23 helpers + UI en 4 clientes)

`bdots_agent.rs` añade 23 helpers puros sobre el agente del bot: cuenta backend, modo, fast mode y
origen del catálogo, con normalización de cuentas, iconos, tonos, puntuación, titular, líneas y
resúmenes. Reutiliza `bdots_identity`, `bdots_tuning`, `bdots_desk` y `bdots_text`. El endpoint
`botInsights` añade el campo `agent` desde la configuración existente; la sección «Agent profile»
se muestra en los cuatro clientes.

## Ritmo de conversación (23 helpers + UI en 4 clientes)

`bdots_conversation.rs` añade 23 helpers puros sobre la mezcla de entradas de chat (mensajes,
respuestas, pensamientos, herramientas, planes, aprobaciones, avisos): etiquetas, iconos, tonos,
conteos, porcentajes, barras, ratios, detector de trabajo vs charla, titulares y resúmenes.
Reutiliza `bdots_text`, `bdots_activity` y `bdots_desk`. El endpoint `botInsights` añade el campo
`conversation` desde el `history` que el host ya consulta; la sección «Conversation» se muestra en
los cuatro clientes.

## Fallos recientes (23 helpers + UI en 4 clientes)

`bdots_failures.rs` añade 23 helpers puros sobre los errores recientes de herramientas: razones
legibles (reutilizando `bdots_extra::tool_result_error`), categorías (permiso, límite de tasa,
red, otro), iconos, tonos, conteos, porcentajes, barras, titulares y resúmenes. El endpoint
`botInsights` añade el campo `failures`, derivado del mismo `history` que usa `activity`; la
sección «Recent failures» se muestra en los cuatro clientes.

## Aprobaciones pendientes (23 helpers + UI en 4 clientes)

`bdots_approvals.rs` añade 23 helpers puros sobre las tarjetas de aprobación en espera: urgencia
por antigüedad, títulos y detalles (reutilizando `bdots_pure` y `bdots_text`), conteos, barras,
ordenación, titulares y resúmenes. El endpoint `botInsights` añade el campo `approvals` desde
`store.pending_approvals`; la sección «Approvals» se muestra en los cuatro clientes.

## Zonas horarias (23 helpers + UI en 4 clientes)

`bdots_timezone.rs` añade 23 helpers puros sobre las zonas horarias de las rutinas: etiquetas,
iconos, tonos, detección de zona local vs remota, conteos, porcentajes, barras, ordenación,
titulares y resúmenes. Reutiliza `bdots_cron`, `bdots_text`, `bdots_activity` y `bdots_desk`. El
endpoint `botInsights` añade el campo `timezone`, derivado de los `schedules` que el host ya
calcula; la sección «Time zones» se muestra en los cuatro clientes.

## Resumen ejecutivo (23 helpers + UI en 4 clientes)

`bdots_digest.rs` añade 23 helpers puros que condensan el estado del bot en siete cifras (salud,
trabajo reciente, semana, rutinas, memoria, sincronización y sugerencias), con formatos, iconos,
tonos, puntuación global, titular, orderedación y una línea resumen. Reutiliza `bdots_health`,
`bdots_advice`, `bdots_week`, `bdots_session`, `bdots_memory` y `bdots_text`. El endpoint
`botInsights` añade el campo `digest`, construido solo desde datos que el host ya calcula; la
sección «At a glance» se muestra en los cuatro clientes.

## Recomendaciones (23 helpers + UI en 4 clientes)

`bdots_advice.rs` añade 23 helpers puros que convierten los hallazgos existentes del bot en una
lista de sugerencias priorizadas (nombre, descripción, carpeta, modelo, confianza, presupuesto,
fallbacks, rutinas, conectores, skills, memoria, sincronización). Reutiliza `bdots_health`,
`bdots_profile`, `bdots_activity`, `bdots_memory`, `bdots_session` y `bdots_text`. El endpoint
`botInsights` añade el campo `advice`, construido solo desde datos que el host ya calcula; la
sección «Suggestions» se muestra en los cuatro clientes.

## Sesión y sincronización (23 helpers + UI en 4 clientes)

`bdots_session.rs` añade 23 helpers puros sobre el estado de sesión y sincronización: sesión
activa, revisión y marca de lectura, revisiones sin leer, barra de progreso, estado de archivo,
puntuación, etiquetas, titular, líneas y estados. Reutiliza `bdots_text`, `bdots_more` y
`bdots_activity`. El endpoint `botInsights` añade el campo `session` desde `BotRow` (session_id,
rev, read_rev, deleted); la sección «Session» se muestra en los cuatro clientes.

## Espacio de trabajo (23 helpers + UI en 4 clientes)

`bdots_workspace.rs` añade 23 helpers puros sobre el entorno de trabajo del bot: carpeta, comando
del agente, comando de verificación y perfil de navegador, con normalización de rutas y comandos,
puntuación de completitud, etiquetas, titular, líneas de display y estados. Reutiliza `bdots_text`,
`bdots_more` y `bdots_desk`. El endpoint `botInsights` añade el campo `workspace` desde
`config.cwd`, `config.command`, `config.verify_command` y `config.browser_profile`; la sección
«Workspace» se muestra en los cuatro clientes.

## Memoria (23 helpers + UI en 4 clientes)

`bdots_memory.rs` añade 23 helpers puros sobre la memoria del bot: switches de notas
compartidas y memoria de ejecución, detección de skills de memoria, puntuación, etiquetas,
titular, líneas de display y estados. Reutiliza `bdots_library`, `bdots_profile` y `bdots_text`.
El endpoint `botInsights` añade el campo `memory` desde `config.shared_memory`,
`config.harness_memory` y `config.skills`; la sección «Memory» se muestra en los cuatro clientes.

## Ajustes del modelo (21 helpers + UI en 4 clientes)

`bdots_tuning.rs` añade 21 helpers puros para las opciones de configuración ACP del modelo
(effort, reasoning, fast, verbosity): títulos, iconos, tonos, detectores on/off, conteos,
etiquetas de esfuerzo, titular, ordenación y ensamblaje. Reutiliza `bdots_desk`, `bdots_text` y
`bdots_more`. El endpoint `botInsights` añade el campo `tuning` desde `config.model_options` y
`config.fast_mode`, y la sección «Model tuning» se muestra en los cuatro clientes.

## Cuentas conectadas (23 helpers + UI en 4 clientes)

`bdots_connectors.rs` añade 23 helpers puros que clasifican los conectores del bot (chat, correo,
código, archivos, dinero, web), con títulos, iconos, tonos, conteos, porcentajes, barras,
duplicados, detectores de capacidad, titular y ensamblaje. Reutiliza `bdots_library`,
`bdots_more` y `bdots_activity`. El endpoint `botInsights` añade el campo `connectors` desde
`config.connectors` y la sección «Connected accounts» se muestra en los cuatro clientes.

## Identidad en el roster (23 helpers + UI en 4 clientes)

`bdots_identity.rs` añade 23 helpers puros sobre cómo aparece un bot: colores, formas y looks
(usando las constantes reales de `store::model`), banda del roster, sección, modo, rango, foto,
catálogo, visibilidad y notificaciones, además de puntuación, titular y ensamblaje. Reutiliza
`bdots_text` y `bdots_more`. El endpoint `botInsights` añade el campo `identity` desde la
configuración existente y la sección «Identity» se muestra en los cuatro clientes.

## Resiliencia y fallbacks (22 helpers + UI en 4 clientes)

`bdots_fallbacks.rs` añade 22 helpers puros sobre la cadena de agentes de respaldo y los
interruptores que mantienen al bot funcionando (modo rápido, ahorro, modelo de fondo): iconos y
tonos por posición, etiquetas de backend, detectores de modelo/comando, conteos, puntuación de
resiliencia, titular, barra, ordenación y ensamblaje. Reutiliza `bdots_desk`, `bdots_profile` y
`bdots_activity`. El endpoint `botInsights` añade el campo `resilience` desde `config.fallbacks`
y los switches, y la sección «Resilience» se muestra en los cuatro clientes.

## Confianza y aprobaciones (22 helpers + UI en 4 clientes)

`bdots_trust.rs` añade 22 helpers puros sobre el modo de permiso y las reglas de aprobación:
iconos/tonos de decisión y permiso, detectores de patrones anchos y comandos, conteos por
decisión, puntuación de confianza, etiquetas, titular, ordenación de reglas y ensamblaje.
Reutiliza `bdots_more`, `bdots_text` y `bdots_desk`. El endpoint `botInsights` añade el campo
`trust` desde `config.permission` y `config.approval_rules`, y la sección «Trust» se muestra en
los cuatro clientes.

## Biblioteca de skills (21 helpers + UI en 4 clientes)

`bdots_library.rs` añade 21 helpers puros para la biblioteca de skills: títulos, detectores de
script/referencia/asset, conteo y porcentajes por tipo, barras, duplicados, títulos distintos,
ordenación, "más útiles", titular, tono y ensamblaje de líneas. Reutiliza `bdots_desk`,
`bdots_text` y `bdots_more`. El endpoint `botInsights` añade el campo `library`, construido desde
`config.skills`, y la sección «Skill library» se muestra en los cuatro clientes.

## Resumen de rutinas (21 helpers + UI en 4 clientes)

`bdots_routines.rs` añade 21 helpers puros que resumen las rutinas de un bot: activas/pausadas,
programadas, próxima ejecución, porcentaje y barra de actividad, mezcla de disparadores (cron
vs evento), ordenación por proximidad, titular y tono. Reutiliza `bdots_cron`, `bdots_text` y
`bdots_activity`. El endpoint `botInsights` añade el campo `routineSummary`, construido desde el
mismo array `routines` que ya calcula el host (sin recalcular), y la sección «Schedule» se muestra
en los cuatro clientes.

## Resumen semanal (21 helpers + UI en 4 clientes)

`bdots_week.rs` añade 21 helpers puros que convierten la actividad diaria en tendencias,
picos, rachas, promedios y un titular en lenguaje natural. Reutiliza `bdots_activity`,
`bdots_text` y `bdots_profile`. El endpoint `botInsights` añade el campo `week` y la sección
«This week» se muestra en el Insights existente de los cuatro clientes.

## Perfil de capacidades (23 helpers + UI en 4 clientes)

`bdots_profile.rs` añade 23 helpers puros que resumen qué puede hacer un bot (computadora,
conectores, skills, rutinas, delegación, web, correo, memoria, agente, modelo, fallbacks,
verificación, modelo de fondo, perfil de navegador y reglas de aprobación). Reutiliza los
validadores de los módulos anteriores en vez de duplicarlos. El endpoint `botInsights` añade el
campo `profile` y la sección «Capabilities» se muestra en el Insights existente de los cuatro
clientes.

## Salud de configuración (23 helpers + UI en 4 clientes)

`bdots_health.rs` aporta 23 helpers puros que validan la configuración de un bot (nombre,
descripción, modelo, carpeta, skills, rutinas, presupuesto, conectores, nombre de secreto y slug)
y producen una puntuación, un titular y una lista ordenada de hallazgos. Reutiliza los validadores
ya portados en lugar de duplicarlos. El endpoint `botInsights` añade el campo `health`, y la
sección «Health» se renderiza dentro de la pantalla de Insights existente en Desktop, iOS,
Android y TUI.

## Timeline de actividad (23 helpers + UI en 4 clientes)

`bdots_activity.rs` añade 23 helpers puros para dar forma a la actividad reciente de un bot:
`kind_icon`, `kind_tone`, `is_write`, `call_detail`, `first_line`, `shorten`, `strip_ansi`,
`call_status`, `group_label`, `group_calls`, `merge_adjacent`, `count_by_kind`, `dominant_kind`,
`error_count`, `activity_headline`, `percent`, `bar_width`, `bucket_day`, `days_between`,
`day_label`, `daily_counts`, `window_start` y `window_span`. Todos tienen tests y todos son
llamados por `activity_view` en el endpoint `botInsights` — no hay `dead_code` nuevo.

El endpoint existing `botInsights` incorpora un campo `activity` con resumen, grupos de pasos,
tira diaria y hasta 12 pasos. La sección «Activity» se renderiza dentro de la pantalla de
Insights que ya existe en Desktop, iOS, Android y TUI, reutilizando sus componentes: no se
añadió ninguna pantalla ni método de host nuevo.

## Pantalla de insights

Los helpers puros ya tienen una superficie real: el método de solo lectura `botInsights {botId}`
(`host/src/api/bot_insights.rs`) devuelve los horarios descritos con `describe_cron`, el modelo y su
proveedor con `provider_of`, el uso diario con `compact_count` y los skills. La pantalla se añadió
a las cuatro clients: pestaña **Insights** en el inspector de Desktop, en los detalles de iPhone y
de Android, y página **Insights** en el TUI (`shift+←→`). Cada cliente solo renderiza lo que el host
calcula; el método no se reporta a analítica porque es una lectura.

## Verificación

- Desktop: `npm run typecheck` limpio; `npm test` → 834 tests, 806 pasan, 4 fallan, 24 omitidos.
  Los 4 fallos son de idioma del sistema (cadenas en español frente a las esperadas en inglés:
  `En cola`/`Queued`, `En marcha`/`Running`, `Escrito`/`Messaged` y un mensaje de timeout), no del
  código cambiado: se ejecutó con el locale español de esta máquina.
- Android: `:design:test :core:test` → BUILD SUCCESSFUL; `TabPillGeometryTest` 2/2.
- Host: `cargo fmt --check` limpio. `cargo test` no pudo completarse en este equipo: `aws-lc-sys`
  necesita NASM (se usó `AWS_LC_SYS_PREBUILT_NASM=1`) y el equipo tenía 0,45 GB libres por builds
  simultáneos de otros agentes, así que la compilación se interrumpió sin mensaje de error. Queda
  pendiente ejecutar `cargo test` en un entorno con memoria suficiente.
- iOS: `xcodebuild` no está disponible en Windows; los tests de CodyncKit no se ejecutaron aquí.

La rama está pusheada como `feat/bdots-50-mejoras` y el PR es
[Thndrr00/Codync#8](https://github.com/Thndrr00/Codync/pull/8) contra `main` (el fork no tiene rama `dev`).
El primer merge con `main` produjo un árbol híbrido inconsistente (el typecheck de Desktop fallaba
porque `main` y esta rama venían de linajes distintos). Se descartó ese merge y se rehízo la rama
como `main` + los commits de esta rama aplicados uno a uno: el PR quedó limpio, sin conflictos, y
`npm run typecheck` vuelve a pasar. En `mcp.rs`, `call` conserva sus cinco argumentos para el CLI y
`call_with_pruner` sirve al puente MCP. La tarjeta de `ask_user` lleva `askUserTool` para no
confundirse con las tarjetas `question` que `main` ya usa en `external/question.rs`. El PR sigue en
`UNSTABLE` (sin conflictos; checks pendientes).

Sobre más adiciones: el nodo de aprobación humana en flujos y el correo entrante por bot son
demasiado grandes para escribirlos sin poder compilar el host aquí; los skills de diseño son solo
documentación y no aportan comportamiento. Se dejan fuera a propósito, no por olvido.

## Puente iOS

`HostClient+Bdots.swift` reutiliza el único `HostClient.call` que ya usan las demás pantallas —no
crea un segundo transporte— para `listBotApps`, `createAppLink`, `revokeAppLink`, `deleteBotApp`,
`listOpenItems`, `listBookmarks`, `deleteBookmark` y `appRequest`. Android ya tenía el mismo puente
en `AppStoreBots.kt`/`MiniAppBridge.kt`; el de iOS usa los modelos de `BdotsModels.swift` y conserva
la CSP que devuelve el host para la vista web. Tests de decodificación en `BdotsBridgeTests.swift`
(sin ejecutar aquí: no hay `xcodebuild` en Windows).
