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
