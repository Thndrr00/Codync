# Implementaciones de otros repos: catálogo para una app de bots definitiva

Fecha: 2026-10-21. Estado: investigación; sin cambios de código.
Alcance: [CopilotKit/OpenDots](https://github.com/CopilotKit/OpenDots),
[nightly-labs/openbot](https://github.com/nightly-labs/openbot) y
[elie222/rakazo](https://github.com/elie222/rakazo).

Commits revisados:

| Repo | Commit | Fecha | Licencia |
| --- | --- | --- | --- |
| OpenDots | `ab39f2508f3fd23a949837622456f3d61a108e9a` | 2026-10-15 | MIT |
| openbot | `b4cb1bcebf6b7c7d3a341551a293d569ff150acc` | 2026-10-21 | PolyForm Noncommercial 1.0.0 |
| rakazo | `8d182d10734135615f0d8ea7e925a82ef756d647` | 2026-10-21 | Apache-2.0 |

Este documento es un catálogo de decisiones, no un plan cerrado. Contiene 60 fichas
accionables (13 de OpenDots, 25 de openbot y 24 de rakazo; dos fichas combinan dos
repos) y 40 piezas adicionales inventariadas al final. Ninguna ficha se ha validado
ejecutando los proyectos: la evidencia es el árbol de archivos, la documentación y
el código leído en los commits de arriba. Los nombres de archivos nuevos y de
métodos son propuestas; antes de escribir código hay que verificar el dispatcher y
los contratos vigentes.

## 1. Resumen ejecutivo

Los tres repos resuelven el mismo problema desde tres tesis distintas:

- **OpenDots** es una plantilla de producto: espacios de documentos, especialistas
  con nombre y rol, una computadora por bot, tarjetas de revisión humana, canales
  (Slack), llamadas de voz e investigación web. Su valor está en el **producto y
  los flujos de confianza**, no en la infraestructura.
- **openbot** es un **sistema operativo de agentes**: varios proveedores (Codex,
  Claude, Grok, ACP), colas persistentes, tareas y esperas, canales con dueño,
  webhooks firmados, skills, MCP con OAuth, navegador CDP, computer use, API local
  de scripts y protocolo remoto versionado. Su valor está en la **orquestación y
  la operación**, pero su licencia impide reutilizar código comercialmente.
- **rakazo** es un **producto self-hosted con rigor de plataforma**: espacios como
  frontera de autorización, computadoras de equipo y privadas, proveedores de
  sandbox, cifrado en reposo con búsqueda cifrada, secretos con origen fijado,
  aprobaciones con reglas, flotas de bots, tickets, artefactos versionados,
  grabación de flujos convertida en skill y voz multiproveedor. Su valor está en
  la **seguridad, la portabilidad y los detalles de producto**.

Para Codync, la lectura útil no es "copiar features", sino **absorber los contratos
que ya están probados en producción** y reimplementarlos en el host Rust, que es
donde vive la lógica. Codync ya cubre más superficie que los tres repos en algunos
ejes (ACP multiagente, grupos e hilos, memoria con sueños, pantalla remota, relay
E2E, app iPhone/Android/escritorio/TUI). El hueco real está en:

1. **Entregables durables**: documentos y artefactos versionados, no solo chat.
2. **Confianza explícita**: aprobación por herramienta, secretos reutilizables,
   claves de efecto e idempotencia.
3. **Trabajo en segundo plano**: tareas, esperas, preguntas laterales y API local.
4. **Aislamiento**: computadora por bot en contenedor, no solo la máquina real.
5. **Canales**: Slack/Telegram/Discord de primera clase.
6. **Seguridad en reposo**: cifrado de transcripciones, memoria y archivos.
7. **Operación**: flujos de rutina, watch triggers, resultados estructurados.

Las diez apuestas con mejor relación impacto/esfuerzo:

| # | Apuesta | Fichas | Por qué |
| --- | --- | --- | --- |
| 1 | Aprobaciones, secretos y claves de efecto | 31-35 | Convierte "el agente hizo algo" en un contrato auditable |
| 2 | Documentos y artefactos con revisión | 1-7 | El trabajo deja de morir en el transcript |
| 3 | Tareas, esperas y pregunta lateral | 9-11 | Trabajo real en paralelo sin bloquear el chat |
| 4 | API local de scripts + CLI | 46 | Automatización externa sin abrir la app |
| 5 | SandboxProvider y computadora por bot | 19-20 | Aísla al agente de la máquina del usuario |
| 6 | Canales Slack/Telegram/Discord | 53-55 | Lleva el bot a donde está el usuario |
| 7 | Cifrado en reposo + búsqueda cifrada | 49-50 | Protege el dato si se filtra el SQLite |
| 8 | Flujos de rutina, filtros y watch | 40-42 | Automatización condicional, no solo cron |
| 9 | Grabar un flujo y convertirlo en skill | 29 | Convierte conocimiento tácito en capacidad |
| 10 | Gestión de flota | 12 | Un bot puede organizar a los demás con límites |

## 2. Método y límites

Qué se hizo:

- Se leyó el árbol completo de los tres repos vía API de GitHub y se identificaron
  los módulos por responsabilidad, no por nombre de carpeta.
- Se leyeron los README, los documentos de arquitectura y los archivos fuente que
  definen los contratos (tipos, esquemas, validaciones, stores, rutas).
- Se contrastó cada pieza con el código y la documentación de Codync
  (`docs/features/`, `docs/architecture/`, `host/src/`, `apps/`).

Qué no se hizo:

- No se clonaron ni ejecutaron los proyectos; no hay verificación de runtime.
- No se auditaron dependencias, CVEs ni el estado real de sus tests.
- No se estimaron días; "Esfuerzo" es relativo a un ingeniero que ya conoce
  Codync: S ≤ 1 semana, M 1-3 semanas, L 3-6 semanas, XL > 6 semanas.
- Las rutas citadas son evidencia de lectura, no una obligación de copiarlas.

Advertencia: los tres repos son jóvenes (creados entre agosto y septiembre de
2026) y cambian rápido. Antes de adoptar una idea, volver a leer el commit actual.

## 3. Licencias: qué se puede reutilizar y qué no

| Repo | Licencia | Reutilización de código | Decisión recomendada |
| --- | --- | --- | --- |
| OpenDots | MIT | Sí, con aviso de copyright | Se puede portar código puntual citando el aviso |
| rakazo | Apache-2.0 | Sí, con NOTICE y concesión de patentes | Se puede portar código citando NOTICE |
| openbot | PolyForm Noncommercial 1.0.0 | No para uso comercial | **Solo estudiar ideas y contratos; reimplementación limpia** |

PolyForm Noncommercial permite uso no comercial; una app de bots distribuida o
vendida no encaja. Aunque se reimplemente, conviene dejar constancia de que no se
copió código: un `docs/reference/upstream-attribution.md` con repo, commit, ruta y
"idea tomada, código no copiado" para las piezas de openbot. Para MIT y Apache-2.0,
lo mismo pero con el aviso de licencia completo.

Conclusión práctica: **OpenDots y rakazo pueden donar código; openbot solo puede
donar diseño.** Todas las fichas de openbot están redactadas como contrato a
reimplementar.

## 4. Las tres tesis, lado a lado

| Eje | OpenDots | openbot | rakazo |
| --- | --- | --- | --- |
| Producto | Espacios + especialistas + computadora | Espacio de trabajo de agentes persistentes | Bots persistentes self-hosted |
| Runtime de agente | CopilotKit Intelligence + AG-UI | Codex app-server, Claude, Grok, ACP, custom | Pi (cualquier modelo) |
| Multi-proveedor | Un solo stack | Muy fuerte | Fuerte, con OAuth y probe |
| Base de datos | SQLite + servicio de conversaciones | SQLite local | PostgreSQL + Prisma |
| Multi-tenant | Un owner | Cuentas + servidores hosteados | Espacios, miembros, roles |
| Escritorio | Web y móvil | Electron + móvil + web | Electron + web + Expo |
| Computadora | Contenedor por Dot | Máquina local + computer use | Docker/E2B/Daytona/Boat/CreateOS |
| Navegador | Control de página + takeover | CDP completo + PiP + recorder | Chromium por bot + helper |
| Aprobaciones | Tarjeta por herramienta MCP | Takeover y preguntas | Reglas + tarjetas + efecto |
| Secretos | Tokens de conexión | 1Password/Bitwarden | Secretos por bot con origen |
| Voz | Realtime + compute delegation | Speech host + transcripción | Multiproveedor + barge-in |
| Canales | Slack | Slack/Discord/Telegram | Webhooks de mensajería |
| Cifrado | No documentado | No documentado | En reposo + búsqueda cifrada |
| Licencia | MIT | Noncommercial | Apache-2.0 |

Lectura por repositorio:

### 4.1 OpenDots: qué tomar

Fortalezas:

- Convierte conversaciones en **páginas** con editor, subpáginas y autosave.
- Tiene el mejor flujo de **revisión humana antes de guardar** y de **aprobación
  de herramientas MCP** con argumentos exactos y ejecución única.
- La **computadora por bot** está bien separada: credencial derivada, volúmenes
  persistentes, permisos por capacidad, actividad auditada.
- Voz e investigación web están integradas al mismo agente, no como apps aparte.
- El diseño de "especialista" (nombre, rol, instrucciones, herramientas) es simple
  y comprensible.

Debilidades:

- Depende de CopilotKit Intelligence para conversaciones; no es totalmente
  self-hosted sin licencia.
- Un solo owner y una sola instancia; no hay multi-tenant.
- Es una plantilla alfa, sin la madurez operativa de openbot.

Fichas que aporta: 1-6, 19, 25-28, 31, 53.

### 4.2 openbot: qué tomar

Fortalezas:

- **Orquestación real**: colas FIFO persistentes, tareas con hilo propio, esperas,
  carriles de drenaje, límites de concurrencia y recuperación tras crash.
- **Canales con semántica de trabajo**: dueño de tarea, delegación explícita,
  historial compartido, stop/resume/reassign/archive/restore.
- **Webhooks de primera**: rutas firmadas, filtros JSON Pointer, plantillas,
  recibos y rotación de secreto.
- **Navegador CDP** con snapshot de accesibilidad, refs, formularios, PiP y
  grabación; computer use con daemon y overlays por pantalla.
- **Operación**: API local de scripts con CLI y códigos de salida, importación de
  instancias, plantillas de agente, telemetría de uso y límites.
- Protocolo remoto versionado con capacidades opcionales.

Debilidades:

- Licencia noncommercial.
- Los agentes corren con acceso total y sin aprobaciones por defecto (su propio
  README lo advierte).
- Complejidad alta: 5.000+ archivos; muchas capas para un producto pequeño.

Fichas que aporta: 9-11, 14-18, 30, 36, 38-41, 43-47, 52-55, 59-60.

### 4.3 rakazo: qué tomar

Fortalezas:

- **Fronteras de autorización**: espacios, roles, computadoras de equipo vs
  privadas, secretos por bot y por usuario.
- **Seguridad demostrable**: cifrado en reposo con claves por espacio, AAD que
  ata el cifrado a tabla/columna/fila, búsqueda con tokens HMAC, consentimiento
  de datos de IA.
- **Aprobaciones con reglas** y claves de efecto idempotentes.
- **Producto**: flotas, tickets, artefactos versionados, enseñar un flujo,
  actividad de herramientas, voz con barge-in y recuperación de llamadas.
- **Portabilidad**: contrato de sandbox, backups, self-update, despliegue.

Debilidades:

- Beta; PostgreSQL + Prisma + worker añaden operación.
- Mucha superficie y documentación muy densa.
- Depende de Pi como runtime, aunque el contrato es neutral.

Fichas que aporta: 7-8, 12-13, 20-24, 29, 32-35, 37, 42, 47-51, 56-58.

### 4.4 Convergencia: la misma necesidad, tres respuestas

| Necesidad | OpenDots | openbot | rakazo | Lectura para Codync |
| --- | --- | --- | --- | --- |
| Objeto de trabajo durable | Spaces y páginas | Tareas e hilos | Artefactos y tickets | Elegir un modelo único, no cuatro |
| Trabajo en curso | Tarjetas de tarea | Tasks con carril | Runs y tablero | Empezar por tareas (9) |
| Aprobación | Revisión + Ask first | Attention registry | Reglas + efecto | Un solo contrato (31-35) |
| Computadora | Contenedor por Dot | Máquina local + CDP | SandboxProvider | Contrato + dos modos (19-20) |
| Canal | Slack | Slack/Discord/Telegram | Webhooks | Driver común (53-54) |
| Memoria | Threads de Intelligence | Store + selección | Proveedores | Proveedor + presupuesto (47) |
| Voz | Realtime + compute | Speech host | Multiproveedor + barge-in | Recuperación y nonce (57) |
| Entrega al usuario | Chat + página | Chat + archivo | Chat + artefacto | Documento/artefacto primero (1-7) |

La convergencia es la señal más fuerte: los tres equipos llegaron por separado a
la misma conclusión. Un bot útil necesita **objetos durables**, **permisos
explícitos** y **un contrato de computadora**. Codync ya tiene el runtime y la
memoria; le faltan las dos primeras capas.

## 5. Qué ya tiene Codync (no volver a construirlo)

| Capacidad | Dónde vive | Estado | Implicación |
| --- | --- | --- | --- |
| Agentes ACP múltiples | `host/src/agent/`, `backends.rs` | Completo | No crear un runtime paralelo |
| Grupos, hilos y lanes | `host/src/chat/group.rs`, `store/model.rs` | Completo | Reutilizar `Lane` para documentos y tareas |
| Delegación bot-a-bot | `host/src/chat/team.rs` (`ask_bot`, `message_bot`) | Completo | La flota y los subagentes deben apoyarse aquí |
| Cola de turnos y cancelación | `host/src/agent/bot/queue.rs` | Parcial | Falta persistencia, pausa y carriles |
| Memoria, sueños y búsqueda | `host/src/chat/memory/`, `crates/memory` | Completo | Los proveedores de memoria son una extensión |
| Rutinas, cron, webhooks y eventos | `host/src/routines/` | Parcial | Ya existen; faltan filtros JSON Pointer, flujos y watch |
| Computer use | `host/src/screen/cua.rs`, `computer.rs` | Completo | Base para permisos por capacidad y auditoría |
| Pantalla remota y takeover | `host/src/screen/`, `apps/screen-*` | Completo | Base para leases y snapshot fresco |
| Conectores MCP, skills y Composio | `host/src/market/` | Completo | Falta política por herramienta |
| Credenciales cifradas | `host/src/market/vault.rs` | Completo | Reutilizar el vault para secretos de bot |
| 1Password por referencia | `host/src/market/passwords.rs` | Parcial | Falta autofill y Bitwarden |
| Voz y llamadas | `host/src/voice.rs`, `apps/ios/.../Call/` | Parcial | Ya existen; falta recuperación y nonce |
| Push y Live Activity | `relay/`, `apps/ios/Widgets/` | Completo | No copiar el Dynamic Island de openbot |
| Relay E2E y mailbox | `cloud/`, `host/src/remote/` | Completo | Modelo de confianza ya resuelto |
| Analítica de uso | `host/src/analytics.rs`, `usage.rs` | Completo | Base para presupuesto de contexto |
| Escritorio, iPhone, Android, TUI | `apps/desktop`, `apps/ios`, `apps/android`, `host/src/tui` | Completo | Toda ficha debe cubrir los cuatro |

Regla de trabajo: si una ficha necesita estado, se añade al host; los clientes
solo renderizan y envían intenciones. Ninguna ficha puede implementarse dos veces
en dos clientes.

## 6. Cómo leer cada ficha

- **Fuente**: repo, ruta y, cuando existe, documento que describe el
  comportamiento.
- **Qué hace**: el mecanismo, no el marketing.
- **Hueco en Codync**: qué falta hoy y por qué no es duplicado.
- **Adaptación**: contrato propuesto en el host, tablas, métodos y superficies.
- **Esfuerzo / impacto**: relativo, no una estimación de calendario.
- **Depende de**: fichas que deben ir antes.
- **Aceptación**: prueba observable, no "compila".

## 7. Catálogo de 60 implementaciones

### Bloque A — Documentos, artefactos y revisiones (1-8)

El chat es un mal lugar para un entregable. Este bloque convierte el resultado de
una conversación en algo que se puede leer, versionar, revisar y volver a abrir.

#### 1. Biblioteca de Spaces con subpáginas

- **Fuente:** OpenDots — `src/server/page-service.ts`, `src/server/pages.ts`,
  `src/client/SpaceLibrary.tsx`, `PageOutline.tsx`; README, sección *Spaces*.
- **Qué hace:** un Space es una casa de documentos. Las páginas se listan con
  búsqueda, vista de cuadrícula o lista, y se organizan como subpáginas anidadas.
  Cada página tiene un destino por defecto y se abre en un editor enfocado.
- **Hueco en Codync:** no hay store de documentos; el conocimiento vive en el
  transcript, la memoria y los archivos del workspace. Un usuario no puede
  "abrir el informe que el bot escribió ayer" sin buscar en el chat.
- **Adaptación:** tabla `documents(id, bot_id, space_id, parent_id, title, body,
  format, revision, created_at, updated_at, deleted_at)`; métodos
  `listDocuments`, `getDocument`, `saveDocument`, `moveDocument`,
  `deleteDocument`. `space_id` puede empezar siendo el `bot_id` o el grupo; no
  crear multi-tenant todavía. En escritorio, una vista de biblioteca; en iPhone y
  Android, una lista con búsqueda; en TUI, un comando `docs`.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** crear dos páginas, anidar una, buscar por título y reabrir
  después de reiniciar el host; el cuerpo sobrevive byte a byte.

#### 2. Editor con slash commands y modo Markdown

- **Fuente:** OpenDots — `src/client/editor/RichEditor.tsx`,
  `src/client/editor/slash-commands.ts`, `markdown.ts`, `DocumentMenu.tsx`.
- **Qué hace:** edición visual con negrita, listas, encabezados, enlaces y un
  menú `/` para insertar bloques; el modo Markdown sigue disponible y el cuerpo
  se guarda como Markdown.
- **Hueco en Codync:** los clientes renderizan Markdown, pero no lo editan. Un
  documento sin edición no reemplaza al chat.
- **Adaptación:** guardar siempre Markdown; el editor visual es una capa del
  cliente. Empezar por escritorio con un editor de bloques y por iOS/Android con
  edición de Markdown plano y vista previa. Slash commands mínimos: `/doc`,
  `/routine`, `/skill`, `/task`. No implementar un editor colaborativo.
- **Esfuerzo:** M (Markdown) / L (visual). **Impacto:** alto. **Depende de:** 1.
- **Aceptación:** editar, guardar, cerrar y reabrir conserva el Markdown; el
  render de cada cliente coincide con el del host.

#### 3. Autosave con revisión y protección contra pisadas

- **Fuente:** OpenDots — `src/client/editor/autosave.ts`,
  `use-page-autosave.ts`, `src/client/page-snapshots.ts`; test
  `tests/autosave.test.ts`.
- **Qué hace:** guarda con debounce, informa el progreso, conserva el borrador si
  falla y compara revisiones para que una edición vieja no pise una nueva.
- **Hueco en Codync:** no aplica hoy porque no hay documentos; el patrón también
  sirve para instrucciones de bot y borradores de rutina.
- **Adaptación:** `revision` monotónica por documento; `saveDocument` acepta
  `ifRevision` y devuelve `conflict` con el cuerpo actual; el cliente conserva el
  borrador y ofrece "recargar" o "sobrescribir". Autosave cada 800 ms de
  inactividad, con un guardado forzado al cerrar.
- **Esfuerzo:** S. **Impacto:** alto. **Depende de:** 1.
- **Aceptación:** dos clientes editan la misma página; el segundo recibe
  conflicto y no pierde texto; un fallo de red no borra el borrador local.

#### 4. Conversación por página y por especialista

- **Fuente:** OpenDots — `src/client/PageConversation.tsx`,
  `page-chat-requests.ts`, `page-context.ts`, `PageDocument.tsx`.
- **Qué hace:** cada página tiene su propia conversación con el especialista; el
  agente recibe el contenido de la página como contexto no confiable y puede
  proponer cambios.
- **Hueco en Codync:** los hilos (`Lane`) dan conversaciones separadas por
  mensaje raíz, pero no por documento. Un bot no puede "trabajar en este
  documento" con una sesión propia y volver a él.
- **Adaptación:** extender `Lane` con `doc:<id>`; al abrir una página, el cliente
  usa esa lane; el prompt incluye el documento entre delimitadores y con la
  advertencia de datos no confiables que ya usa `chat/context.rs`.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 1.
- **Aceptación:** dos páginas del mismo bot tienen sesiones y transcripts
  distintos; cambiar de página no mezcla historial.

#### 5. Tarjeta de revisión humana antes de guardar

- **Fuente:** OpenDots — `src/shared/page-review.ts`,
  `src/client/PageReviewCard.tsx`, `page-review-decision.ts`; README *Review
  before saving*; test `tests/page-review-card.test.tsx`.
- **Qué hace:** el agente llama `review_space_page` con título, contenido y
  Space. La conversación se pausa y aparece una tarjeta con *Approve & save* o
  *Decline*. Solo la aprobación crea la página y devuelve el enlace.
- **Hueco en Codync:** las aprobaciones actuales son del harness ACP (permisos de
  herramienta), no del host, y no existe una acción "guardar documento" con
  revisión. Es el patrón que falta para que un bot no publique solo.
- **Adaptación:** tabla `pending_actions(id, bot_id, lane, kind, payload_hash,
  payload, status, expires_at, created_at)`; herramienta `review_document` que
  crea la acción y espera; método `resolveAction` con `approve|decline`. El host
  ejecuta el payload guardado, nunca el que envía el cliente al aprobar. Las
  tarjetas se renderizan en los cuatro clientes; en TUI, una línea con `a`/`d`.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 1, 3.
- **Aceptación:** aprobar guarda exactamente el borrador revisado; rechazar no
  crea nada; una aprobación expirada no ejecuta; el payload no se puede
  sustituir desde el cliente.

#### 6. Aprobación idempotente y reintento seguro

- **Fuente:** OpenDots — README *Review before saving* y
  `src/server/page-service.ts`; el reintento con el mismo borrador recupera la
  página ya guardada.
- **Qué hace:** si el agente reintenta tras un timeout, no se duplica el
  documento; la misma aprobación devuelve el resultado original.
- **Hueco en Codync:** `clientNonce` deduplica envíos de usuario, no efectos de
  herramientas. Un reintento puede duplicar una rutina, un archivo o un mensaje.
- **Adaptación:** `effect_key` único por acción (`hash(kind + payload + bot)`);
  tabla `effects(effect_key, result, created_at)`; el ejecutor consulta antes de
  actuar y guarda el resultado en la misma transacción. Aplicarlo primero a
  documentos, luego a rutinas y archivos.
- **Esfuerzo:** S. **Impacto:** alto. **Depende de:** 5.
- **Aceptación:** ejecutar dos veces la misma aprobación produce un solo
  documento y la misma respuesta.

#### 7. Artefactos versionados con sandbox de render

- **Fuente:** rakazo — `packages/adapters/src/artifacts.ts`,
  `packages/db/src/artifact-versions.ts`, `packages/core/src/artifact-sandbox.ts`,
  migración `20260925120000_artifact_versions`, `docs/media/artifacts-tab/`.
- **Qué hace:** los archivos generados por un bot son artefactos con versiones,
  metadatos, nonce de cliente y un sandbox para renderizarlos sin ejecutar
  código arbitrario en el origen de la app.
- **Hueco en Codync:** `chat/files.rs` y `file-attachments.md` cubren adjuntos de
  chat, no una biblioteca de entregables con historial. Un PDF regenerado
  reemplaza al anterior sin trazabilidad.
- **Adaptación:** `artifacts(id, bot_id, thread_id, name, mime, size, sha256,
  current_version, created_at)` + `artifact_versions(id, artifact_id, version,
  blob_path, sha256, created_at)`. Blobs content-addressed en el data dir; el
  cliente previsualiza con el visor existente. Añadir "restaurar versión".
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** regenerar un artefacto crea la versión 2; restaurar la 1
  devuelve el contenido original; el hash coincide.

#### 8. Historial de instrucciones con diff y autor

- **Fuente:** rakazo — `packages/core/src/instruction-diff.ts`,
  `docs/fleet-management.md` (historial de instrucciones con razón y autor);
  openbot — `src/backend/agent/profile-generation.ts` como contraste.
- **Qué hace:** cada cambio de instrucciones se guarda como versión con autor,
  motivo y diff; una edición de un bot hijo no pisa un cambio concurrente.
- **Hueco en Codync:** `BotConfig` guarda las instrucciones actuales; no hay
  versiones, autor ni comparación. Un bot que edita a otro no tiene control de
  concurrencia.
- **Adaptación:** `bot_instruction_versions(id, bot_id, body, author_kind,
  author_id, reason, created_at)`; `updateBot` acepta `ifVersion`; método
  `instructionDiff(botId, from, to)`. Aplicar también a las instrucciones de
  grupo y a las de un subagente.
- **Esfuerzo:** S. **Impacto:** medio. **Depende de:** nada.
- **Aceptación:** dos ediciones concurrentes: la segunda recibe conflicto y
  puede ver el diff; restaurar una versión crea una nueva, no borra historial.

### Bloque B — Orquestación del trabajo (9-18)

Un bot que solo responde cuando le hablas no es un compañero. Este bloque añade
trabajo en paralelo, esperas, preguntas laterales y límites de concurrencia.

#### 9. Tareas de agente con hilo y sesión propios

- **Fuente:** openbot — `src/backend/agent-tasks/agent-tasks.ts`,
  `agent-task-store.ts`, `agent-task-schema.ts`; herramienta `start_task`; test
  `agent-task-migration.test.ts`.
- **Qué hace:** un bot abre tareas de fondo, cada una con su propio hilo, sesión
  de proveedor y estado. La UI tiene una pestaña *Tasks* con progreso y la tarea
  no contamina el chat principal.
- **Hueco en Codync:** hay rutinas (programadas) e hilos (respuestas), pero no una
  lista de trabajo en curso con ciclo de vida (`queued|running|waiting|done|
  failed|cancelled`). El usuario no puede ver "qué está haciendo" más allá del
  estado del bot.
- **Adaptación:** `agent_tasks(id, bot_id, lane, title, brief, state, progress,
  result, created_at, updated_at)`; herramientas `start_task`, `list_tasks`,
  `cancel_task`; un carril por tarea reutilizando `agent/bot/queue.rs`. En
  escritorio, panel lateral; en iPhone/Android, una sección en el detalle del
  bot; en TUI, `t` abre la lista.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** dos tareas del mismo bot corren sin mezclar transcripts; una
  cancelación deja el estado `cancelled` y no produce respuesta huérfana.

#### 10. Esperas de agente (`await_event` / `close_wait`)

- **Fuente:** openbot — `src/backend/agent-waits/agent-waits.ts`,
  `agent-wait-store.ts`, `agent-wait-schema.ts`.
- **Qué hace:** el agente declara que no puede continuar hasta que ocurra algo
  (una rutina, un webhook, una respuesta). La espera se guarda, el usuario puede
  cancelarla y una ejecución de rutina la despierta con el contexto necesario.
- **Hueco en Codync:** una rutina puede ejecutarse, pero el agente no puede
  "quedarse esperando" de forma explícita; el turno termina y el contexto se
  pierde.
- **Adaptación:** `agent_waits(id, bot_id, note, routine_id, state, created_at,
  woken_at)`; herramientas `await_event`, `close_wait`; al despertar, el input
  repite la petición y la nota. Límite por bot y expiración.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** 9, 40.
- **Aceptación:** una espera despierta exactamente una vez; cancelarla impide el
  turno; reiniciar el host no pierde la espera.

#### 11. Pregunta lateral sin tocar el hilo

- **Fuente:** openbot — `src/backend/agent/side-question.ts`; límites
  `SIDE_QUESTION_CHAT_MESSAGES = 60`, presupuesto de 60k caracteres, sin
  herramientas.
- **Qué hace:** el usuario pregunta "¿por qué elegiste esa librería?" sin abrir
  un turno nuevo en el chat. El host arma un prompt acotado con el final del
  historial y las preguntas laterales anteriores; la respuesta es efímera.
- **Hueco en Codync:** cualquier pregunta se envía al agente y entra al
  transcript, consume contexto y puede cambiar el trabajo en curso.
- **Adaptación:** método `askSideQuestion {botId, lane, question}`; el host lee
  los últimos N mensajes, recorta por presupuesto, llama al modelo sin
  herramientas y devuelve texto efímero. No persistir como entry; guardar solo
  métricas. UI: mantener pulsado el botón de enviar o `⌘/`.
- **Esfuerzo:** S. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** la pregunta no aparece en el transcript ni altera la sesión;
  una pregunta sin contexto suficiente responde "no consta" en vez de inventar.

#### 12. Gestión de flota: un bot organiza a otros

- **Fuente:** rakazo — `packages/adapters/src/fleet-tools.ts`,
  `docs/fleet-management.md`, migración `20261018120000_bot_fleet_management`.
- **Qué hace:** con un flag por bot, el bot puede listar bots y grupos, crear
  grupos, organizar secciones/pines, editar instrucciones y modelo de bots que
  creó, y crear rutinas hijas. Cada acción destructiva muestra una tarjeta de
  aprobación con el contenido resuelto; no puede borrar, mover de espacio, leer
  secretos ni editar bots ajenos.
- **Hueco en Codync:** `team.rs` permite pedir a otro bot, pero no administrar la
  flota. Un bot no puede decir "crea un equipo de investigación con estos tres".
- **Adaptación:** `fleet_management: bool` en `BotConfig` (off por defecto, no
  heredado); herramientas `list_fleet`, `create_group`, `update_group`,
  `organize_bot`, `update_bot`, `set_child_instructions`,
  `set_child_bot_model`, `create_child_routine`; validación de versión de
  instrucciones; tarjetas con allow/deny; límites (2-10 miembros, 200 bots).
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 8, 35.
- **Aceptación:** un bot crea un grupo y no puede editar un bot que no creó;
  apagar el flag a mitad de turno bloquea la siguiente llamada.

#### 13. Subagentes efímeros con modelo propio

- **Fuente:** rakazo — VISION (*bots that delegate to peer bots or short-lived
  subagents*), `packages/adapters/src/cloud-agent-tools.ts`,
  `cloud-agent-tools-select.ts`; openbot — carriles de tareas como contraste.
- **Qué hace:** además de delegar a un bot persistente, el bot lanza un subagente
  temporal con su propia instrucción, modelo y límite de turnos; no tiene
  identidad visible ni memoria propia y devuelve un resultado.
- **Hueco en Codync:** `ask_bot` delega a un bot persistente, con su chat y su
  carpeta. No hay una unidad desechable para "explora esto en paralelo con otro
  modelo".
- **Adaptación:** herramienta `spawn_subagent {brief, model?, maxTurns?}`;
  el host crea una lane temporal, usa el mismo motor ACP, limita profundidad (2)
  y paralelismo (4), y devuelve solo el texto final. Sin herramientas de flota ni
  secretos. Al terminar, la lane se borra o se archiva como evento.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** 9, 15.
- **Aceptación:** un subagente no aparece en la lista de bots; no puede lanzar
  otro subagente; su fallo no tumba el turno padre.

#### 14. Canal con dueño de tarea, delegación y reassign

- **Fuente:** openbot — `src/backend/channel-service.ts`, `channel-store.ts`,
  `channel-history.ts`, `docs/architecture/channels-and-messaging.md`; límites
  `CHANNEL_ASSIGNMENT_LIMIT`, `CHANNEL_PARALLEL_LIMIT`.
- **Qué hace:** un canal es un chat compartido con varios agentes, pero solo uno
  es dueño de la tarea. El dueño puede delegar de forma explícita, el historial
  es común y el usuario puede parar, reanudar, reasignar, archivar y restaurar.
- **Hueco en Codync:** los grupos ya tienen varios bots y colas, pero no un
  `ownerBotId`, ni reassign, ni archivo/restauración con historial. Cuando el bot
  dueño se atasca, el grupo queda bloqueado.
- **Adaptación:** campos `owner_bot_id`, `assignment_state`, `parallel_limit`,
  `archived_at` en el grupo; acciones `assignGroupTask`, `reassignGroupTask`,
  `archiveGroup`, `restoreGroup`; eventos de asignación en el transcript. La UI
  muestra quién tiene la tarea y permite reasignar desde el chat.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 15.
- **Aceptación:** reasignar mueve la tarea y el historial se conserva; archivar
  impide nuevo trabajo pero permite restaurar; un miembro oculto no recibe
  trabajo nuevo.

#### 15. Carriles de drenaje, slots de turno y límites de paralelismo

- **Fuente:** openbot — `src/backend/agent/drain-scheduler.ts`,
  `src/backend/agent/turn-slots.ts`, `src/backend/idle-thread-pool.ts`,
  `agent-tasks/agent-tasks.ts` (un carril por tarea).
- **Qué hace:** el trabajo se reparte en carriles justos; cada bot tiene un
  número limitado de turnos simultáneos; las tareas y canales compiten sin
  bloquear el chat principal.
- **Hueco en Codync:** `agent/bot/queue.rs` serializa por bot y `CancelGroup`
  limpia la cola del grupo. No hay límite configurable ni prioridad entre chat,
  grupo, rutina y tarea; una rutina larga puede retrasar al usuario.
- **Adaptación:** `turn_slots(bot_id, max_concurrent)` con default 1 para chat y
  2 para tareas; prioridad `user > group > task > routine`; un scheduler en
  `hub.rs` que reparta y exponga `queueState`. Medir antes de optimizar.
- **Esfuerzo:** M. **Impacto:** medio. **Depende de:** 9.
- **Aceptación:** con dos tareas y un mensaje del usuario, el mensaje se atiende
  primero; ninguna tarea se queda sin ejecutar de forma permanente.

#### 16. Recuperación tras reinicio y cola persistente

- **Fuente:** openbot — `src/backend/agent/boot-recovery.ts`,
  `queue-controls.ts`, `agent-task-store.ts`; documenta que la cola sobrevive a
  un crash.
- **Qué hace:** al arrancar, el host reconstruye el trabajo pendiente, reanuda lo
  que puede y marca lo demás con un error accionable; no pierde mensajes.
- **Hueco en Codync:** `hub.rs` marca las solicitudes pendientes como fallidas al
  arrancar y no las reintenta. Un reinicio a mitad de una rutina deja un hueco.
- **Adaptación:** tabla `queued_turns(id, bot_id, lane, kind, payload, state,
  attempts, created_at)`; estados `queued|running|done|failed`; al arrancar,
  reencolar `queued` y marcar `running` como `interrupted` (no reintentar efectos
  no idempotentes). Combinar con 6.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 6, 15.
- **Aceptación:** matar el host con un mensaje en cola lo conserva; al arrancar
  se ejecuta una vez; una rutina interrumpida no duplica su efecto.

#### 17. Pausa, reanudación y cancelación por mensaje

- **Fuente:** openbot — README (*FIFO message queues with pause, resume,
  cancellation*) y `src/backend/agent/queue-controls.ts`.
- **Qué hace:** el usuario pausa la cola de un bot sin detener el turno actual,
  reanuda después, o cancela un mensaje concreto antes de que empiece.
- **Hueco en Codync:** existe `Stop` y `CancelGroup`, pero no pausar la cola ni
  cancelar un mensaje identificado. El usuario solo puede detener todo.
- **Adaptación:** `pauseQueue`, `resumeQueue`, `cancelQueued {entryId}`; estado
  visible en la cabecera del chat ("3 en cola, en pausa"); en TUI, `p`/`r`.
- **Esfuerzo:** S. **Impacto:** medio. **Depende de:** 16.
- **Aceptación:** pausar no cancela el turno en curso; reanudar respeta el orden;
  cancelar un mensaje no afecta a los demás.

#### 18. Presupuesto de contexto visible y compactación anticipada

- **Fuente:** openbot — `src/backend/agent/context-compaction.ts`,
  `src/backend/claude-context-categories.ts`; rakazo —
  `docs/context-and-evals-tasks.md` (historial reciente acotado + catálogo de
  navegación).
- **Qué hace:** el host conoce el tamaño del contexto, lo muestra por categorías
  y compacta antes de agotar la ventana; distingue historial, herramientas,
  archivos y sistema.
- **Hueco en Codync:** `usage.rs` y el epoch de compactación existen, pero el
  usuario no ve el presupuesto ni hay una política proactiva para harnesses que
  no compactan solos.
- **Adaptación:** `contextBudget` por modelo en `agent/model_catalog`; evento
  `context` con `{used, limit, categories}`; aviso al 75%; si el harness soporta
  compactación ACP, invocarla; si no, avisar y ofrecer *New session*.
- **Esfuerzo:** M. **Impacto:** medio. **Depende de:** nada.
- **Aceptación:** una conversación larga muestra el aviso antes de fallar; la
  compactación no pierde los últimos mensajes ni el snapshot de instrucciones.

### Bloque C — Computadoras, pantalla y navegador (19-30)

Codync ya opera la máquina real del usuario con `cua-driver` y Codync Screen. Este
bloque añade una segunda modalidad, aislada y portable, y endurece la primera.

#### 19. Computadora por bot en contenedor con volúmenes persistentes

- **Fuente:** OpenDots — `src/server/computer-service.ts`,
  `computer-store.ts`, `computer-routes.ts`, `compose.computers.yml`,
  `deployment/computers/`, `docs/COMPUTERS.md`.
- **Qué hace:** un supervisor crea un contenedor por Dot, con workspace y perfil de
  navegador persistentes. El token maestro vive en la app y deriva una credencial
  distinta por bot. Si el servicio no está disponible, las herramientas fallan:
  nunca caen al shell o a los archivos del host. Parar conserva los datos; no hay
  reset destructivo.
- **Hueco en Codync:** el bot usa la computadora real del usuario, con los
  permisos de esa persona. No existe un modo "este bot vive en una caja".
- **Adaptación:** `ComputerRuntime` en el host con dos implementaciones:
  `LocalRuntime` (lo actual) y `ContainerRuntime` (Docker). Campos
  `computer_mode: local|container`, `computer_id`. Volúmenes
  `codync-bot-<id>-workspace` y `codync-bot-<id>-profile`. Credencial derivada
  por bot; el host nunca expone el token maestro. Empezar con la CLI de Docker
  (`docker create/start/stop`) para no añadir una dependencia pesada; evaluar
  `bollard` después.
- **Esfuerzo:** XL. **Impacto:** muy alto. **Depende de:** 20, 27.
- **Aceptación:** dos bots no ven los archivos del otro; parar y arrancar conserva
  archivos y sesión de navegador; sin Docker, el modo contenedor falla con un
  error claro y no usa el host.

#### 20. Contrato de proveedor de sandbox

- **Fuente:** rakazo — `docs/computer-runtime.md`, `infra/sandboxes/computer`,
  `infra/sandboxes/desktop`, `infra/sandboxes/supervisor`,
  `packages/adapters/src/computer-tools.ts`, `computer-workspace.ts`,
  `cloud-agent-tools.ts`.
- **Qué hace:** define una frontera única: ciclo de vida (provisionar, reconectar,
  parar, destruir), escritorio (observar, actuar por lotes, input, sesión viva),
  ejecución de comandos, archivos (listar, leer, escribir, importar y exportar
  completo). Docker, E2B, Daytona y Boat implementan el mismo contrato; el
  proveedor solo aporta ejecución, rutas persistentes y URL de pantalla.
- **Hueco en Codync:** no hay trait; la pantalla y el shell están acoplados a la
  máquina local.
- **Adaptación:** trait Rust `SandboxProvider` con `capabilities()` y métodos
  async; `LocalProvider` y `DockerProvider` primero. Un `SandboxRegistry` por
  bot. Los errores se tipan (`Unavailable`, `Busy`, `Expired`, `Denied`) para que
  el cliente muestre estados distintos, no un texto genérico.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** el mismo test de ciclo de vida pasa con local y Docker; cambiar
  de proveedor no cambia el contrato que ve el agente.

#### 21. Computadora de equipo frente a computadora privada

- **Fuente:** rakazo — VISION (*Computers are durable places*),
  `docs/computer-runtime.md`, `packages/adapters/src/computer-support.ts`,
  `computer-workspace.ts`.
- **Qué hace:** una Team Computer comparte archivos, herramientas y una identidad
  de navegador canónica entre bots del mismo espacio; cada bot trabaja en
  `bots/<bot-id>/` y lo compartido va en `shared/`. Una Private Computer es el
  hogar completo de un solo bot. Los perfiles de Chrome no se clonan ni se borran
  al liberar el escritorio.
- **Hueco en Codync:** cada bot tiene su workspace (`bots/<id>/workspace`), que es
  el equivalente a "privada"; no existe una computadora compartida explícita ni
  una carpeta `shared`.
- **Adaptación:** `computer_scope: private|team` y `team_computer_id` en
  `BotConfig`; layout `computers/<team>/bots/<bot>/` y `computers/<team>/shared/`;
  aviso en la UI de que las carpetas organizan, no aíslan. Empezar solo con
  `private` y añadir `team` cuando exista el runtime de contenedor.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** 19, 20.
- **Aceptación:** dos bots de equipo ven `shared/` y no el workspace del otro;
  cambiar a privada no mueve archivos sin confirmación.

#### 22. Perfiles de navegador persistentes por bot con fencing de generación

- **Fuente:** rakazo — `docs/computer-runtime.md`;
  `packages/adapters/src/computer-browser.ts`, `computer-recovery.ts`; openbot —
  `src/backend/browser-identity.ts`, `browser-policy.ts`,
  `browser-control-sessions.ts`.
- **Qué hace:** cada bot tiene su propio perfil de Chromium, con cookies, logins e
  historial independientes. Varios Chromium no escriben el mismo directorio a la
  vez; un `generation` impide que un escritorio viejo pise el estado nuevo. Las
  sesiones se guardan en la identidad compartida cuando aplica.
- **Hueco en Codync:** el navegador es el del usuario; los logins se guardan por
  sitio (`market/logins.rs`) y `type_login` los escribe, pero no hay perfil
  aislado por bot ni fencing.
- **Adaptación:** `profiles/<bot>/` en el runtime; `generation` por perfil;
  `browser_open` devuelve `{profileId, generation}`; cualquier escritura con
  generación vieja se rechaza. Nunca borrar un perfil al cerrar la sesión.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 20.
- **Aceptación:** dos bots conservan sesiones distintas tras reiniciar; un driver
  viejo no puede escribir; cerrar el viewer no borra el perfil.

#### 23. Leases de pantalla y displays por bot

- **Fuente:** rakazo — `packages/core/src/screen-lease.ts`,
  `packages/core/src/node/screen-capability.ts`, `apps/api/src/screen-proxy.ts`,
  `packages/core/src/node/screen-capability.test.ts`.
- **Qué hace:** cada bot activo recibe su propio display y ventana de Chrome. Ver
  y controlar son capacidades separadas, revocables, con su propio socket; un
  socket viejo no puede redirigirse al siguiente bot. Hay un límite configurable
  de escritorios y se libera el más antiguo inactivo.
- **Hueco en Codync:** hay una sola pantalla y un takeover; no hay leases por bot
  ni capacidades separadas de ver/controlar.
- **Adaptación:** `ScreenLease {bot_id, display, socket, generation}`;
  `screenView` y `screenControl` como tokens distintos; un gateway único; al
  liberar, cerrar clientes antes de reutilizar el slot; error
  `multiScreenUnavailable` si se supera el límite.
- **Esfuerzo:** L. **Impacto:** medio-alto. **Depende de:** 19, 20.
- **Aceptación:** dos bots muestran pantallas distintas; revocar control no corta
  la vista; un token viejo no controla el display reutilizado.

#### 24. Arranque perezoso y apagado por inactividad

- **Fuente:** rakazo — `docs/computer-runtime.md` (*Computer resources start
  lazily and are released when inactive*), `packages/adapters/src/computer-idle.ts`,
  `computer-lifecycle.ts`.
- **Qué hace:** el stack gráfico arranca solo cuando un bot usa una herramienta
  gráfica; al quedar inactivo se apaga, conservando el workspace y el perfil. Los
  perfiles persistentes ocupan disco, no RAM.
- **Hueco en Codync:** si el bot tiene computer use, la sesión está disponible;
  no hay un ciclo de vida de recursos por bot.
- **Adaptación:** máquina de estados `stopped|starting|ready|idle|stopping`;
  despertar en la primera herramienta; apagar tras N minutos sin uso; nunca
  apagar con un turno activo; exponer el estado en el panel (ficha 26).
- **Esfuerzo:** S. **Impacto:** medio. **Depende de:** 19, 20.
- **Aceptación:** un bot sin uso no mantiene un contenedor corriendo; la primera
  herramienta despierta en menos de X segundos o devuelve `starting`.

#### 25. Takeover con snapshot fresco y tarjeta para secretos

- **Fuente:** OpenDots — `docs/COMPUTERS.md` (*Take control*: pausa la entrada del
  agente; al liberar, el agente debe obtener un snapshot fresco); openbot —
  `src/backend/agent/attention-registry.ts`, `browser-identity.ts`,
  `docs/architecture/skills-and-plugins.md` (tarjeta de contraseña/código).
- **Qué hace:** cuando la persona toma el control, el agente deja de escribir. Al
  devolverlo, no puede actuar con referencias viejas: necesita un snapshot nuevo.
  Para contraseñas o códigos, el host abre una tarjeta y rellena el valor sin que
  el modelo lo vea; si el agente ejecutó scripts en ese origen, la tarjeta es
  obligatoria.
- **Hueco en Codync:** el takeover existe y bloquea herramientas que no sean de
  lectura, pero no hay época de snapshot ni regla de "snapshot fresco" posterior.
- **Adaptación:** `takeover_epoch` por bot; las herramientas de acción llevan
  `snapshot_id` y el host rechaza uno anterior a la época; la tarjeta de secreto
  reutiliza `market/passwords.rs` y nunca devuelve el valor al agente.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 23, 27.
- **Aceptación:** tras un takeover, una acción con snapshot viejo falla; el
  secreto se rellena y el transcript no lo contiene.

#### 26. Panel de computadora: estado, archivos, terminal y actividad

- **Fuente:** OpenDots — `src/client/ComputerPanel.tsx`,
  `ComputerToolCard.tsx`, `src/server/computer-routes.ts`,
  `src/shared/computer-types.ts`; README, *Dot computers*.
- **Qué hace:** un panel muestra el estado de la computadora, el navegador, los
  archivos, la salida de terminal y la actividad, con permisos por bot. Las
  tarjetas de herramienta muestran el navegador en vivo y el archivo guardado
  dentro del chat.
- **Hueco en Codync:** hay ajustes de computer access y una vista de pantalla
  remota, pero no un panel único con estado, archivos, terminal y actividad.
- **Adaptación:** una vista "Computadora" en el detalle del bot, con pestañas
  `Estado`, `Archivos`, `Terminal`, `Actividad`; reutilizar métodos existentes de
  archivos y pantalla; en iPhone/Android, solo estado, archivos y actividad
  (terminal en escritorio/TUI); en TUI, `c`.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** 24, 27, 28.
- **Aceptación:** el panel muestra el estado real y sobrevive a reconexiones; las
  acciones de archivo respetan permisos; la actividad se actualiza en vivo.

#### 27. Permisos por capacidad con revocación en caliente

- **Fuente:** OpenDots — `docs/COMPUTERS.md` (browser, workspace-file y shell por
  Dot; empiezan deshabilitados; revocar cancela la solicitud activa).
- **Qué hace:** el bot tiene tres permisos independientes, no un solo "computer
  use". El servidor los comprueba en cada llamada, no el cliente. Revocar impide
  acciones nuevas y cancela la solicitud en curso; no deshace efectos ya
  completados.
- **Hueco en Codync:** `BotConfig.computer: bool` es todo o nada; el harness
  además aplica sus propias aprobaciones.
- **Adaptación:** `computer_permissions {browser, files, shell}` en `BotConfig`;
  comprobación en `computerTools` antes de ejecutar; `CancellationToken` por bot;
  la UI explica que revocar no revierte. Mantener el permiso global como
  interruptor maestro.
- **Esfuerzo:** S. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** con `shell` off, una llamada de shell devuelve `denied` y no
  arranca proceso; revocar durante una llamada la cancela.

#### 28. Auditoría de acciones sin datos sensibles

- **Fuente:** OpenDots — `docs/COMPUTERS.md` (*Activity*: nombres de acción,
  quién la pidió, éxito/fallo; excluye valores tecleados, contenido de archivos y
  comandos completos; 1.000 registros por Dot).
- **Qué hace:** hay un rastro consultable de qué hizo el bot y quién lo pidió, sin
  filtrar secretos ni convertir el log en una copia del contenido.
- **Hueco en Codync:** el trace del harness muestra pasos, pero no hay un registro
  de auditoría del host con actor y resultado, ni retención definida.
- **Adaptación:** `computer_activity(id, bot_id, action, actor_kind, actor_id,
  result, redacted_args, created_at)`; redactar valores y comandos; retención por
  número; exponer en el panel y en un CSV exportable.
- **Esfuerzo:** S. **Impacto:** medio-alto. **Depende de:** 27.
- **Aceptación:** una acción con contraseña no guarda el valor; el usuario puede
  ver quién lanzó una acción y cuándo.

#### 29. Grabar una sesión y convertirla en skill

- **Fuente:** rakazo — `packages/core/src/teach-playbook.ts`,
  `teach-recording.ts`, `apps/api/src/taught-skills.ts`, migración
  `0010_taught_skills`; docs `agent-verification.md` (grabaciones de replay).
- **Qué hace:** se graban punteros, teclas, portapapeles, scrolls y snapshots; los
  eventos sensibles se sanean; un generador produce un playbook con *cuándo
  usarlo, entradas, pasos, cómo comprobar, qué devolver, límites de aprobación y
  manejo de fallos*; el usuario lo revisa antes de instalarlo como skill.
- **Hueco en Codync:** las skills se instalan desde un catálogo o se escriben a
  mano; no se puede "enseñar" un flujo haciendo clic.
- **Adaptación:** grabar desde Codync Screen (`screen/helper.rs` ya envía eventos);
  guardar `recordings(id, bot_id, goal, events_json, snapshots_json)`; generar el
  playbook con el modelo y una plantilla; sanear texto con `password|secret|token|
  api_key`; instalar como skill existente tras aprobación.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 25, 26.
- **Aceptación:** una grabación con una contraseña no la guarda; el playbook
  reproduce el flujo en un test; el usuario puede editar antes de instalar.

#### 30. Navegador CDP con snapshot de accesibilidad y refs

- **Fuente:** openbot — `src/backend/browser-cdp-snapshot.ts`,
  `browser-cdp.ts`, `browser-cdp-input.ts`, `browser-tool-actions.ts`,
  `browser-fill-fields.ts`, `browser-tab-operations.ts`,
  `browser-navigation.ts`, `browser-recorder.ts`.
- **Qué hace:** controla Chromium por CDP; el snapshot devuelve texto y
  referencias de elementos; las acciones usan refs y rechazan refs obsoletas; los
  formularios se rellenan campo a campo; hay pestañas, descargas, capturas,
  grabación y PiP.
- **Hueco en Codync:** `cua-driver` actúa por accesibilidad o píxeles sobre apps;
  para la web es más lento y frágil que un DOM con refs. No hay modo navegador
  dedicado.
- **Adaptación:** un servidor MCP `browser` respaldado por CDP, con perfil por bot
  (ficha 22); `browser_snapshot` devuelve `{url, title, text, elements:[{ref,
  role, name}]}`; `browser_act` acepta `ref` y rechaza si cambió; captura como
  fallback. No reemplaza a `cua-driver`: lo complementa para apps nativas.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 22.
- **Aceptación:** navegar, rellenar y enviar un formulario de prueba sin píxeles;
  una ref vieja devuelve error; el perfil conserva la sesión tras reiniciar.

### Bloque D — Herramientas, aprobaciones y secretos (31-39)

La diferencia entre un asistente y un agente peligroso es el contrato de
aprobación. Este bloque hace explícito qué puede hacer cada herramienta, con qué
credencial y con qué garantía de una sola ejecución.

#### 31. Política "Ask first" por herramienta MCP

- **Fuente:** OpenDots — `src/server/connection-tools.ts`,
  `connection-store.ts`, `connections.ts`, `ConnectionActionCard.tsx`,
  `docs/CONNECTIONS.md`.
- **Qué hace:** cada herramienta de una conexión empieza habilitada. Si el
  servidor la marca `readOnlyHint`, corre sola; si no, empieza en *Ask first*. Al
  llamarla, no se ejecuta: el servidor guarda conexión, herramienta y argumentos,
  muestra una tarjeta y solo ejecuta con *Approve & run* por una ruta de owner.
  La aprobación caduca en una hora, corre como máximo una vez y solo si la
  herramienta sigue habilitada.
- **Hueco en Codync:** las aprobaciones vienen del harness ACP; el host no tiene
  política por herramienta ni guarda los argumentos exactos. Cambiar de harness
  cambia el comportamiento.
- **Adaptación:** `connector_tool_policies(connector_id, tool_name, mode)` donde
  `mode = allow|ask|deny`; `pending_tool_calls(id, bot_id, connector_id,
  tool_name, args_json, status, expires_at)`; el proxy MCP del host
  (`codync-host mcp remote`) consulta la política y, si es `ask`, devuelve una
  tarjeta en vez de ejecutar. `resolveToolCall` ejecuta el JSON guardado, no el
  que manda el cliente. Las tarjetas se renderizan en los cuatro clientes; en
  Slack/TUI se indica continuar en la app.
- **Esfuerzo:** L. **Impacto:** muy alto. **Depende de:** nada.
- **Aceptación:** una herramienta `ask` no se ejecuta sin aprobación; aprobar
  ejecuta exactamente los argumentos guardados; expirar no ejecuta; deshabilitar
  la herramienta entre la tarjeta y la aprobación la rechaza.

#### 32. Secretos reutilizables con origen fijado

- **Fuente:** rakazo — `docs/bot-secrets.md`, `apps/api/src/agent-secrets.ts`,
  `apps/api/src/bot-secrets-router.test.ts`.
- **Qué hace:** el bot llama `request_secret` con nombre, propósito, origen HTTPS
  y tipo de auth; el usuario guarda el valor en una tarjeta protegida. El valor se
  cifra con AES-256-GCM atado al registro, pertenece a usuario/espacio/bot y
  sobrevive a otros runs. `list_secrets` devuelve solo metadatos; `secret_request`
  inyecta la credencial en una petición al origen exacto. Soporta bearer, header
  con nombre y basic. Rechaza redirects y destinos inseguros; límites 100 por bot
  y 16 KB; el modelo no tiene herramienta para leer el valor.
- **Hueco en Codync:** `market/vault.rs` cifra credenciales de conectores y
  `market/passwords.rs` lee referencias de 1Password, pero no existe un
  `request_secret` genérico para que un bot pida una API key al usuario.
- **Adaptación:** reutilizar el vault (XChaCha20-Poly1305 ya está); tabla
  `bot_secrets(id, bot_id, user_id, name, origin, auth_kind, auth_header,
  ciphertext, created_at)`; herramienta `request_secret`; ejecutor
  `secret_request` en el host con allowlist de origen, timeout 30 s, límite 1 MB
  y redacción de ecos. Tarjeta en los cuatro clientes.
- **Esfuerzo:** L. **Impacto:** muy alto. **Depende de:** 37.
- **Aceptación:** el valor nunca aparece en el transcript ni en el log; una
  petición a otro origen falla; tras reiniciar, la referencia sigue funcionando.

#### 33. Variables de comando por bot

- **Fuente:** rakazo — `docs/bot-secrets.md`, sección *Command variables*.
- **Qué hace:** cuando el valor debe estar en el shell y no en un header, se
  guarda como credencial de comando. El nombre se exporta en mayúsculas con
  guiones como guiones bajos (`netbird-setup-key` → `$NETBIRD_SETUP_KEY`), se
  carga fresco antes de cada comando, la variable del bot gana a la del espacio y
  si falla se deja sin definir sin exponer el error.
- **Hueco en Codync:** no hay variables de entorno por bot gestionadas por el
  host; el harness hereda el entorno del proceso.
- **Adaptación:** `bot_env(id, bot_id, name, ciphertext)` en el mismo vault;
  inyectar solo en el ejecutor de shell del host, nunca en el prompt; precedencia
  explícita; redactar en logs y en la salida de comandos.
- **Esfuerzo:** M. **Impacto:** medio. **Depende de:** 32.
- **Aceptación:** el comando ve la variable; el agente no puede leerla con
  `env` desde otra herramienta; borrarla deja de aplicarse en el siguiente
  comando.

#### 34. Reglas de aprobación con auto-review

- **Fuente:** rakazo — `packages/core/src/action-approval.ts`,
  migración `0011_action_approval_rules`, VISION (*uncertain automated review
  fails toward asking*).
- **Qué hace:** reglas por bot y herramienta (`always allow`, `ask`, `deny`), con
  una revisión automática solo para casos de bajo riesgo; si la revisión duda,
  pide aprobación en vez de actuar.
- **Hueco en Codync:** no hay reglas; cada harness decide. El usuario no puede
  decir "git status siempre, git push nunca sin preguntar".
- **Adaptación:** `approval_rules(id, bot_id, tool_pattern, risk, mode,
  created_at)`; el host evalúa antes de la herramienta; `auto` solo si la regla
  lo permite y el modelo revisor devuelve alta confianza; la tarjeta muestra la
  regla aplicada y permite cambiarla.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 31.
- **Aceptación:** una regla `deny` bloquea sin tarjeta; `ask` muestra tarjeta;
  `auto` con duda muestra tarjeta; cambiar la regla aplica en la siguiente
  llamada.

#### 35. Claves de efecto para aprobaciones idempotentes

- **Fuente:** rakazo — `packages/core/src/approval-effect-key.ts`; tests
  `approval-effect-key.test.ts`.
- **Qué hace:** cada aprobación lleva una clave de efecto; ejecutarla dos veces
  no repite el efecto y el resultado se recupera.
- **Hueco en Codync:** no existe; se solapa con la ficha 6 pero aplica a
  herramientas, no solo a documentos.
- **Adaptación:** `effect_key` en `pending_tool_calls` y en `pending_actions`;
  tabla `effects`; el ejecutor hace `INSERT OR IGNORE` y devuelve el resultado
  guardado; cubrir también rutinas y envíos de archivo.
- **Esfuerzo:** S. **Impacto:** alto. **Depende de:** 6, 31.
- **Aceptación:** doble aprobación = un solo efecto; el segundo cliente recibe el
  mismo resultado.

#### 36. Idempotencia de llamadas de herramienta

- **Fuente:** openbot — `src/backend/agent/tool-call-idempotency.ts`.
- **Qué hace:** asocia el id de llamada del proveedor a un efecto y evita repetir
  la misma llamada cuando el modelo reintenta o el stream se corta.
- **Hueco en Codync:** `entries.find_by_nonce` deduplica mensajes de usuario, no
  llamadas de herramienta.
- **Adaptación:** `tool_calls(id, bot_id, turn_id, provider_call_id, tool_name,
  args_hash, result, created_at)`; el router del host consulta antes de ejecutar;
  las herramientas no idempotentes (enviar correo, pagar) exigen clave.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 35.
- **Aceptación:** un reintento del proveedor no duplica el efecto; el resultado
  guardado se devuelve al modelo.

#### 37. Política de egreso y destinos remotos

- **Fuente:** rakazo — `packages/core/src/egress.ts`; `docs/bot-secrets.md`
  (redirects y destinos inseguros rechazados; metadata y link-local bloqueados).
- **Qué hace:** centraliza qué hosts puede alcanzar una herramienta: HTTPS,
  bloqueo de link-local y metadata de nube, sin redirects, timeout y tamaño
  máximo, con allowlist por integración.
- **Hueco en Codync:** el proxy MCP y Composio salen a la red sin una política
  única; cada conector resuelve sus límites.
- **Adaptación:** `egress.rs` con `Policy {allow, deny, allow_private}`; resolver
  DNS y comprobar la IP final (evitar rebinding); aplicar a `secret_request`,
  webhooks salientes, descargas de skills y conectores; log de decisión.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** una URL a `169.254.169.254` se rechaza; un redirect a otro host
  no se sigue; una allowlist deja pasar solo lo declarado.

#### 38. Importar agentes de otras apps

- **Fuente:** openbot — `src/main/agent-import-service.ts`,
  `agent-import-manifest.ts`, `hermes-import-reader.ts`,
  `hermes-import-service.ts`.
- **Qué hace:** lee un manifiesto de otro producto (nombre, instrucciones, modelo,
  skills, rutinas, memoria) y crea el agente con un informe de conflictos; no
  importa secretos.
- **Hueco en Codync:** no hay importación; un usuario que viene de OpenClaw,
  Hermes o un export propio pierde su configuración.
- **Adaptación:** `importManifest {json}` con dry-run, mapeo de conectores por
  nombre y reporte de lo que no se pudo importar; nunca importar credenciales;
  CLI `codync-host import --dry-run`; UI en Ajustes.
- **Esfuerzo:** M. **Impacto:** medio. **Depende de:** nada.
- **Aceptación:** un manifiesto de prueba crea bots, skills y rutinas; un secreto
  en el manifiesto se ignora y se informa; el dry-run no escribe nada.

#### 39. MCP con OAuth aislado y redacción de logs

- **Fuente:** openbot — `src/backend/mcp-oauth-provider.ts`,
  `mcp-oauth-proxy.ts`, `mcp-oauth-store.ts`, `mcp-redaction.ts`,
  `mcp-schema.ts`, `mcp-probe.ts`.
- **Qué hace:** cada servidor MCP tiene su propio token OAuth, su redirect y su
  almacén; los headers y variables de entorno se enmascaran en logs y en la UI;
  el esquema se valida y se prueba antes de instalar.
- **Hueco en Codync:** `market/oauth.rs` y `market/vault.rs` cubren el flujo, pero
  no hay una pasada de redacción central ni probe de esquema antes de instalar.
- **Adaptación:** un `redact()` aplicado a logs, errores y trace; token por
  servidor con refresco; `probeConnector` que lista herramientas y valida el
  esquema; mostrar solo nombres y descripciones, nunca valores.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** 31.
- **Aceptación:** un token en un header no aparece en logs ni en la UI; un
  servidor con esquema inválido no se instala; refrescar el token no reinicia el
  bot.

### Bloque E — Rutinas, eventos y automatización (40-46)

Codync ya tiene cron, intervalos, webhooks y eventos. Este bloque añade
condiciones, composición y operación.

#### 40. Webhooks con filtros JSON Pointer, plantillas y recibos

- **Fuente:** openbot — `src/backend/webhook-trigger.ts`,
  `src/main/host-events-service.ts`, `docs/architecture/events.md`,
  `docs/webhooks.md`.
- **Qué hace:** un trigger webhook tiene URL pública, tipo de evento opcional y
  hasta 128 filtros JSON Pointer que comparan un escalar; todos deben coincidir.
  Las plantillas de payload no ejecutan código. Se guardan recibos sin cuerpo, se
  rota el secreto y se encola la revocación. Límite de 64 rutas por host.
- **Hueco en Codync:** ya hay webhooks por rutina, firma GitHub y filtros por mapa
  de campos; faltan punteros anidados, plantillas, recibos y una revocación
  explícita.
- **Adaptación:** extender `Trigger::Event.filters` a
  `[{pointer, equals}]`; `payload_template` con `{{path}}`; tabla
  `webhook_receipts(route_id, received_at, status, bytes)` sin cuerpo; al borrar,
  encolar revocación; UI para rotar el secreto.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** nada.
- **Aceptación:** un filtro `data.pull_request.merged == true` dispara solo en
  merges; un recibo quedita sin body; rotar invalida el secreto anterior.

#### 41. Flujos de rutina entre varios bots

- **Fuente:** openbot — `src/backend/routine-flows/routine-flow-graph.ts`,
  `routine-flow-store.ts`, `routine-flow-tools.ts`, `routine-flow-routines.ts`.
- **Qué hace:** una rutina puede encadenar agentes: empieza en el dueño y sigue
  enlaces hacia adelante. Se calcula la profundidad, se rechazan ciclos y enlaces
  al dueño, y cada agente ejecuta su paso con la salida del anterior.
- **Hueco en Codync:** una rutina pertenece a un bot; para encadenar hay que
  crear varias rutinas y coordinarlas a mano.
- **Adaptación:** `routine_flows(routine_id, from_bot_id, to_bot_id, position)`;
  validación DAG al guardar; al ejecutar, cada paso es un turno en cola y el
  resultado viaja como contexto; la UI muestra el grafo en solo lectura y luego
  editable.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 15, 43.
- **Aceptación:** un flujo de tres pasos produce tres turnos en orden; un ciclo se
  rechaza al guardar; el fallo de un paso detiene el flujo y lo reporta.

#### 42. Rutinas "watch" que despiertan solo si algo cambió

- **Fuente:** rakazo — `packages/adapters/src/routine-watch.ts`, migración
  `20261019000000_routine_watch`, `docs/agent-verification.md` (replay).
- **Qué hace:** un checker calcula una huella de lo observado (árbol de archivos,
  URL, consulta a un conector) y la compara con la última; nunca llama al modelo.
  Si cambió, dispara la rutina; si el equipo está ocupado, reintenta; si falla,
  pausa y explica. Límite de 2.000 entradas por huella.
- **Hueco en Codync:** los triggers son tiempo o evento, no "cuando cambie esto".
- **Adaptación:** `routine_watch(kind, target, every_minutes, state_json)`;
  `kind = files|url|connector`; hash SHA-256; no despertar si el hash es igual;
  estado visible en la rutina; presupuesto por comprobación.
- **Esfuerzo:** M. **Impacto:** alto. **Depende de:** 43.
- **Aceptación:** tocar un archivo dispara una sola ejecución; sin cambios no hay
  run ni coste de modelo; un fallo de red reintenta y luego pausa con motivo.

#### 43. Resultados de rutina estructurados

- **Fuente:** openbot — `src/backend/routine-results/routine-results.ts`,
  `routine-result-store.ts`, `routine-result-schema.ts`,
  `src/main/ipc/routine-result-handlers.ts`.
- **Qué hace:** cada ejecución guarda estado, resumen, resultado JSON, error y
  tiempos; el cliente lista y filtra sin leer el chat.
- **Hueco en Codync:** el resultado aparece como entry del chat; no hay registro
  consultable ni enlace estable.
- **Adaptación:** `routine_runs(id, routine_id, status, summary, result_json,
  started_at, ended_at, error)`; retención configurable; enlace a artefactos;
  filtros por rutina/estado/fecha en los clientes y en TUI.
- **Esfuerzo:** S. **Impacto:** medio. **Depende de:** nada.
- **Aceptación:** una ejecución fallida queda listada con su error; el detalle
  abre el chat en el punto correcto; borrar la rutina no borra el historial.

#### 44. Ejecuciones silenciosas sin novedades

- **Fuente:** openbot — `src/backend/agent/routine-quiet-runs.ts`; capacidad
  `quiet-turn-v1` en `packages/contracts`.
- **Qué hace:** una rutina programada puede terminar sin novedades con
  `[[no-update]]` o la herramienta `routine_no_update`. El turno no cuenta como
  no leído, no cambia la vista previa, no notifica y no suena; el historial se
  conserva. Una respuesta con texto adicional sí se muestra.
- **Hueco en Codync:** toda rutina produce un mensaje y una notificación; un
  chequeo cada hora genera ruido.
- **Adaptación:** misma herramienta y marcador; `quiet: true` en el evento de
  finalización; los clientes suprimen notificación y preview; el historial y el
  run quedan. Un test run nunca es silencioso.
- **Esfuerzo:** S. **Impacto:** alto. **Depende de:** 43.
- **Aceptación:** diez ejecuciones sin novedad no generan notificaciones; una con
  texto sí; el historial muestra las diez.

#### 45. Calendario ICS de rutinas

- **Fuente:** openbot — `src/main/routine-feed-ics.ts`,
  `src/main/routine-feed-server.ts`.
- **Qué hace:** sirve un feed ICS con las rutinas y su recurrencia para
  suscribirse desde un calendario externo, con token en la URL.
- **Hueco en Codync:** las rutinas se ven dentro de la app; no se pueden
  suscribir en Calendar/Outlook.
- **Adaptación:** `GET /routines.ics?token=...`; un `VEVENT` por rutina con
  `RRULE` derivada de cron o intervalo; solo lectura; token rotable; excluir
  rutinas pausadas o marcarlas como `CANCELLED`.
- **Esfuerzo:** S. **Impacto:** medio. **Depende de:** nada.
- **Aceptación:** Calendar muestra la rutina en la hora y zona correctas; rotar
  el token invalida el feed anterior.

#### 46. API local de automatización y CLI estable

- **Fuente:** openbot — `src/main/automation-server.ts`,
  `src/backend/automation-command.ts`, `docs/automation.md`.
- **Qué hace:** mientras al menos un agente lo permite, escucha solo en
  `127.0.0.1` en un puerto libre y escribe `url`, `token` y `headers` en una
  carpeta `0700` con archivos `0600`. Rechaza cualquier `Origin` y un `Host`
  ajeno antes de mirar el token. Permite listar agentes, enviar mensaje, ejecutar
  rutina, consultar estado, responder preguntas y aprobaciones, cambiar modelo o
  cuenta y abrir chat. El CLI devuelve códigos 0-5 y soporta `--wait`, `--json` y
  `--id` como clave de reintento.
- **Hueco en Codync:** existe `codync-host send` y el TUI, pero no una API local
  documentada, estable y con archivos de descubrimiento; un script tiene que
  conocer detalles internos.
- **Adaptación:** flag `allowAutomation` por bot (off por defecto); listener
  loopback con las mismas defensas; `codync-cli` con `list`, `run`, `status`,
  `agent set-model`, `agent new-chat`; rutas `POST /v1/bots/:id/messages`,
  `POST /v1/bots/:id/routines/:rid/run`, `GET /v1/pending`, `POST /v1/pending/:id`.
  Documentar en `docs/features/automation.md`.
- **Esfuerzo:** M. **Impacto:** muy alto. **Depende de:** 17, 31.
- **Aceptación:** un script envía y espera la respuesta; un `Origin` malicioso
  recibe 403; matar y reiniciar el host rota el token sin romper el script;
  los códigos de salida distinguen "espera persona" de "timeout".

### Bloque F — Memoria, contexto y seguridad del dato (47-52)

Codync ya tiene memoria propia, sueños y búsqueda de historial. Falta hacerla
portable, acotada y cifrada.

#### 47. Proveedor de memoria intercambiable con presupuesto

- **Fuente:** openbot — `src/backend/agent/agent-memory-selection.ts`,
  `agent-memory-store.ts`, `agent-memory-limit-preference-store.ts`,
  `host-memory.ts`; rakazo — `packages/adapters/src/memory-context.ts`,
  `memory-provider-factory.ts`, `honcho-memory-provider.ts`,
  `supermemory-memory-provider.ts`, `serenity-memory-provider.ts`,
  `memory-tools.ts`.
- **Qué hace:** la memoria es un proveedor con contrato (`search`, `write`,
  `forget`, presupuesto) y el host elige uno por bot. El presupuesto limita
  cuánta memoria entra al prompt; si el proveedor falla, el turno sigue sin
  memoria en vez de romperse.
- **Hueco en Codync:** `crates/memory` y `chat/memory/` son la única
  implementación; no hay selección por bot ni presupuesto explícito, y el fallo
  de una consulta no está tipado.
- **Adaptación:** trait `MemoryProvider` con `LocalMemory` (el actual) y un
  adaptador externo opcional; `memory_provider` y `memory_budget_tokens` en
  `BotConfig`; degradación silenciosa con aviso en el trace; nunca enviar
  memoria a un proveedor externo sin consentimiento (ficha 51).
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** nada. Un proveedor
  externo exige además el consentimiento de la ficha 51.
- **Aceptación:** cambiar de proveedor no pierde la memoria local; un proveedor
  caído no tumba el turno; el presupuesto se respeta.

#### 48. Historial acotado con recuperación y catálogo de navegación

- **Fuente:** rakazo — `docs/context-and-evals-tasks.md`: conservar el historial
  cargado si cabe en `min(16.384 tokens, un cuarto del presupuesto de entrada)`;
  si no, historial reciente acotado más un catálogo de navegación y
  `history_search` con cursor.
- **Qué hace:** no envía toda la conversación; mantiene lo reciente, ofrece
  herramientas para recuperar mensajes originales y mide el coste de cada
  estrategia antes de cambiar el default.
- **Hueco en Codync:** `search_messages` existe y el harness compacta, pero el
  host no decide un historial activo ni reinyecta originales; depende del
  harness.
- **Adaptación:** resumen por lane + `history_search`/`fetch_messages` ya
  existentes; presupuesto de contexto (ficha 18); conservar originales en SQLite
  y no reescribirlos; probar con conversaciones largas reales antes de fijar el
  umbral.
- **Esfuerzo:** M. **Impacto:** medio. **Depende de:** 18, 47.
- **Aceptación:** una conversación de 100k tokens sigue respondiendo con
  evidencia recuperada; el coste por turno no crece linealmente.

#### 49. Cifrado en reposo con claves por espacio

- **Fuente:** rakazo — `docs/content-encryption.md`, `packages/db` (key ring,
  `$sealed`, `contentEncryptionOf`), `packages/adapters/src/artifacts.ts`
  (segmentos de 64 KiB).
- **Qué hace:** cifra mensajes, eventos, resúmenes, memoria y archivos antes de
  llegar a la base. Cada espacio tiene una clave de datos de 256 bits envuelta
  por una clave maestra; el ciphertext se ata a tabla, columna, fila y espacio
  con AAD, así que moverlo falla. Los artefactos se cifran por segmentos con
  nonce y autenticación del header, el espacio, el archivo y el último segmento.
- **Hueco en Codync:** el transporte es E2E y las credenciales están en el
  vault, pero `codync.db` guarda transcripts, memoria y documentos en claro. Quien
  copie el archivo ve todo.
- **Adaptación:** clave de datos por bot (o espacio) envuelta por una maestra
  guardada en el llavero del SO; XChaCha20-Poly1305 (ya es dependencia de
  `market/vault.rs`); AAD `table|column|row|bot`; cifrar `entries.data`,
  `memory_*`, documentos y artefactos; migración en segundo plano tolerante a
  filas antiguas; rotación de clave con versiones.
- **Esfuerzo:** XL. **Impacto:** muy alto. **Depende de:** nada.
- **Aceptación:** una copia de `codync.db` sin la clave no revela mensajes; mover
  un ciphertext a otra fila falla; la app sigue leyendo filas antiguas durante la
  migración.

#### 50. Búsqueda sobre contenido cifrado

- **Fuente:** rakazo — `docs/content-encryption.md`, sección *Search*.
- **Qué hace:** como SQLite/Postgres no pueden hacer full-text sobre ciphertext,
  guarda tokens HMAC por espacio (palabra completa y prefijos de 2 a 10
  caracteres) con índice, recupera candidatos, descifra y filtra. Acepta el
  tradeoff de filtrado de frecuencia y colisiones.
- **Hueco en Codync:** la búsqueda (`search_messages`) depende de SQL directo
  sobre texto claro; si se cifra (ficha 49) deja de funcionar.
- **Adaptación:** `search_tokens(entry_id, token)` con índice; HMAC con clave
  derivada por bot; prefijos; límite de candidatos (500) y descifrado; fallback a
  los 200 mensajes recientes para consultas de un carácter; documentar la fuga
  de frecuencia.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 49.
- **Aceptación:** buscar una palabra recupera mensajes cifrados; no se puede
  buscar por el medio de una palabra; una consulta común no devuelve todo.

#### 51. Consentimiento de datos de IA por cuenta, espacio y proveedor

- **Fuente:** rakazo — `packages/core/src/ai-consent.ts`,
  `docs/ai-data-sharing-review.md`, migración de consentimiento.
- **Qué hace:** antes de subir un adjunto, enviar un mensaje, arrancar una rutina
  o mandar voz desde móvil, el cliente pide permiso y guarda el consentimiento
  por cuenta, espacio, destinatario y versión del aviso. Retirarlo afecta solo a
  acciones nuevas.
- **Hueco en Codync:** la analítica tiene consentimiento por dispositivo, pero no
  existe un consentimiento de compartición de contenido con proveedores de IA.
- **Adaptación:** `ai_consent(user_id, scope, recipient, disclosure_version,
  granted_at, revoked_at)`; el host lo exige para acciones móviles; enlace a la
  política; texto por proveedor; no bloquear trabajo ya autorizado.
- **Esfuerzo:** M. **Impacto:** medio-alto (requisito de App Store).
- **Depende de:** nada.
- **Aceptación:** sin consentimiento no se sube el adjunto; revocar no cancela lo
  ya enviado; el aviso nombra destinatario y propósito.

#### 52. Panel de categorías de contexto

- **Fuente:** openbot — `src/backend/claude-context-categories.ts`; rakazo —
  `docs/context-and-evals-tasks.md` (medición por estrategia).
- **Qué hace:** desglosa el contexto en sistema, herramientas, historial,
  archivos y salida reservada, para que el usuario entienda por qué se agota.
- **Hueco en Codync:** `usage.rs` muestra tokens, no categorías; el usuario no
  sabe qué los consume.
- **Adaptación:** evento `context` con `{used, limit, categories}` calculado por
  el host a partir del snapshot de instrucciones, la lista de herramientas y el
  historial; panel en el detalle del bot; sin cambio de comportamiento.
- **Esfuerzo:** S. **Impacto:** medio. **Depende de:** 18.
- **Aceptación:** el panel coincide con el uso real; al desactivar una skill baja
  la categoría de herramientas en el siguiente turno.

### Bloque G — Canales y mensajería (53-56)

Un bot que solo vive dentro de la app pierde la mitad de su valor. Este bloque lo
lleva a Slack, Telegram y Discord con reglas de autorización claras.

#### 53. Slack de primera clase

- **Fuente:** OpenDots — `src/server/slack-channel.ts`, README (*Connections* y
  canales); openbot — `src/backend/messaging/slack/slack-connect.ts`,
  `slack-driver.ts`, `slack-events.ts`, `slack-events-transport.ts`,
  `slack-orchestrator.ts`, `slack-render.ts`, `slack-web-api.ts`.
- **Qué hace:** DMs, menciones y respuestas en hilo se mapean a conversaciones del
  bot; se verifica la firma de Slack; se renderizan respuestas y se publican
  estados; un canal puede tener un bot asignado.
- **Hueco en Codync:** Slack solo aparece como fuente de eventos para rutinas;
  no hay app, ni OAuth, ni hilos, ni respuestas.
- **Adaptación:** `channels(id, kind, bot_id, external_id, workspace_id,
  allowed_roles, created_at)` y `channel_threads(channel_id, external_thread,
  bot_id, lane)`; `MessagingDriver` (ficha 54); manifest de Slack con scopes
  mínimos (`chat:write`, `im:history`, `channels:history`, `app_mentions:read`);
  firma con `X-Slack-Signature`; render Markdown → blocks; nunca volcar secretos.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 54.
- **Aceptación:** mencionar al bot en un canal crea una conversación y responde en
  hilo; un mensaje sin mención no despierta al bot; una firma inválida se rechaza.

#### 54. Telegram y Discord con un driver común

- **Fuente:** openbot — `src/backend/messaging/telegram/*`,
  `src/backend/messaging/discord/*`, `messaging-service.ts`,
  `messaging-chunks.ts`, `messaging-mentions.ts`, `messaging-status-posts.ts`,
  `messaging-types.ts`.
- **Qué hace:** un contrato único (`connect`, `events`, `send`, `render`,
  `chunk`, `mentions`) con implementaciones por proveedor; los mensajes largos se
  parten, las menciones se resuelven y los estados se publican.
- **Hueco en Codync:** no hay integración con Telegram ni Discord.
- **Adaptación:** trait `MessagingDriver`; `TelegramDriver` con long polling
  (sin webhook público) y `DiscordDriver` con gateway; allowlist de chats y
  roles; rate limits por proveedor; reintentos con backoff; dedupe por id de
  mensaje.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** nada.
- **Aceptación:** un mensaje de Telegram llega al bot y responde; un chat no
  autorizado se ignora; un mensaje de 5.000 caracteres se parte sin perder orden.

#### 55. Memoria por canal y roles permitidos

- **Fuente:** openbot — `src/backend/channel-memory-store.ts`,
  `src/backend/messaging-memory-store.ts`,
  `src/main/ipc/channel-memory-handlers.ts`,
  `src/backend/messaging-allowed-roles-migration.test.ts`.
- **Qué hace:** cada canal tiene su propia memoria, distinta de la del bot; solo
  los roles permitidos pueden encargar trabajo; el prompt del canal añade sus
  reglas sin contaminar el chat principal.
- **Hueco en Codync:** la memoria es del bot o compartida; no hay memoria de
  canal ni control de roles por remitente.
- **Adaptación:** `channel_memories(channel_id, body, created_at)` y
  `allowed_roles` en `channels`; el prompt del turno de canal incluye solo esa
  memoria; un remitente no autorizado recibe una negativa breve y no crea turno.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** 53, 54.
- **Aceptación:** dos canales del mismo bot tienen memorias distintas; un
  invitado no puede lanzar rutinas; el chat principal no ve la memoria del canal.

#### 56. Recibos de entrega, comandos y respuestas tipadas

- **Fuente:** rakazo — `apps/api/src/messaging-receipts.ts`,
  `messaging-inbound.ts`, `packages/core/src/messaging-commands.ts`,
  `packages/core/src/answerable-ask.ts`, `typed-reply.postgres.test.ts`.
- **Qué hace:** registra el recibo de entrega del proveedor, deduplica por id,
  interpreta comandos (`/new`, `/stop`, `/status`) y permite responder una
  tarjeta solo si su run está en `waiting_input`, con acciones por id o etiqueta.
- **Hueco en Codync:** las tarjetas ACP tienen opciones, pero no hay recibos de
  canal ni comandos; una respuesta tardía puede quedar huérfana.
- **Adaptación:** `channel_receipts(channel_id, external_id, state, updated_at)`;
  parser de comandos antes de crear el turno; `answerable` calculado por el host
  (nunca por el cliente); una respuesta a una tarjeta cerrada se rechaza con un
  mensaje claro.
- **Esfuerzo:** M. **Impacto:** medio. **Depende de:** 53, 54.
- **Aceptación:** un mensaje duplicado no crea dos turnos; `/stop` detiene el
  turno; una respuesta a una tarjeta expirada no se aplica.

### Bloque H — Voz, transparencia y ecosistema (57-60)

#### 57. Recuperación de llamadas, nonce y grupos de voz

- **Fuente:** rakazo — `packages/core/src/call-request.ts`,
  `call-nonce.ts`, `voice-chat-groups.ts`, `barge-in.ts`; migración
  `20261019010000_call_recovery_index`; `apps/api/src/calls.test.ts`.
- **Qué hace:** una llamada tiene nonce y estado; si el cliente se desconecta,
  puede recuperar la llamada en curso; el barge-in interrumpe la respuesta; los
  grupos tienen reglas de quién habla.
- **Hueco en Codync:** la llamada existe y el avatar interrumpe, pero no hay
  recuperación tras reconexión ni nonce antirreplay; el estado de llamada no es
  una entidad del host.
- **Adaptación:** `calls(id, bot_id, nonce, state, started_at, ended_at,
  last_heartbeat)`; token de reanudación; rechazar nonces repetidos; conservar
  metadatos, nunca audio; el grupo usa el mismo motor con un `speaker` por bot.
- **Esfuerzo:** M. **Impacto:** medio. **Depende de:** nada.
- **Aceptación:** cortar la red y volver reanuda la misma llamada; un nonce
  repetido se rechaza; el historial muestra duración y participantes.

#### 58. Tarjeta de actividad de herramientas

- **Fuente:** rakazo — `docs/tool-activity.md`,
  `packages/core/src/tool-activity.ts`,
  `apps/web/src/components/ToolActivityDisclosure.test.tsx`.
- **Qué hace:** una tarjeta plegable lista las últimas herramientas con conteo y
  duración; en vivo empieza abierta, al terminar se pliega a una línea; está off
  por defecto y solo cambia el render, no los datos.
- **Hueco en Codync:** el trace muestra pasos, pero no hay una tarjeta resumida ni
  una preferencia de dispositivo. En móvil el trace es difícil de leer.
- **Adaptación:** render en cliente desde los eventos de trace existentes;
  preferencia local `showToolActivity`; resumen `n herramientas · 12 s`;
  expandir para ver pasos; sin cambios de host.
- **Esfuerzo:** S. **Impacto:** medio. **Depende de:** nada.
- **Aceptación:** con la preferencia off no se muestra nada; con on, el conteo
  coincide con el trace; el render no altera el transcript.

#### 59. Generar un bot desde una descripción

- **Fuente:** openbot — `src/backend/agent/profile-generation.ts`,
  `profile-save.ts`, `profile-creation-recovery.ts`, `avatar-file.ts`,
  `src/main/agent-template-service.ts`.
- **Qué hace:** el usuario describe el bot que quiere; el host genera nombre,
  rol, instrucciones, avatar y skills sugeridas, y muestra una vista previa
  editable antes de guardar. No instala nada sin confirmación.
- **Hueco en Codync:** crear un bot es un formulario vacío con nombre,
  descripción, carpeta y agente; no hay generación ni plantillas.
- **Adaptación:** método `draftBot {description}` que devuelve un borrador
  (`name`, `description`, `instructions`, `avatar`, `skills`, `model`); el
  cliente lo muestra editable; `createBot` solo al confirmar; no ejecutar
  herramientas durante la generación.
- **Esfuerzo:** M. **Impacto:** medio-alto. **Depende de:** 8.
- **Aceptación:** una descripción produce un borrador coherente; cancelar no crea
  nada; el usuario puede cambiar todos los campos antes de guardar.

#### 60. Transferencia de instancia: exportar e importar todo

- **Fuente:** openbot — `src/main/instance-transfer-service.ts`,
  `instance-transfer-manifest.ts`, `src/main/ipc/instance-transfer-handlers.ts`.
- **Qué hace:** exporta una instancia completa (bots, instrucciones, rutinas,
  skills, memoria y ajustes) a un manifiesto portable, sin secretos, e importa en
  otra máquina con IDs nuevos y un informe de conflictos.
- **Hueco en Codync:** no hay backup lógico ni migración entre computadoras; el
  usuario depende de copiar `~/.codync` a mano, con claves y rutas absolutas.
- **Adaptación:** `exportInstance {includeFiles?}` → zip con `manifest.json` +
  datos; `importInstance {zip, dryRun}`; nunca incluir tokens, claves ni
  credenciales; reescribir `cwd` y avisar de rutas inexistentes; CLI
  `codync-host export/import`.
- **Esfuerzo:** L. **Impacto:** alto. **Depende de:** 38, 49.
- **Aceptación:** exportar en una máquina e importar en otra recrea bots, rutinas
  y skills; los secretos no viajan; el dry-run lista conflictos sin escribir.

## 8. Backlog extendido: 40 piezas adicionales

Estas piezas no entran en el top 60 por dependencia, coste o impacto, pero están
inventariadas para no volver a investigarlas. "Promover" significa que puede
convertirse en ficha si cambia la prioridad.

| # | Pieza | Repo | Ruta de referencia | Nota |
| --- | --- | --- | --- | --- |
| E1 | Tickets y tablero | rakazo | `packages/adapters/src/ticket-tools.ts`, `ticket-context.ts`, `ticket-changes.ts`, `ticket-wake.ts` | Promover si se prioriza trabajo por tickets; solapa con 9 |
| E2 | Detección de trabajo atascado | rakazo | `packages/core/src/stuck-work.ts` | Base para alertas de 9 |
| E3 | Taxonomía de estado y error de run | rakazo | `packages/core/src/run-state.ts`, `run-error.ts` | Mejora la UI de fallos |
| E4 | Catálogo perezoso de herramientas | rakazo | `packages/adapters/src/lazy-tool-catalog.ts` | Rendimiento con muchos MCP |
| E5 | Guardia de bucle de herramientas | rakazo | `packages/adapters/src/tool-loop.ts`, `pi-tool-call-guard.ts` | Seguridad y coste |
| E6 | Scratchpad efímero | rakazo | `packages/adapters/src/scratchpad-tools.ts` | Notas de turno, bajo coste |
| E7 | Gráficas desde el chat | rakazo | `packages/adapters/src/plot-tool.ts`, `packages/core/src/plot/plot-spec.ts` | Entregable visual |
| E8 | Generación de media | rakazo | `packages/adapters/src/media-generation-tools.ts` | Imagen/video, coste alto |
| E9 | Menciones y slash del compositor | rakazo | `packages/core/src/composer-mentions.ts`, `composer-slash.ts` | UX; Codync ya tiene compositor |
| E10 | Reacciones, citas y respuestas | rakazo | `packages/core/src/message-reactions.ts`, `message-quote.ts`, `message-replies.ts` | UX de chat |
| E11 | Paginación y búsqueda de hilos | rakazo | `packages/core/src/message-pages.ts`, `search.ts` | Escala de transcripts |
| E12 | Passkeys y política de contraseñas | rakazo | `packages/core/src/passkey-offer.ts`, `password-policy.ts` | Auth de la nube |
| E13 | Secretos con Infisical | rakazo | `apps/api/src/cli/secrets-infisical.ts`, `docs/infisical-secrets.md` | Empresa |
| E14 | Backups automáticos | rakazo | `infra/systemd/rakazo-backup.*`, `host-alert.py` | Operación |
| E15 | Agentes en la nube | rakazo | `packages/core/src/cloud-agent.ts`, `cloud-agent-tools.ts` | Solapa con 19-20 |
| E16 | OAuth y probe de modelos | rakazo | `packages/core/src/model-oauth.ts`, `model-probe.ts`, `model-providers.ts` | Onboarding |
| E17 | Secciones, pines y avatares animados | rakazo | `bot-sections.ts`, `bot-avatar-shapes.ts`, `avatar-motion.ts` | Codync ya tiene formas/pines |
| E18 | Runtime headless | OpenDots | `src/server/headless.ts`, `runner.ts` | Servidor sin UI |
| E19 | Investigación con fuentes | OpenDots | `src/server/parallel.ts`, `research.ts` | Búsqueda web con evidencia |
| E20 | Recibos de voz | OpenDots | `src/shared/voice-receipt.ts` | Prueba de lo dicho |
| E21 | Marcadores de tarea programada | OpenDots | `src/shared/scheduled-message.ts` | Replay de historial |
| E22 | Mascota y estados de ánimo | OpenDots | `src/client/Mascot.tsx` | Polish |
| E23 | Tema oscuro generado | OpenDots | `src/build/dark-theme.ts`, `src/client/theme.ts` | Diseño |
| E24 | Aviso de poll largo | OpenDots | `src/client/poll-notice.ts` | UX de espera |
| E25 | Allowlist de orígenes | OpenDots | `src/server/app-origin.ts` | Endurecer API local |
| E26 | Seguridad y transporte del navegador | OpenDots | `src/browser/security.ts`, `transport.ts` | Revisión de seguridad |
| E27 | Telemetría de setup | OpenDots | `src/server/setup-telemetry.ts`, `docs/TELEMETRY.md` | Producto |
| E28 | Tarjetas de tarea | OpenDots | `src/client/TaskPresentation.tsx`, `TaskActions.tsx` | UI de 9 |
| E29 | Codex app-server como backend | openbot | `src/backend/app-server-client.ts` | Alternativa a ACP |
| E30 | Registro e instalación de ACP | openbot | `src/main/acp-registry*.ts` | Comparar con `backends.rs` |
| E31 | Detección de modelos locales | openbot | `src/main/model-server-probe.ts`, `provider-detection.ts` | Ollama/LM Studio |
| E32 | Cuentas de proveedor y límites | openbot | `provider-account-store.ts`, `usage-limit-gate.ts` | Multi-cuenta |
| E33 | Precio por uso | openbot | `src/backend/agent/usage-pricing.ts` | Coste visible |
| E34 | Gate de duplicación | openbot | `src/backend/agent/duplication-gate.ts` | Evita duplicados |
| E35 | Historial de archivos por hilo | openbot | `src/backend/agent/thread-file-history.ts` | Timeline de cambios |
| E36 | Snapshots y replay de conversación | openbot | `conversation-snapshots.ts`, `thread-replay.ts` | Depuración |
| E37 | Salida estructurada | openbot | `src/backend/structured-output.ts` | JSON fiable |
| E38 | Plan del turno visible | openbot | `src/backend/agent/plan-updates.ts` | Transparencia |
| E39 | Generación de imágenes | openbot | `src/backend/agent/image-gen-runtime.ts` | Media |
| E40 | i18n y monitor de recursos | openbot | `packages/i18n`, `src/main/resource-monitor.ts`, `idle-restart.ts` | Operación y alcance |

## 9. Arquitectura de integración propuesta

### 9.1 Módulos nuevos del host

| Módulo | Responsabilidad | Fichas |
| --- | --- | --- |
| `host/src/docs/` | Documentos, revisiones, plantillas y render | 1-6 |
| `host/src/artifacts/` | Artefactos, versiones, blobs y sandbox | 7 |
| `host/src/tasks/` | Tareas, esperas y carriles | 9, 10, 15-17 |
| `host/src/approvals/` | Políticas, tarjetas, efectos y reglas | 31, 34-36 |
| `host/src/secrets/` | Secretos de bot, variables y ejecución | 32, 33 |
| `host/src/sandbox/` | `SandboxProvider`, Docker y leases | 19-24 |
| `host/src/browser/` | CDP, refs, perfiles y grabación | 22, 30 |
| `host/src/channels/` | Drivers de mensajería y bindings | 53-56 |
| `host/src/context/` | Presupuesto, categorías y compactación | 18, 48, 52 |
| `host/src/crypto/` | Claves, cifrado y búsqueda cifrada | 49, 50 |
| `host/src/transfer/` | Export/import de instancia | 38, 60 |
| `host/src/automation/` | Listener local y CLI | 46 |

No crear un módulo por ficha. Las fichas 8, 12, 13 y 59 viven en `agent/` y
`chat/`; 40-45 en `routines/`; 47 en `chat/memory/`; 57 en `voice.rs`.

### 9.2 Tablas nuevas (SQLite)

| Tabla | Campos mínimos | Fichas |
| --- | --- | --- |
| `documents` | `id, bot_id, space_id, parent_id, title, body, revision, created_at, updated_at` | 1-4 |
| `pending_actions` | `id, bot_id, lane, kind, effect_key, payload, status, expires_at` | 5, 6, 35 |
| `artifacts`, `artifact_versions` | `id, bot_id, name, mime, sha256, version, blob_path` | 7 |
| `bot_instruction_versions` | `id, bot_id, body, author, reason, created_at` | 8 |
| `agent_tasks` | `id, bot_id, lane, title, brief, state, progress, result` | 9 |
| `agent_waits` | `id, bot_id, note, routine_id, state, woken_at` | 10 |
| `queued_turns` | `id, bot_id, lane, kind, payload, state, attempts` | 16, 17 |
| `connector_tool_policies` | `connector_id, tool_name, mode` | 31 |
| `pending_tool_calls` | `id, bot_id, connector_id, tool_name, args_json, status, expires_at` | 31 |
| `approval_rules` | `id, bot_id, tool_pattern, risk, mode` | 34 |
| `effects` | `effect_key, result, created_at` | 6, 35, 36 |
| `bot_secrets`, `bot_env` | `id, bot_id, name, origin, auth, ciphertext` | 32, 33 |
| `computers` | `id, bot_id, mode, provider, state, generation` | 19-24 |
| `computer_activity` | `id, bot_id, action, actor, result, redacted_args` | 28 |
| `recordings` | `id, bot_id, goal, events_json, snapshots_json` | 29 |
| `channels`, `channel_threads` | `id, kind, bot_id, external_id, allowed_roles` | 53-56 |
| `channel_memories`, `channel_receipts` | `channel_id, body, external_id, state` | 55, 56 |
| `routine_runs` | `id, routine_id, status, summary, result_json, error` | 43, 44 |
| `routine_flows` | `routine_id, from_bot_id, to_bot_id, position` | 41 |
| `routine_watch` | `routine_id, kind, target, every_minutes, state_json` | 42 |
| `webhook_receipts` | `route_id, received_at, status, bytes` | 40 |
| `search_tokens` | `entry_id, token` | 50 |
| `ai_consent` | `user_id, scope, recipient, version, granted_at, revoked_at` | 51 |
| `calls` | `id, bot_id, nonce, state, started_at, ended_at` | 57 |

### 9.3 Contratos de API

Toda adición es aditiva y con detección de capacidad, según
`docs/reference/compatibility.md`:

- Métodos nuevos: `listDocuments`, `saveDocument`, `reviewDocument`,
  `resolveAction`, `listArtifacts`, `restoreArtifact`, `startTask`, `listTasks`,
  `cancelTask`, `askSideQuestion`, `listWaits`, `cancelWait`, `listRuns`,
  `exportInstance`, `importInstance`.
- Eventos nuevos: `documentChanged`, `artifactChanged`, `taskChanged`,
  `queueChanged`, `pendingAction`, `contextUpdated`, `channelChanged`.
- Capacidades: `documents-v1`, `tasks-v1`, `approvals-v1`, `sandbox-v1`,
  `channels-v1`, `crypto-v1`. Un cliente viejo ignora el evento y sigue.
- Regla: el host valida y ejecuta; el cliente nunca envía el payload de una
  aprobación, solo el id y la decisión.

### 9.4 Superficies por cliente

| Ficha | Escritorio | iPhone | Android | TUI |
| --- | --- | --- | --- | --- |
| Documentos (1-4) | Editor completo | Lista + Markdown | Lista + Markdown | `docs` |
| Revisión (5) | Tarjeta | Tarjeta | Tarjeta | `a`/`d` |
| Artefactos (7) | Galería + visor | Lista + visor | Lista + visor | `ls` |
| Tareas (9) | Panel lateral | Sección del bot | Sección del bot | `t` |
| Pregunta lateral (11) | Atajo | Mantener enviar | Mantener enviar | `/ask` |
| Computadora (26) | Panel completo | Estado/archivos | Estado/archivos | `c` |
| Canales (53-56) | Ajustes | Ajustes | Ajustes | `channels` |
| Automatización (46) | Ajustes + CLI | Solo estado | Solo estado | CLI |

## 10. Roadmap por olas

Cada ola termina con tests verdes, e2e de la plataforma y documentación
actualizada. No empezar una ola sin cerrar la anterior.

### Ola 0 — Confianza (6-8 semanas)

Fichas: 6, 27, 28, 31, 32, 33, 34, 35, 36, 37, 39.

Objetivo: que ninguna herramienta con efectos se ejecute sin política, que las
credenciales tengan un contrato y que reintentar no duplique nada. Es la base de
todo lo demás. Sin esta ola, las siguientes amplían la superficie de riesgo.

Salida: `pending_tool_calls`, `effects`, `bot_secrets`, `bot_env`, `egress`,
`computer_permissions`, `computer_activity` y tarjetas en los cuatro clientes.

### Ola 1 — Entregables (4-6 semanas)

Fichas: 1-5, 7, 8, 58.

Objetivo: que el trabajo quede en documentos y artefactos versionados, con
revisión humana. Es la ola con más impacto visible para el usuario.

Salida: biblioteca de documentos, editor, autosave, conversación por documento,
tarjeta de revisión, artefactos versionados y tarjeta de herramientas.

### Ola 2 — Trabajo en segundo plano (6-8 semanas)

Fichas: 9-11, 15-17, 43, 44, 45, 46.

Objetivo: tareas y esperas reales, cola persistente y API local para scripts.
Depende de la cola de la ola 0 para no duplicar.

Salida: pestaña de tareas, `await_event`, pregunta lateral, cola persistente,
runs estructurados, ejecuciones silenciosas, feed ICS y `codync-cli`.

### Ola 3 — Computadoras aisladas (8-12 semanas)

Fichas: 19-26, 29, 30.

Objetivo: un modo de computadora que no sea la máquina del usuario, con navegador
propio y grabación de flujos. Es la ola más grande; conviene separar "Docker
local" de "proveedores remotos".

Salida: `SandboxProvider`, Docker, computadora de equipo, perfiles, leases, panel,
takeover con snapshot y teach.

### Ola 4 — Canales y dato cifrado (8-12 semanas)

Fichas: 53-56, 49-51, 40-42.

Objetivo: llevar el bot a Slack/Telegram/Discord y proteger el dato en reposo.
El cifrado es irreversible una vez migrado: hacerlo con backup y plan de
recuperación.

Salida: drivers de canal, bindings, memoria de canal, recibos, cifrado por bot,
búsqueda cifrada, consentimiento, webhooks con filtros y watch.

### Ola 5 — Ecosistema (8-10 semanas)

Fichas: 12-14, 18, 38, 47, 48, 52, 57, 59, 60.

Objetivo: flotas, subagentes, contexto avanzado, voz recuperable y portabilidad.
Depende de que las olas anteriores estabilicen los contratos.

Salida: flota, subagentes, reassign, presupuesto de contexto, proveedor de
memoria, recuperación de llamadas, generación de bots, importación y
transferencia.

Cobertura: las seis olas suman las 60 fichas, sin repetir ninguna.

## 11. Riesgos y decisiones pendientes

| Riesgo | Impacto | Mitigación |
| --- | --- | --- |
| Licencia de openbot | Alto | Reimplementar; nunca copiar código; registro de atribución |
| Explosión de alcance | Alto | Olas cerradas; una ficha sin aceptación no se considera hecha |
| Cifrado mal migrado | Muy alto | Backup, claves versionadas, migración tolerante, rollback probado |
| Contenedores comparten kernel | Alto | Documentar el límite; no prometer aislamiento fuerte; evaluar gVisor |
| Slack app review | Medio | Empezar con una app interna y scopes mínimos |
| Coste de auto-review | Medio | Solo bajo riesgo y con presupuesto; default ask |
| Ruido de aprobaciones | Alto | Reglas por herramienta y "allow" explícito; mostrar el motivo |
| Docker en Windows | Medio | Detectar Docker Desktop; si falta, modo local con aviso |
| Paridad móvil | Alto | Ninguna ficha se cierra sin iOS/Android/escritorio/TUI o razón documentada |
| Pérdida de contexto al compactar | Alto | Conservar originales y probar recuperación (ficha 48) |
| Webhooks como superficie de ataque | Alto | Firma, límite de cuerpo, rate limit, filtros y recibos sin body |
| Secretos en logs | Muy alto | Redacción central y tests que fallan si aparece un valor |

Decisiones que requieren al usuario antes de construir:

1. ¿La computadora aislada es Docker local, un proveedor remoto, o ambos?
2. ¿Los documentos son por bot, por grupo o globales a la instancia?
3. ¿Las aprobaciones sustituyen al harness o conviven con él?
4. ¿El cifrado en reposo es obligatorio para todos o opt-in?
5. ¿Los canales son solo Slack en la primera entrega?
6. ¿La API local se documenta como contrato estable con versionado?
7. ¿La flota puede crear bots o solo organizarlos?
8. ¿Se permite importar desde otros productos en la primera versión?

## 12. Validación

Por ficha:

- **Unidad:** reglas puras (validación de DAG, filtros, hash de efecto, parser de
  comandos) sin I/O.
- **Integración:** store + host + API, con SQLite temporal y un agente falso.
- **Contrato:** un cliente viejo recibe solo lo que entiende; un evento nuevo no
  rompe el render.
- **E2E:** según `AGENTS.md`, mínimo macOS; completo si la ficha toca plataforma,
  packaging o el proceso principal del escritorio (macOS, Linux en Lima y Windows
  en UTM). iPhone en simulador con Device Hub.
- **Seguridad:** intento de aprobar con payload distinto, origen no permitido,
  ciphertext movido, nonce repetido, firma inválida.
- **Rendimiento:** contexto, búsqueda cifrada, cola con 100 tareas y arranque de
  contenedor.

No se han ejecutado compilaciones ni pruebas para escribir este documento. Antes
de cerrar una ficha, reportar qué checks no pudieron correr y por qué.

## 13. Anexos

### A. Glosario

- **Bot / agente:** identidad persistente con instrucciones, carpeta y sesión.
- **Lane:** par `(chat, thread)` donde vive un turno; se extiende a `doc:<id>`.
- **Harness:** CLI o agente ACP que ejecuta el modelo (Claude, Codex, etc.).
- **Efecto:** acción con consecuencias externas (guardar, enviar, pagar).
- **Clave de efecto:** identificador idempotente de un efecto.
- **Sandbox:** máquina aislada donde corre la computadora del bot.
- **Watch:** trigger que compara una huella y solo despierta si cambió.
- **HITL:** human-in-the-loop; la conversación espera una decisión humana.

### B. Mapa de fuentes por repositorio

OpenDots:

- Producto y documentos: `README.md`, `docs/SETUP.md`, `docs/COMPUTERS.md`,
  `docs/CONNECTIONS.md`.
- Servidor: `src/server/{page-service,pages,computer-service,computer-store,
  connections,connection-tools,slack-channel,voice,parallel,research}.ts`.
- Cliente: `src/client/{SpaceLibrary,SpaceWorkspace,PageDocument,PageReviewCard,
  ComputerPanel,ConnectionActionCard,CallView}.tsx`.

openbot:

- Arquitectura: `docs/ARCHITECTURE.md`, `docs/architecture/*.md`,
  `docs/automation.md`, `docs/webhooks.md`.
- Orquestación: `src/backend/agent-tasks/`, `agent-waits/`,
  `agent/{side-question,context-compaction,drain-scheduler,boot-recovery,
  tool-call-idempotency}.ts`.
- Canales: `src/backend/messaging/{slack,telegram,discord}/`,
  `channel-service.ts`.
- Webhooks y rutinas: `src/backend/{webhook-trigger,routine-flows,routine-results}/`,
  `src/main/{host-events-service,routine-feed-ics}.ts`.
- Navegador y computer use: `src/backend/browser-*.ts`,
  `src/main/cua-driver-runtime.ts`.
- Automatización y transferencia: `src/main/{automation-server,
  instance-transfer-service}.ts`.

rakazo:

- Producto: `VISION.md`, `README.md`, `docs/{fleet-management,bot-secrets,
  computer-runtime,content-encryption,tool-activity,agent-verification}.md`.
- Aprobaciones y secretos: `packages/core/src/{action-approval,
  approval-effect-key,egress}.ts`, `apps/api/src/agent-secrets.ts`.
- Computadora: `packages/core/src/screen-lease.ts`,
  `packages/adapters/src/{computer-tools,browser-tools,artifacts,
  routine-watch}.ts`, `infra/sandboxes/`.
- Voz y teaching: `packages/core/src/{teach-playbook,teach-recording,
  call-request,call-nonce,barge-in}.ts`.
- Canales: `apps/api/src/messaging-*.ts`, `packages/core/src/messaging-*.ts`.

### C. Qué no copiar

- El modo `danger-full-access` con `approvalPolicy: never` de openbot. Es una
  decisión de desarrollo, no un default de producto.
- La dependencia de un servicio de conversaciones hospedado de OpenDots. Codync
  ya es local-first; no introducir una dependencia obligatoria.
- El modelo de un solo owner de OpenDots si se quiere multi-tenant.
- La complejidad de 5.000 archivos de openbot. Codync debe quedarse modular y
  pequeño.
- El cifrado "todo o nada" sin plan de migración. Rakazo lo hace con tolerancia a
  filas antiguas; copiar ese detalle, no solo el algoritmo.

### D. Próximas cinco acciones

1. Decidir las ocho preguntas de la sección 11 y registrar la decisión en
   `docs/architecture/`.
2. Escribir el contrato de `pending_actions` + `effects` (fichas 5, 6, 35) y
   validarlo con un prototipo de una tarde.
3. Medir el contexto real de una conversación larga en Codync (ficha 18) antes de
   elegir umbrales.
4. Prototipar el `SandboxProvider` con Docker y un bot (fichas 19-20) para
   descubrir el coste real en macOS, Linux y Windows.
5. Abrir una rama de investigación para cifrado en reposo con un backup real y
   un test de recuperación (fichas 49-50), sin tocar `main` hasta tener plan de
   rollback.


