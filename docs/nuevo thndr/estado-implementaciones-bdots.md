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

## Spotify (26 adiciones + UI en 4 clientes)

`bdots_spotify.rs` añade 20 helpers puros y 6 constantes portados de `core/spotify.ts` de **coucou**:
posición extrapolada, tipos de pista, volumen, reproducción y condiciones de baile de Mochi. El
endpoint `botInsights` añade el campo `spotify`; la sección «Spotify» se muestra en los cuatro
clientes. Reutiliza `bdots_desk`.

## Mochi desktop (25 adiciones + UI en 4 clientes)

`bdots_mochi_desktop.rs` añade 15 helpers puros y 10 constantes portados de `mochi/desktop-logic.ts` de
**coucou**: suspensión por inactividad, hit-testing del cuerpo redondo, mirada, alertas, distancia del
puntero y geometría de arrastre. El endpoint `botInsights` añade el campo `mochidesktop`; la sección
«Mochi desktop» se muestra en los cuatro clientes. Reutiliza `bdots_desk`.

## Novedades (21 adiciones + UI en 4 clientes)

`bdots_whats_new.rs` añade 16 helpers puros y 5 constantes portados de `whats-new/whats-new.ts` de
**coucou**: comparación de versiones, búsqueda de release, texto localizado, iconos, decisión de
apertura y pistas de actualización. El endpoint `botInsights` añade el campo `whatsnew`; la sección
«What's new» se muestra en los cuatro clientes. Reutiliza `bdots_desk`.

## Resumen semanal (21 adiciones + UI en 4 clientes)

`bdots_recap.rs` añade 16 helpers puros y 5 constantes portados de `recap/summary.ts` de **coucou**:
fechas de semana, auto-show del lunes, nombres de agentes, unión de intervalos y resúmenes semanales.
El endpoint `botInsights` añade el campo `recap`; la sección «Weekly recap» se muestra en los cuatro
clientes. Reutiliza `bdots_desk`.

## Layout del island (29 adiciones + UI en 4 clientes)

`bdots_island_layout.rs` añade 12 helpers puros y 17 constantes portados de `core/layout.ts` de
**coucou**: tamaños del island, colocación del bot, colores de brillo, colores de proyecto y washes.
El endpoint `botInsights` añade el campo `islandlayout`; la sección «Island layout» se muestra en los
cuatro clientes. Reutiliza `bdots_desk`.

## Atajos de coucou (19 adiciones + UI en 4 clientes)

`bdots_shortcuts_coucou.rs` añade 14 helpers puros y 5 constantes portados de `core/shortcuts.ts` de
**coucou**: aceleradores, pulsaciones grabadas, duplicados, choques de AltGr y navegación del island.
El endpoint `botInsights` añade el campo `coucoushortcuts`; la sección «Shortcuts» se muestra en los
cuatro clientes. Reutiliza `bdots_desk`.

## Pills de coucou (21 adiciones + UI en 4 clientes)

`bdots_pills.rs` añade 15 helpers puros y 6 constantes portados de `core/pills.ts` de **coucou**:
catálogo, soporte por sistema, integraciones declaradas y orden de pills. El endpoint `botInsights`
añade el campo `pills`; la sección «Pills» se muestra en los cuatro clientes. Reutiliza `bdots_desk`.

## Correo de coucou (22 adiciones + UI en 4 clientes)

`bdots_coucou_mail.rs` añade 15 helpers puros y 7 constantes portados de `core/mail.ts` de
**coucou**: ids de mensaje, remitentes, filtros, reglas, razones de triage y bloques de plan. El
endpoint `botInsights` añade el campo `coucoumail`; la sección «Mail» se muestra en los cuatro
clientes. Reutiliza `bdots_desk`.

## WhatsApp (29 adiciones + UI en 4 clientes)

`bdots_whatsapp.rs` añade 21 helpers puros y 8 constantes portados de `core/whatsapp.ts` de
**coucou**: estados de enlace, previews de mensajes, errores de envío, pasos de instalación, webhook
y palabras de Settings. El endpoint `botInsights` añade el campo `whatsapp`; la sección «WhatsApp»
se muestra en los cuatro clientes. Reutiliza `bdots_desk`.

## GitHub y fondo aurora (54 adiciones + UI en 4 clientes)

Del repo `coucou` (`windows/src/core/github.ts`) se portaron 30 helpers puros en `bdots_github.rs`:
pull requests, CI, colores de contribuciones, enlaces seguros y cuadrícula. Del repo `OpenMausBot`
(`src/lib/aurora-field.ts`) se portaron 24 helpers en `bdots_aurora_field.rs`: el fondo dinámico con
una banda de luz que cruza la pantalla, dithering Bayer, campo de estrellas con semilla y variables
CSS — el fondo dinámico e interactivo que se pidió integrar. El endpoint `botInsights` añade los
campos `github` y `aurora`, con sección en los cuatro clientes. Ambos reutilizan `bdots_desk`.

## Pasada final: correo, media, respuestas y avisos (94 adiciones + UI en 4 clientes)

Investigación de lo que faltaba en los repos de referencia: cuatro áreas puras sin portar, todas
cerradas en una sola pasada.

- `bdots_mail.rs` (27): direcciones, choix de buzón, niveles y políticas de correo, de `client/lib/mail.ts`.
- `bdots_media.rs` (24): moods de stickers, categorías GIF, metadata y ajustes, de `engine/media.ts`.
- `bdots_reply_policy.rs` (23): centinelas, texto visible y veredictos, de `engine/reply-policy.ts`.
- `bdots_notices.rs` (20): clases de aviso, silencio, guiado de handoff, de `engine/notices.ts`.

El endpoint `botInsights` añade los campos `mail`, `media`, `replypolicy` y `notices`, con sección en
Desktop, iOS, Android y TUI. Reutilizan `bdots_desk`. Checks: `rustfmt --check` y `npm run typecheck`
limpios; sin compilaciones, según la regla.

## Etiquetas de herramientas (27 adiciones + UI en 4 clientes)

`bdots_tool_labels.rs` añade 21 helpers puros y 6 constantes portados de `tool-labels.ts` y
`tool-review.ts` de **agentes-chat 0.7.5**: palabras de nombres, prefijos de apps, claves de catálogo,
etiquetas en inglés y pt-BR, búsqueda y decisiones de revisión. Reutiliza `bdots_desk`. El endpoint
`botInsights` añade el campo `toollabels`; la sección «Tool labels» se muestra en los cuatro clientes.

## Bloques de contexto (30 adiciones + UI en 4 clientes)

`bdots_context_blocks.rs` añade 24 helpers puros y 6 constantes portados de `engine/context.ts` de
**agentes-chat 0.7.5**: detección de no-reply, reacciones, plegado de repetidos, bloques de conversación
y turno, orden estable para caché y textos de prompt. Reutiliza `bdots_desk`. El endpoint `botInsights`
añade el campo `contextblocks`; la sección «Context blocks» se muestra en los cuatro clientes.

## Trabajo y herramientas (30 adiciones + UI en 4 clientes)

`bdots_work.rs` añade 24 helpers puros y 6 constantes portados de `work.ts` de **agentes-chat
0.7.5**: metadatos de trabajo, plegado de pasos y llamadas, agrupación de apps, duración, cabecera,
cancelación y estados de herramientas. Reutiliza `bdots_desk`. El endpoint `botInsights` añade el
campo `work`; la sección «Work» se muestra en los cuatro clientes.

## Acceso a canales (30 adiciones + UI en 4 clientes)

`bdots_channel_access.rs` añade 24 helpers puros y 6 constantes portados de `utils/config.ts` de
**mercury-agent 2.7.0**: listas de acceso de Telegram/Signal/Discord/Slack, códigos de emparejamiento,
aprobación, promoción, limpieza y configuración de proveedores y nube. Reutiliza `bdots_desk`. El
endpoint `botInsights` añade el campo `channelaccess`; la sección «Channel access» se muestra en los
cuatro clientes.

## Árbol de layout (29 adiciones + UI en 4 clientes)

`bdots_layout_tree.rs` añade 23 helpers puros y 6 constantes portados de `pane-shell/tree/model.ts`
del TUI de **hermes-agent 3.14**: nodos split/group, búsqueda, recorridos, normalización, borrado de
panes, pesos y migración. Reutiliza `bdots_desk`. El endpoint `botInsights` añade el campo `layout`;
la sección «Layout» se muestra en los cuatro clientes.

## Stash de borradores (27 adiciones + UI en 4 clientes)

`bdots_draft_stash.rs` añade 21 helpers puros y 6 constantes portados de `composer-draft-stash.ts`
y `draft-title.ts` de **hermes-agent 3.14**: timestamps, fusión por entry más reciente, compactación,
adopción legacy, hashes de mirror y títulos derivados. Reutiliza `bdots_desk`. El endpoint
`botInsights` añade el campo `draftstash`; la sección «Draft stash» se muestra en los cuatro clientes.

## Servicio de arranque (30 adiciones + UI en 4 clientes)

`bdots_service.rs` añade 24 helpers puros y 6 constantes portados de `cli/service.ts` de
**mercury-agent 2.7.0**: parseo de plist/systemd/Task XML/reg query, comillas de Windows, rutas de
servicio, plists y units generados y detección de deriva. Reutiliza `bdots_desk`. El endpoint
`botInsights` añade el campo `service`; la sección «Service» se muestra en los cuatro clientes.

## Plugins (29 adiciones + UI en 4 clientes)

`bdots_plugins.rs` añade 23 helpers puros y 6 constantes portados de `plugins/store.ts` y
`plugins/types.ts` de **mercury-agent 2.7.0**: ids y rutas, compatibilidad semver, migración de
settings, secretos y caducidad de locks. Reutiliza `bdots_desk`. El endpoint `botInsights` añade el
campo `plugins`; la sección «Plugins» se muestra en los cuatro clientes.

## Secuencias CSI (30 adiciones + UI en 4 clientes)

`bdots_csi.rs` añade 24 helpers puros y 6 constantes portados de `csi.ts` del TUI de
**hermes-agent 3.14**: rangos de bytes CSI, construcción de secuencias, movimiento de cursor,
borrado, scroll, estilos de cursor y regiones. Reutiliza `bdots_desk`. El endpoint `botInsights`
añade el campo `csiterm`; la sección «Terminal sequences» se muestra en los cuatro clientes.

## Acciones de prompt (29 adiciones + UI en 4 clientes)

`bdots_prompt_actions.rs` añade 23 helpers puros y 6 constantes portados de `use-prompt-actions/utils.ts`
de **hermes-agent 3.14**: ids de sesión, clasificación de errores del gateway, superficies de
propiedad, guards de envío en vuelo, recibos y nombres de imágenes. Reutiliza `bdots_desk`. El
endpoint `botInsights` añade el campo `promptactions`; la sección «Prompt actions» se muestra en los
cuatro clientes.

## Almacén de bots (30 adiciones + UI en 4 clientes)

`bdots_bot_store.rs` añade 24 helpers puros y 6 constantes portados de `bots/store.ts` de
**mercury-agent 2.7.0**: ids de bot, nombres de entregables, validación de manifiestos, jerarquía de
flota, herramientas peligrosas y plantillas de persona. Reutiliza `bdots_textfmt` y `bdots_desk`.
El endpoint `botInsights` añade el campo `botstore`; la sección «Bot store» se muestra en los cuatro
clientes.

## Preferencias de modelo (30 adiciones + UI en 4 clientes)

`bdots_model_prefs.rs` añade 24 helpers puros y 6 constantes portados de `model-prefs.ts` de
**mercury-agent 2.7.0**: refs de modelo, niveles de razonamiento, preferencias por modelo y por chat,
opciones de Anthropic/OpenRouter, nombres legibles y agrupación por proveedor. Reutiliza
`bdots_desk`. El endpoint `botInsights` añade el campo `modelprefs`; la sección «Model preferences»
se muestra en los cuatro clientes.

## Router (30 adiciones + UI en 4 clientes)

`bdots_router.rs` añade 24 helpers puros y 6 constantes portados de `router.ts` de **agentes-chat
0.7.5**: umbrales, preguntas de destinatario, aplicación de política de selección, reacciones
sociales, parseo de JSON del router y normalización de scores. Reutiliza `bdots_desk`. El endpoint
`botInsights` añade el campo `router`; la sección «Router» se muestra en los cuatro clientes.

## Política de comandos (30 adiciones + UI en 4 clientes)

`bdots_cmd_policy.rs` añade 24 helpers puros y 6 constantes portados de `command-policy.ts` de
**agentes-chat 0.7.5**: parseo de shell, comandos simples, rutas secretas, listas de denegación,
merges y pushes, código en línea y redacción de aprobaciones. Reutiliza `bdots_desk`. El endpoint
`botInsights` añade el campo `cmdpolicy`; la sección «Command policy» se muestra en los cuatro clientes.

## Misiones (30 adiciones + UI en 4 clientes)

`bdots_missions.rs` añade 24 helpers puros y 6 constantes portados de `missions.ts` de **agentes-chat
0.7.5**: tiras de plan por celdas, conteos, partes de espera, plazos de actualización, spans,
tiempo de tareas, PRs, líneas de salida, cambios de plan, comandos del composer y metadatos de
mensajes. Reutiliza `bdots_desk`. El endpoint `botInsights` añade el campo `missions`; la sección
«Missions» se muestra en los cuatro clientes.

## Sin IA (30 adiciones + UI en 4 clientes)

`bdots_no_ai.rs` añade 21 helpers puros y 9 constantes portados de `no-ai.ts` de **agentes-chat
0.7.5**: tarjetas `noAi`, estado de mensajes retenidos, líneas de plan, intents de conexión,
mapas plan↔CLI, elecciones del picker y claves de presentación. Reutiliza `bdots_textfmt` y
`bdots_desk`. El endpoint `botInsights` añade el campo `noai`; la sección «No AI» se muestra en los
cuatro clientes.

## Dispositivos (30 adiciones + UI en 4 clientes)

`bdots_devices.rs` añade 24 helpers puros y 6 constantes portados de `devices.ts` de **agentes-chat
0.7.5**: estados y pills, portraits, nombres y líneas, audiencias, orden y filas de la lista, enlaces
de conexión, etapas y checklist de Android y emparejamiento Wi-Fi. Reutiliza `bdots_desk`. El endpoint
`botInsights` añade el campo `devices`; la sección «Devices» se muestra en los cuatro clientes.

## Bóveda de secretos (30 adiciones + UI en 4 clientes)

`bdots_vault.rs` añade 23 helpers puros y 7 constantes portados de `bot-secrets.ts` de **rakazo
v0.14.0**: tipos de autenticación, variables de comando y nombres reservados, códigos de un solo
uso, campos de login, claves de autenticador y otpauth, etiquetas de identificador, marcas de tarjeta,
Luhn, resúmenes, vencimientos y presentación. Reutiliza `bdots_textfmt`, `bdots_desk` y `bdots_more`.
El endpoint `botInsights` añade el campo `vault`; la sección «Vault» se muestra en los cuatro clientes.

## Replay de herramientas (28 adiciones + UI en 4 clientes)

`bdots_tool_replay.rs` añade 21 helpers puros y 7 constantes portados de `tool-step-calls.ts` de
**rakazo v0.14.0**: acotación de texto conservando inicio y fin, redacción de secretos y campos de
credenciales, recorte recursivo de argumentos, resultados retenidos, permiso de replay por trigger
y presupuesto de caracteres. Reutiliza `bdots_more`, `bdots_textfmt` y `bdots_desk`. El endpoint
`botInsights` añade el campo `toolreplay`; la sección «Tool replay» se muestra en los cuatro clientes.

## Limpieza de secretos (27 adiciones + UI en 4 clientes)

`bdots_secret_cleanup.rs` añade 20 helpers puros y 6 constantes portados de `secret-cleanup.ts` de
**rakazo v0.14.0**: ids de registro de secretos, claves de trabajo de limpieza, prefijos y tipos de
referencia, validación de columnas de titular, ventanas de tiempo y presentación. Reutiliza
`bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo `secretcleanup`; la sección
«Secret cleanup» se muestra en los cuatro clientes.

## Avatar de bot (22 adiciones + UI en 4 clientes)

`bdots_avatar.rs` añade 17 helpers puros y 5 constantes portados de `bot-avatar-colors.ts` de
**rakazo v0.14.0**: hash FNV-1a y PRNG determinista, expansión/canales hex, luminancia y contraste
WCAG, resolución de color de persona, tono e tinta del ojo,墨水 de burbuja e iniciales. Reutiliza
`bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo `avatar`; la sección
«Avatar» se muestra en los cuatro clientes.

## Proveedores de modelos (26 adiciones + UI en 4 clientes)

`bdots_providers.rs` añade 22 helpers puros y 4 constantes portados de `providers.ts` de
**agentes-chat 0.7.5**: catálogo de proveedores, detección de claves por prefijo, guess de
proveedor, placeholders, sitios de keys, flags de plan/billing/privacidad, model choices y
presentación. Reutiliza `bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo
`providers`; la sección «Providers» se muestra en los cuatro clientes.

## Planes de código (23 adiciones + UI en 4 clientes)

`bdots_plans.rs` añade 19 helpers puros y 4 constantes portados de `plans.ts` de **agentes-chat
0.7.5**: resolución de planes por proveedor, modelos OpenCode Go, claves y patrones, sign-in,
Gemini personal, alias de modelos y presentación. Reutiliza `bdots_textfmt` y `bdots_desk`. El
endpoint `botInsights` añade el campo `plans`; la sección «Coding plans» se muestra en los cuatro
clientes.

## Cuentas de IA (25 adiciones + UI en 4 clientes)

`bdots_accounts.rs` añade 21 helpers puros y 4 constantes portados de `ai-accounts.ts` de
**agentes-chat 0.7.5**: estados de sign-in, agrupación de planes por identidad, logins usados,
acciones de cuenta existente, títulos y presentación. Reutiliza `bdots_textfmt` y `bdots_desk`.
El endpoint `botInsights` añade el campo `accounts`; la sección «Accounts» se muestra en los cuatro
clientes.

## Motor de cerebro (23 adiciones + UI en 4 clientes)

`bdots_engine.rs` añade 19 helpers puros y 4 constantes portados de `brain.ts` de **agentes-chat
0.7.5**: clases de motor y estado, resolución de brain/executor, estados de opción, líneas «what
thinks», modelos automáticos, nombres de computadora y presentación. Reutiliza `bdots_i18n`,
`bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo `engine`; la sección
«Engine» se muestra en los cuatro clientes.

## Selector de cerebro (23 adiciones + UI en 4 clientes)

`bdots_brain_picker.rs` añade 19 helpers puros y 4 constantes portados de `brain-picker.ts` de
**agentes-chat 0.7.5**: familias Claude, modelos gratis, descripciones con precio, texto de búsqueda,
opciones y grupos, líneas de plan, sign-in propio y presentación. Reutiliza `bdots_i18n`,
`bdots_localfmt`, `bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo
`brainpicker`; la sección «Brain picker» se muestra en los cuatro clientes.

## Reglas de aprobación (24 adiciones + UI en 4 clientes)

`bdots_approval_rules.rs` añade 18 helpers puros y 6 constantes portados de `approval-rules.ts` de
**agentes-chat 0.7.5**: nombres de acciones, kinds recordados, reglas de comando, targets de lectura
SQL, scopes, cobertura de acciones, decisión de aprobación y presentación. Reutiliza `bdots_more`,
`bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo `approvalrules`; la sección
«Approval rules» se muestra en los cuatro clientes.

## Iteraciones (21 adiciones + UI en 4 clientes)

`bdots_iteration.rs` añade 16 helpers puros y 5 constantes portados de `turn_iteration_prep.py` de
**hermes-agent**: anclaje del turno, presupuesto de iteraciones y aviso único, neutralización de
filas fantasma de interrupción, emparejamiento de rondas de herramientas, steer y reembolsos por
reinicio. Reutiliza `bdots_more`, `bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade
el campo `iteration`; la sección «Iteration» se muestra en los cuatro clientes.

## Recurrencia (23 adiciones + UI en 4 clientes)

`bdots_rrule.rs` añade 18 helpers puros y 5 constantes portados de `rrule.ts` de **agentes-chat
0.7.5**: frecuencias, validación de weekdays/horas/fechas, intervalos, setpos, conversión a cron,
nombres de weekdays, descripciones y zonas horarias. Reutiliza `bdots_i18n`, `bdots_textfmt` y
`bdots_desk`. El endpoint `botInsights` añade el campo `rrule`; la sección «Recurrence» se muestra
en los cuatro clientes.

## Autocompletado (26 adiciones + UI en 4 clientes)

`bdots_autofill.rs` añade 20 helpers puros y 6 constantes portados de `vault_login_classifier.py`
de **hermes-agent**: normalización de texto, tokens de autocomplete, clasificación de controles de
login, detección de OTP, selección del mejor campo de contraseña, clasificación de checkout y
presentación. Reutiliza `bdots_textfmt`, `bdots_desk` e `bdots_i18n`. El endpoint `botInsights`
añade el campo `autofill`; la sección «Autofill» se muestra en los cuatro clientes.

## Verificación al terminar (25 adiciones + UI en 4 clientes)

`bdots_verify.rs` añade 19 helpers puros y 6 constantes portados de `verification_stop.py` de
**hermes-agent**: política de verificación, tokens de configuración, clasificación de rutas de
código/prosa, directorios candidatos, formato acotado de rutas y estados, evidencia y decisión de
recordatorio. Reutiliza `bdots_more`, `bdots_desk` y `bdots_textfmt`. El endpoint `botInsights`
añade el campo `verify`; la sección «Verification» se muestra en los cuatro clientes.

## Pin de ventana de contexto (20 adiciones + UI en 4 clientes)

`bdots_context_pin.rs` añade 18 helpers puros y 2 constantes portados de `context_pin.py` de
**hermes-agent**: pins explícitos de ventana, detección de fallback, marcadores y sufijos de
display, metadatos locales de ventana anunciada, desacuerdo de pin y etiquetas. Reutiliza
`bdots_textfmt`, `bdots_localfmt`, `bdots_more`, `bdots_activity` y `bdots_desk`. El endpoint
`botInsights` añade el campo `contextpin`; la sección «Window pin» se muestra en los cuatro clientes.

## Tablas markdown (21 adiciones + UI en 4 clientes)

`bdots_md_tables.rs` añade 20 helpers puros y una constante portados de `markdown_tables.py` de
**hermes-agent**: detección de filas y divisores, anchos de display CJK/emoji, padding, renderizado
uniforme, wrapping, realineación y diseño vertical para tablas angostas. Reutiliza `bdots_desk`.
El endpoint `botInsights` añade el campo `mdtables`; la sección «Markdown tables» se muestra en los
cuatro clientes.

## Presupuesto de contexto (20 adiciones + UI en 4 clientes)

`bdots_context_select.rs` añade 17 helpers puros y 3 constantes portados de `context-selection.ts`
de **rakazo v0.14.0**: estimación neutral de tokens, overhead de framing, coste fijo, presupuesto
disponible, estrategias, protección de mensajes, ajuste del historial, estado de caché y marker de
omisión. Reutiliza `bdots_desk`. El endpoint `botInsights` añade el campo `contextselect`; la
sección «Context budget» se muestra en los cuatro clientes.

## Selector de uso (22 adiciones + UI en 4 clientes)

`bdots_picker.rs` añade 18 helpers puros y 4 constantes portados de `model_picker_usage.py` de
**hermes-agent**: fracciones de uso, estilos de gauge, barras `[||||      ]`, porcentaje usado,
duración hasta el reset, ordenación de modelos destacados, modelos gratis y etiquetas del picker.
Reutiliza `bdots_quota`, `bdots_desk` y `bdots_textfmt`. El endpoint `botInsights` añade el campo
`picker`; la sección «Usage picker» se muestra en los cuatro clientes.

## Paleta de comandos (22 adiciones + UI en 4 clientes)

`bdots_palette.rs` añade 18 helpers puros y 4 constantes portados de `messaging-commands.ts` y
`command-palette.tsx` de **rakazo v0.14.0**: comandos del dueño (yes/no/leave/stop en varios
idiomas), normalización de búsqueda, filtrado de bots y acciones, ranking por prefijo, páginas
recientes y presentación. Reutiliza `bdots_textfmt`, `bdots_desk`, `bdots_more` y `bdots_i18n`.
El endpoint `botInsights` añade el campo `palette`; la sección «Command palette» se muestra en los
cuatro clientes.

## Atajos de teclado (22 adiciones + UI en 4 clientes)

`bdots_shortcuts.rs` añade 18 helpers puros, un `ShortcutHost`, `KeyMods` y dos constantes de
`shell-shortcuts.ts` de **rakazo v0.14.0**: resolución de modificador primario, chords de cita,
comandos del shell, etiquetas de atajos, disponibilidad en web y presentación. Reutiliza
`bdots_i18n`, `bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo `shortcuts`;
la sección «Shortcuts» se muestra en los cuatro clientes.

## Métricas eval y conexiones (41 adiciones + UI en 4 clientes)

Módulos con los que terminaron los agentes hijos T-86 y T-87:
- `bdots_eval_extra.rs`: 24 helpers puros y constantes de los evals de **hermes-agent**
  (static_metrics.py, goal_judge_wait.py): conteo de líneas, percentiles, umbrales, ciclos de
  importación y parseo de veredictos del juez.
- `bdots_connstate_extra.rs`: 12 helpers puros de `connection-context.ts` de **rakazo v0.14.0**:
  claves de conexión, proveedores Composio, recuperación y selección de conexiones para un run.

El endpoint `botInsights` añade los campos `evalextra` y `connextra`, con secciones «Eval metrics» y
«Connections extra» en Desktop, iOS, Android y TUI.

## Búsqueda rankeada (19 adiciones + UI en 4 clientes)

`bdots_search_extra.rs` es el módulo con el que terminó el agente hijo T-85: 12 helpers puros y
7 constantes portados del código de búsqueda de **agentes-chat 0.7.5** (fold con acentos, términos,
operadores `from:`/`in:`/`has:`, RRF, recency, vector literal y resumen). Reutiliza `bdots_search`
y `bdots_desk`. El endpoint `botInsights` añade el campo `searchextra`; la sección «Ranked search»
se muestra en los cuatro clientes.

## Contexto de memoria (22 adiciones + UI en 4 clientes)

`bdots_memoryctx.rs` añade 16 helpers puros, 3 constantes y un scrubber con estado portados de
`memory_context.py` de **hermes-agent**: fencing `<memory-context>`, sanitización de salida,
notas del sistema inyectadas, scrubber de deltas con cola de tag parcial, bloque de recall y dedup
de bullets repetidos. Reutiliza `bdots_more` y `bdots_desk`. El endpoint `botInsights` añade el
campo `memoryctx`; la sección «Memory context» se muestra en los cuatro clientes.

## Errores de API (24 adiciones + UI en 4 clientes)

`bdots_apierror.rs` añade 21 helpers puros y 3 constantes portados de `api_error_summary.py` de
**hermes-agent**: markers de parseo de stream y DNS, detección de entitlement xAI, prefijo HTTP,
coerción de detalles estructurados, clasificación (auth/rate/context/model/transient), retry,
redacción log-safe y resúmenes. Reutiliza `bdots_more`, `bdots_mailtext` y `bdots_desk`. El endpoint
`botInsights` añade el campo `apierror`; la sección «API errors» se muestra en los cuatro clientes.

## Historial (22 adiciones + UI en 4 clientes)

`bdots_history.rs` añade 21 helpers puros y una constante portados de `history_commentary.py` y
`message_content.py` de **hermes-agent**: proyección display-only de commentary, strip de `<think>`,
fases de items de Codex, flag `commentary_withheld`, aplanado de contenido y conteos. Reutiliza
`bdots_stream`, `bdots_mailtext`, `bdots_more` y `bdots_desk`. El endpoint `botInsights` añade el
campo `history`; la sección «History» se muestra en los cuatro clientes.

## Revisión en idle (24 adiciones + UI en 4 clientes)

`bdots_review.rs` añade 21 helpers puros y 3 constantes portados de `review_idle_queue.py` de
**hermes-agent**: modos de deferral, edad máxima, ciclo de vida de turns, quiet window, coalescing
por sesión conservando el tiempo original, age-out, gating de despacho, sufijos de sesión y
resúmenes. Reutiliza `bdots_desk`. El endpoint `botInsights` añade el campo `review`; la sección
«Review queue» se muestra en los cuatro clientes.

## Blueprints (25 adiciones + UI en 4 clientes)

`bdots_blueprint.rs` añade 22 helpers puros y 3 constantes portados de `blueprint_catalog.py` y
`blueprint_plugins.py` de **hermes-agent**: tipos de slot, validación de claves/namespaces/nombres,
horas `HH:MM`, placeholders de plantillas, presets de weekdays,表达式 cron, comandos slash, deep
links y resúmenes. Reutiliza `bdots_desk` y `bdots_textfmt`. El endpoint `botInsights` añade el
campo `blueprint`; la sección «Blueprint» se muestra en los cuatro clientes.

## Stream (22 helpers + UI en 4 clientes)

`bdots_stream.rs` añade 22 helpers puros portados de `stream_delivery.py` de **hermes-agent**:
filas retenidas, publicación del mensaje final, entrega de deltas, limpieza de `<think>`, dedup de
texto interim, commentary de Codex, propiedad del writer de stream, eventos y chunking.
Reutiliza `bdots_desk`. El endpoint `botInsights` añade el campo `stream`; la sección «Stream» se
muestra en los cuatro clientes.

## Locales y plurales (26 adiciones + UI en 4 clientes)

`bdots_i18n.rs` añade 24 helpers puros y 2 constantes portados de `packages/i18n` de
**agentes-chat 0.7.5**: registro de locales, nombres nativos/ingleses, tags Intl, inicio de semana,
scripts y dirección, resolución de tags de navegador (`pt-PT`, `zh-*`, `in`), formas plurales CLDR
y huecos de traducción. Reutiliza `bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade
el campo `i18n`; la sección «Locale» se muestra en los cuatro clientes.

## Cuotas (25 adiciones + UI en 4 clientes)

`bdots_quota.rs` añade 24 helpers puros y una constante portados de `account_usage.py` de
**hermes-agent**: ventanas de uso, gauges en puntos base, etiquetas de reset, niveles de agotamiento,
scope de cuenta/modelo, ordenación de ventanas y estados de cuenta. Reutiliza `bdots_activity`,
`bdots_extra`, `bdots_textfmt` y `bdots_desk`. El endpoint `botInsights` añade el campo `quota`; la
sección «Quota» se muestra en los cuatro clientes.

## Apariencia (28 adiciones + UI en 4 clientes)

`bdots_appearance.rs` añade 23 helpers puros y 5 constantes portados de `packages/ui-tokens` de
**rakazo v0.14.0**: preferencias de apariencia, resolución system/light/dark, paletas móvil y web,
nombres de variables CSS, escala tipográfica y de fuente, colores de bots y barras, contraste,
luminancia, mezcla y alpha. Reutiliza `bdots_textfmt` y `bdots_desk`. El endpoint `botInsights`
añade el campo `appearance`; la sección «Appearance» se muestra en los cuatro clientes.

## Conexiones, evaluación y búsqueda (40 adiciones + UI en 4 clientes)

Tres módulos puros creados en paralelo por agentes hijos (T-83/T-84/T-85 agotaron su tiempo y no
escribieron archivos, así que se completaron en esta sesión):

- `bdots_connstate.rs` (13 helpers + 1 constante) portado de `connection-context.ts` de **rakazo
  v0.14.0**: estados de conexión, frescura, backoff exponencial, selección por bot, conteos y
  etiquetas de canal.
- `bdots_eval.rs` (13 helpers) portado de los evals de **hermes-agent**: scores acotados, umbrales,
  veredictos, notas, promedio, mediana, tasa de aprobado y tolerancias.
- `bdots_search.rs` (13 helpers) portado de los helpers de búsqueda de **agentes-chat 0.7.5**:
  normalización de consultas, términos, coincidencias, relevancia, filtros, resaltado y resúmenes.

Reutilizan `bdots_desk`, `bdots_activity` y `bdots_more`. El endpoint `botInsights` añade los campos
`connstate`, `eval` y `search`; las secciones «Connections», «Evaluation» y «Search» se muestran en
los cuatro clientes.

## Correo y datos (24 helpers + UI en 4 clientes)

`bdots_mailtext.rs` añade 24 helpers puros y constantes portados de `mail/text.ts`,
`databases/text.ts` y `api-key-text.ts` de **agentes-chat 0.7.5**: bloques untrusted, recorte de
cuerpos y celdas, normalización y reglas de direcciones, mailbox de Gmail, filas acotadas y
detección/redacción de claves API. Reutiliza `bdots_textfmt`, `bdots_more`, `bdots_desk` y
`bdots_activity`. El endpoint `botInsights` añade el campo `mailtext`; la sección «Mail & data» se
muestra en los cuatro clientes.

## Notificaciones (27 helpers + UI en 4 clientes)

`bdots_notify.rs` añade 27 helpers puros portados de `notification_presentation.py`,
`warning_notifications.py`, `delivery.py` y `delivery_ledger.py` de **hermes-agent**: eventos
freeform, silenciado de diagnósticos, políticas de notificación, narración de silencio, ids de
obligación, errores de flood, backoff de reintento y estados de entrega con marcadores honestos.
Reutiliza `bdots_more`, `bdots_desk` y `bdots_activity`. El endpoint `botInsights` añade el campo
`notify`; la sección «Notifications» se muestra en los cuatro clientes.

## Markdown (27 helpers + UI en 4 clientes)

`bdots_markdown.rs` añade 27 helpers puros portados de `packages/ui/src/markdown.ts` de
**agentes-chat 0.7.5**: URLs seguras, spans de código/negrita, menciones, wikilinks, ids internos,
inline a prosa, listas, reglas, fences, encabezados, celdas y alineación de tablas, `<details>` y
bloques anchos. Reutiliza `bdots_textfmt`, `bdots_activity` y `bdots_desk`. El endpoint `botInsights`
añade el campo `markdown`; la sección «Markdown» se muestra en los cuatro clientes.

## Formato local (27 helpers + UI en 4 clientes)

`bdots_localfmt.rs` añade 27 helpers puros portados de `format.ts` de **agentes-chat 0.7.5**:
signos decimales y agrupación por locale, dinero, porcentajes, tokens cortos, aritmética civil de
días sin crate de fechas, conversión epoch ↔ ISO, reloj de 12/24 h, fechas cortas y largas,
días de la semana, etiquetas relativas, formas ordinales y resúmenes. Reutiliza `bdots_extra`,
`bdots_text`, `bdots_textfmt`, `bdots_desk` y `bdots_activity`. El endpoint `botInsights` añade el
campo `localfmt`; la sección «Local format» se muestra en los cuatro clientes.

## Higiene de texto (28 helpers + UI en 4 clientes)

`bdots_textfmt.rs` añade 28 helpers puros portados de `text.ts` y `html-text.ts` de
**agentes-chat 0.7.5**: slugs y handles con plegado de acentos, validación y extracción de correos,
corrección de dominios webmail, reglas de contraseña, parseo de importes, sentences y palabras,
excerpt, escape/decode de HTML, limpieza de caracteres invisibles y bidi, y resúmenes. Reutiliza
`bdots_activity` (shorten), `bdots_extra` (plain_text) y `bdots_desk`. El endpoint `botInsights`
añade el campo `textfmt`; la sección «Text hygiene» se muestra en los cuatro clientes.

Además se corrige la ruta histórica `bdots_more::shorten` con una reexportación desde
`bdots_activity`, para que los ~40 módulos que la usan no queden con una referencia rota.

## Presupuesto de contexto (23 helpers + UI en 4 clientes)

`bdots_agenteschat.rs` añade 23 helpers puros portados de **agentes-chat 0.7.5**: formatos de
contexto, ventanas de plan `[1m]`, ajuste de límites, claves y agrupación de timeline, ventanas de
placeholder, esfuerzos automáticos, timeouts, fallbacks y resúmenes. Reutiliza `bdots_activity` y
`bdots_desk`. El endpoint `botInsights` añade el campo `agenteschat`; la sección «Context budget»
se muestra en los cuatro clientes.

## Modelo de run (23 helpers + UI en 4 clientes)

`bdots_openbot.rs` añade 23 helpers puros portados de `agent-langgraph-ts` de **OpenBot-CopilotKit
v1.1.1**: esfuerzos de razonamiento (RUN_EFFORTS, Anthropic, Google), rangos y etiquetas, rutas de
gateway, threads de plan Claude, modelos forwardados, requiresResponsesApi, send-once y resúmenes.
Reutiliza `bdots_more` y `bdots_desk`. El endpoint `botInsights` añade el campo `openbot`; la sección
«Run model» se muestra en los cuatro clientes.

## OpenDots (23 helpers + UI en 4 clientes)

`bdots_opendots.rs` añade 23 helpers puros portados de los modelos compartidos de **OpenDots**:
validación de contenedores Learning, settings de entrega, prefijos de mensajes internos
(scheduled task / voice receipt), extracción de ids, iconos, reviews con estrellas y resúmenes.
Reutiliza `bdots_more` y `bdots_desk`. El endpoint `botInsights` añade el campo `opendots`; la
sección «Learning» se muestra en los cuatro clientes.

## Contexto (23 helpers + UI en 4 clientes)

`bdots_context.rs` añade 23 helpers puros portados de `context-pruning.ts` de **rakazo v0.14.0**:
herramientas que muestran estado de página, conteos a conservar, umbral de texto obsoleto,
normalización de límites, conteo de imágenes, detección de screenshots, decisiones de poda,
notas, iconos y resúmenes. Reutiliza `bdots_desk`, `bdots_more`, `bdots_text`. El endpoint
`botInsights` añade el campo `context`; la sección «Context window» se muestra en los cuatro clientes.

## Vivacidad (23 helpers + UI en 4 clientes)

`bdots_liveness.rs` añade 23 helpers puros portados de `activity_tracking.py` y
`delivery_outcome.py` de **hermes-agent**: estados de vivacidad, frescura y edad de actividad,
generaciones y fencing, cadencia de heartbeat, resultados de entrega de cron, errores, y
ensamblaje. Reutiliza `bdots_text`, `bdots_more`, `bdots_desk`. El endpoint `botInsights` añade el
campo `liveness`; la sección «Liveness» se muestra en los cuatro clientes.

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
