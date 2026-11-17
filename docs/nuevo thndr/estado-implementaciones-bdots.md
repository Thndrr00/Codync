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

## Verificación

- Desktop: `npm run typecheck` limpio; `npm test` → 562 tests, 538 pasan, 0 fallan, 24 omitidos.
- Android: `:design:test :core:test` → BUILD SUCCESSFUL; `TabPillGeometryTest` 2/2.
- Host: `cargo fmt --check` limpio. `cargo test` no pudo completarse en este equipo: `aws-lc-sys`
  necesita NASM (se usó `AWS_LC_SYS_PREBUILT_NASM=1`) y el equipo tenía 0,45 GB libres por builds
  simultáneos de otros agentes, así que la compilación se interrumpió sin mensaje de error. Queda
  pendiente ejecutar `cargo test` en un entorno con memoria suficiente.
- iOS: `xcodebuild` no está disponible en Windows; los tests de CodyncKit no se ejecutaron aquí.

La rama está pusheada como `feat/bdots-50-mejoras` y el PR es
[Thndrr00/Codync#8](https://github.com/Thndrr00/Codync/pull/8) contra `main` (el fork no tiene rama `dev`).

## Puente iOS

`HostClient+Bdots.swift` reutiliza el único `HostClient.call` que ya usan las demás pantallas —no
crea un segundo transporte— para `listBotApps`, `createAppLink`, `revokeAppLink`, `deleteBotApp`,
`listOpenItems`, `listBookmarks`, `deleteBookmark` y `appRequest`. Android ya tenía el mismo puente
en `AppStoreBots.kt`/`MiniAppBridge.kt`; el de iOS usa los modelos de `BdotsModels.swift` y conserva
la CSP que devuelve el host para la vista web. Tests de decodificación en `BdotsBridgeTests.swift`
(sin ejecutar aquí: no hay `xcodebuild` en Windows).
