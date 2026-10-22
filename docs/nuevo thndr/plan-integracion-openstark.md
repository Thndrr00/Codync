# Plan de integración: Openstark → Codync

Fecha: 2026-10-21. Estado: plan; sin cambios de código.

Fuentes:

- Checkout local de `nightly-labs/openbot` (lo que aquí se llama **Openstark**) en
  `C:\Users\Edwin Villeda\Documents\Openstark`, commit
  `9491b02ab4a34da634f20556bf058aa55ed02f2e` (2026-10-21, rama `main`).
- Complementa, en la misma carpeta: [ideas-openbot-para-reutilizar.md](ideas-openbot-para-reutilizar.md)
  (deep dive: 61 ideas + patrones S1-S7, con los contratos leídos en el commit
  `b4cb1bce`) y [implementaciónes de otros repos](implementaci%C3%B3nes%20de%20otros%20repos.md)
  (catálogo general, "ficha N").
- Estado actual de Codync: leído directamente del repo (`host/src/`, `apps/`,
  `cloud/`, `relay/`, `docs/`), no del deep dive.

Cómo leerlo:

- §1: la decisión que rige todo (contratos sí, código no).
- §2: **línea base** de Codync de hoy, con evidencia de archivos reales.
- §3: qué se integra y qué no (mapa completo de las 61 ideas + S1-S7 a items).
- §4: fases: orden, dependencias, duración, release.
- §5: la implementación por fase. Cada item lleva: **Fuente** (Openstark),
  **Llegada** (Codync, archivos reales), **Pasos** (numerados, en el orden en que
  se ejecutan), **Clientes** (iOS/Android/escritorio/TUI), **Compat**, **Tests**,
  **Esfuerzo**. Los contratos exactos (topes, estados, firmas) están en el deep
  dive: aquí solo se remite (idea N / S-N / ficha N).
- §6: trabajo transversal (atribución, docs, versionado, e2e, commits nuevos).
- §7: hitos. §8: riesgos.

Esfuerzo: S ≤ 1 semana, M 1-3 semanas, L 3-6 semanas, XL > 6 semanas, relativo a
un ingeniero que ya conoce el repo. Los nombres nuevos de tipos/tablas/métodos
son **propuestas**; al escribir cada item se verifica contra el dispatcher y los
contratos vigentes.

## 1. Decisión: integrar los contratos, no el código

La licencia de Openstark (PolyForm Noncommercial 1.0.0) prohíbe copiar código a
un producto comercial. La estrategia, ya decidida en el catálogo:
**reimplementar limpio cada contrato**. Cada item de este plan es "reimplementar
la idea N con el contrato del deep dive", y la constancia queda en
`docs/reference/upstream-attribution.md` (X1): repo, commit, ruta de evidencia,
"idea tomada, código no copiado".

Regla de oro de todo el plan: **la lógica vive en el host Rust**
(`docs/architecture/overview.md`). Todo item se implementa primero como método de
API + evento SSE + estado en el store; los cuatro clientes lo renderizan. Ningún
item agrega lógica a un cliente que deba vivir en el host.

## 2. Línea base: Codync de hoy

Lo que ya existe (evidencia) y lo que eso significa para el plan.

| Área | Hoy (evidencia) | Consecuencia |
| --- | --- | --- |
| Motor de turnos | Un actor por bot (`host/src/agent/bot/mod.rs`): `Cmd` mailbox, `Queued` (`User`/`BotRequest`/`Routine`/`Compact`/`Group`) en `VecDeque` **en memoria** (`queue.rs`); un turno a la vez; lane-batching de mensajes del usuario en el mismo lane | La cola no sobrevive un reinicio y no explica *por qué* algo espera (0.2, 0.4) |
| Turnos | `start_turn(lane, entry_ids, text, hidden, done_tx)` (`turn.rs`); `compose` reinyecta `<bot-profile>` tras compactación | El flag `hidden` ya existe (rutinas): sirve para la pregunta lateral (2.4) |
| ACP | `acp.rs`: spawn/request/notify/kill, `FailureKind`; `updates.rs`: `agent_message_chunk`, `thought`, `tool_call(_update)`, `plan`, `usage_update`, `compaction_update`, `config_option_update`; `on_permission` → auto-approve o `NeedsInput` + push; flush de stream cada 300 ms | El punto de entrada para holds, gate de uso, subagentes y delta buffer ya está en `updates.rs` |
| Compaction | `compaction.rs`: `track_usage`, `COMPACTED_FROM` 20 k tokens, handoff `Full`/`Cold`/`Away`, `chat/handoff.rs` (`draft`/`pending`/`kept`/`sent`) | El handoff existe pero sin presupuesto fijo (2.7) |
| Memoria | Crate `codync-mem` (`host/crates/memory`): `memory_entries`, `memory_dreams`, `memory_files`; `own_memory.rs` (memoria propia del harness); MCP builtin `memory` (env `CODYNC_MEMORY_LANE`) | Falta dueño por contexto, esencial/buscable y staging por turno (2.5) |
| Equipo | `chat/team.rs`: `BotRequest` (Queued/Sent/Completed/Failed/Cancelled), MCP `ask_bot`/`message_bot`, ≤64 pendientes, `ASK_TIMEOUT` 600 s | Es el sustrato de tareas y flujos (2.1, 2.9) |
| Rutinas | `routines/`: `Routine`, `Run` (`delivery_id`, `report_pending`, `result`), `Status` (Pending/Starting/Running/Recovering/Succeeded/Failed/Interrupted/Cancelled), `Trigger` (Interval/Once/Cron/Webhook/Event), hooks (≤64 KiB, Bearer o `X-Hub-Signature-256`, `delivery_id`, `Refused::Busy` → 503), runner de tick de 1 s | Ya hay dedup de delivery y 503 ocupado: el upgrade es el contrato completo (4.1) |
| Uso | `usage.rs`: **solo visualización** (Claude `/usage`, `POST /ingest/statusline`, ACP `usage_update`, rollouts de Codex; poll 5 min; API `usage` + SSE `usage`) | No hay enforcement: el gate es 1.2 |
| Aprobaciones | `BotConfig.permission` `ask`\|`auto`; entry kind `permission`; `respondPermission`; auto-approve elige `allow_once`/`allow_always` | Falta registro de atención, redacción en el borde, política auto-approve (1.3) |
| API | `POST /api/{method}` dispatch plano (`api/mod.rs`); eventos SSE: `hello`, `bot`, `entry`, `usage`, `screen`, `cloud`, `accessRequests`, `resync` (`api/events.rs`); rutas `/health`, `/hooks/routines/{id}`, `/events`, `/term/{id}`, `/ingest/statusline`, `/channel` (WS teléfono), `/oauth/callback` | Todo es aditivo: nuevos métodos + campos en eventos existentes |
| Dispositivos | `api/devices.rs`: `Caller` Local/Device/Pairing; scopes `Control`/`Screen`; lista `LOCAL_ONLY` (incluye `routineCall`); close codes 4001/4003/4008/4400/4410 | El teléfono hoy no puede invocar rutinas: cambiarlo es parte de 4.3 |
| Store | SQLite v4 (`store/schema.rs`): `kv`, `bots`, `entries` (rev global + `seq`), `devices`, `push_tickets`, `activity_tickets`; WAL, `secure_delete=ON` | La cola persistida y los recibos entran como migración v5 (0.2, 0.3) |
| Push | `remote/push.rs`: `AlertKind` Done/NeedsInput/Failed; sealed por dispositivo (X25519); supresión si `bot.notify==false`/`hidden`/teléfono conectado; throttle 5 s por (bot,kind); relay `push-batch`; línea genérica para APNs | Falta política única, higiene 410 y quiet (5.6, 5.7, 2.3) |
| Compatibilidad | `compat.rs`: `MIN_APP` 2.18.0, `MIN_HOST` 2.12.0; gate contra App Store; **sin negociación de capabilities** | 6.1 añade capabilities sin romper nada |
| Servicio | Solo loopback, Bearer token (KV + archivo), puertos 19222/19223, `CODYNC_ENV`, service labels por OS | El automation server (4.3) se monta sobre esta superficie |
| Pantalla | `screen/` + helpers Codync Screen + WebRTC (señal por canal, media directa/TURN) | El daemon de computer use ya existe en forma: 3.6 lo endurece |
| Marketplace | `market/`: extensiones instaladas por computadora, `enableNew` | Falta revisiones inmutables y catálogo único (5.4, 5.5) |
| Clientes | Escritorio `host-client.ts` (call + SSE); iOS `HostTransport`/`HostClient`/`RemoteTransport`; Android `EncryptedChannel`/`HostConnector`; TUI `tui/net.rs` | Todo item de UI toca los cuatro (regla de AGENTS.md) |

## 3. Qué se integra

Mapa completo: cada idea del deep dive a su item de implementación. Lo que se
**rechaza** está en §8.

| Deep dive | Item | Deep dive | Item | Deep dive | Item |
| --- | --- | --- | --- | --- | --- |
| S1 contratos | 0.1 | S2 ownership | X2 | S3 eventos+recibos | 0.3 |
| S4 límites | 0.1 | S5 UI una preocupación | X2 | S6 state files versionados | 3.2, X2 |
| S7 políticas locales | 1.3, 4.3 | 1 carriles+holds | 0.4, 1.1 | 2 entregas | 0.2 |
| 3 idempotencia tools | 0.3 | 4 boot recovery | 0.5 | 5 usage gate | 1.2 |
| 6 tareas | 2.1 | 7 esperas | 2.2 | 8 quiet runs | 2.3 |
| 9 flujos | 2.9 | 10 plan/subagentes | 1.4 | 11 pregunta lateral | 2.4 |
| 12 attention registry | 1.3 | 13 bot oculto | 2.10 | 14 delta buffer | 0.6 |
| 15 memoria por dueño | 2.5 | 16 data compartida | 2.11 | 17 adjuntos | 2.12 |
| 18 contexto visible | 2.6 | 19 descriptor harness | 1.1 | 20 handoff presupuesto | 2.7 |
| 21 work steps | 2.7 | 22 history tools | 2.7 | 23 descubrimiento tools | 2.8 |
| 24 runtime manager | 6.3 | 25 catálogo modelos | 6.2 | 26 pricing | 6.2 |
| 27 estado de credenciales | 6.3 | 28 política de comunicación | 2.13 | 29 agentes ACP custom | 6.4 |
| 30 snapshot acotado | 3.1 | 31 propiedad de tabs | 3.2 | 32 fill_fields | 3.3 |
| 33 secrets seguros | 3.4 | 34 browser view | 3.5 | 35 daemon computer use | 3.6 |
| 36 respuesta visual | 3.7 | 37 sitios hosteados | 4.9 | 38 orquestador | 4.6 |
| 39 hilos de ejecución | 4.5 | 40 relay tickets | 4.4 | 41 enlace de plataformas | 4.5, 4.8 |
| 42 memoria mensajería | 4.7 | 43 status post | 4.7 | 44 dedup 3 capas | 0.3 |
| 45 contrato webhook | 4.1 | 46 ciclo de ruta | 4.1 | 47 status webhooks | 4.2 |
| 48 automation server | 4.3 | 49 CLI | 4.3 | 50 importación de agentes | 5.1 |
| 51 transferencia de instancia | 5.2 | 52 plantillas por link | 5.3 | 53 importar de otros sistemas | 5.1 |
| 54 skills con revisions | 5.4 | 55 catálogo de plugins | 5.5 | 56 push sellado | 5.6 |
| 57 política de notificaciones | 5.7 | 58 edición de cola | 5.8 | 59 ciclo de servidor hosteado | 6.5 |
| 60 versionado de protocolo | 6.1 | 61 billing | 6.6 | — | — |

Nada queda sin destino.

## 4. Fases

| Fase | Items | Depende de | Duración aprox. | Release |
| --- | --- | --- | --- | --- |
| 0 — Cimientos | 0.1-0.6 | — | 3-4 sem | minor (protocol additions) |
| 1 — Motor de confianza | 1.1-1.4 | 0 | 3-4 sem | minor |
| 2 — Trabajo en segundo plano | 2.1-2.13 | 0, 1 | 5-7 sem | minor (varios) |
| 3 — Navegador y computadora | 3.1-3.7 | 0, 1 | 6-8 sem | minor |
| 4 — Externos: mensajería, webhooks, automatización | 4.1-4.9 | 0 (4.4-4.8 usan relay) | 6-8 sem | minor + cambios en cloud/relay |
| 5 — Portabilidad y móvil | 5.1-5.8 | 0, 2 | 3-5 sem | minor |
| 6 — Protocolo y endurecimiento | 6.1-6.6 | 0 | continuo | patch/minor por item |

Las fases 0-2 no tocan nube. La fase 4 toca `cloud/` y `relay/`. Los items 6.5-6.6
solo si se hospedan servidores o llega billing. Cada fase es lanzable sola y se
verifica end to end en los apps reales (AGENTS.md): escritorio con Computer Use,
iPhone en Simulator, Android en device/emulator, TUI en terminal.

## 5. Implementación por fase

### Fase 0 — Cimientos

Objetivo: que todo lo entrante sea una **entrega persistida e idempotente**, que
el protocolo de frontera viva en un solo lugar, y que la UI diga *por qué* algo
espera. Sin esto, las fases 1-5 se construyen sobre memoria volátil.

#### 0.1 Crate de contratos + módulo de límites (S1, S4)

- **Fuente:** deep dive S1, S4 (`packages/contracts`, `INPUT_LIMITS`).
- **Llegada:** nuevo crate `host/crates/protocol` (junto a `host/crates/memory`).
- **Pasos:**
  1. Inventariar cada método de `api/*.rs` (`send`, `queued`, `listQueued`,
     `cancelQueued`, `respondPermission`, `routines`, `routineCall`, `hello`,
     `usage`, `registerDevice`, …) con sus parámetros y respuestas.
  2. Definir en el crate: tipos serde por método (req/resp), el enum de eventos
     SSE (`Hello`, `Bot`, `Entry`, `Usage`, `Screen`, `Cloud`, `AccessRequests`,
     `Resync`) con `type` como tag, y los tipos de entry (`EntryKind` y los
     campos conocidos de `data`: `status`, `routineRunId`, `clientNonce`,
     `botMessage`, `delegationId`, …) hoy dispersos en `store/model.rs` y en
     JSON free-form.
  3. Mover los tipos canónicos al crate; el host y `tui/` los importan; el
     renderer de escritorio y los kits de iOS/Android conservan sus tipos
     propios, pero se prueba la paridad (test que serialice el contrato y lo
     compare contra fixtures JSON versionados por cliente).
  4. `LIMITS`: todos los máximos en un solo lugar — texto de mensaje (hoy
     `MAX_MESSAGE_BYTES` 32 000 en `chat/team.rs`), cuerpo de hook (64 KiB en
     `routines/hooks.rs`), WS (1 MiB en `remote/channel.rs`), chunk de upload
     (384 KiB), rutinas (≤1000, triggers 1..=20, timeout 1..=86400 s),
     dispositivos, pairing (10 min, 5/3/10 por minuto). Todo validador del
     dispatcher lee de `LIMITS`.
  5. Tests: todo método del dispatcher tiene contrato; todo límite tiene test.
- **Clientes:** ninguno funcional (refactor interno); fixtures en los 4.
- **Compat:** sin cambio de wire. **Esfuerzo:** M.

#### 0.2 Entregas: la cola persistida (idea 2)

- **Fuente:** idea 2 (mailbox → deliveries, estados, idempotency key, respuesta
  legible por GET).
- **Llegada:** migración del store v4 → v5 en `store/schema.rs`; reescritura de
  `agent/bot/queue.rs` sobre el store.
- **Pasos:**
  1. Tabla `deliveries`: `(id PK, bot_id, source [user|team|routine|webhook|
     script|group], idempotency_key, state [queued|starting|running|completed|
     failed|interrupted|cancelled], entry_id, error (≤2000, redactado),
     enqueued_at, started_at, finished_at, UNIQUE(bot_id, idempotency_key))`.
  2. `idempotency_key` por origen: para `send`, el `clientNonce` existente
     (`find_by_nonce` sigue sirviendo para la dedup instantánea; el nonce se
     convierte en la clave); para team, el id de `BotRequest`; para rutinas, el
     `delivery_id` del `Run` (unificar: `Run.delivery_id` referencia
     `deliveries.id`); para webhooks, el `delivery_id` del hook.
  3. El actor del bot mantiene el espejo en memoria (orden, batching por lane)
     pero **la fuente de verdad es la tabla**: `next_in_queue` lee/actualiza
     estado; cada transición `queued→starting→running→terminal` es una
     escritura atómica del store (mantiene el invariant de `rev` + eventos en
     orden).
  4. Respuesta de `send`: hoy `"queued"`/`"sent"` — se conserva y se añade el
     `deliveryId`. Nuevos métodos: `delivery {id}` → estado + respuesta final
     (la `answer` = última entry `agent` del turno); `listQueued` (existente)
     pasa a devolver `[{id, source, state, hold, enqueuedAt, position}]`.
  5. Prioridad de drenado (sin carriles todavía, solo orden): persona primero
     (`user`), luego `team`, luego `routine`/`webhook`; desempate por
     `enqueued_at`, luego `id`.
- **Clientes:** escritorio/iOS/Android ya listan y cancelan la cola (`queued`,
  `listQueued`, `cancelQueued`): muestran ahora `hold` (item 0.4) y posición;
  TUI muestra "esperando (razón)" en lugar de solo "en cola".
- **Compat:** aditivo (nuevo campo + 1 método). **Tests:** unit (transiciones,
  idempotencia por cada `source`) + e2e (reinicio con cola llena → no se pierde
  nada, nada se duplica). **Esfuerzo:** M-L.

#### 0.3 Recibos de comando + dedup en 3 capas (S3, idea 3, idea 44)

- **Fuente:** S3 (command receipts), idea 3 (tool-call idempotency), idea 44
  (dedup de 3 capas).
- **Llegada:** tabla `command_receipts`; módulo `host/src/receipts.rs`; toques en
  `chat/team.rs` y `routines/`.
- **Pasos:**
  1. Tabla `command_receipts(command_id PK, accepted_at, result_json,
     first_sequence, last_sequence)`: cualquier comando con efecto externo que
     puede reintentarse (tool de mensajería, `run_routine`, `ask_bot`,
     `routineCall`) se registra con `command_id` estable. El reintento lee el
     recibo en vez de reejecutar.
  2. `command_id` de tool-call: `{threadId}:{turnId}:{callId}`; si excede el
     límite de identificador (0.1), reemplazar thread+call por SHA-256
     `<digest>:<turnId>:tool-call` (idea 3, contrato exacto en el deep dive).
  3. Las 3 capas para eventos entrantes: (1) set en memoria (LRU) — inmediato;
     (2) clave de idempotencia persistida (0.2) — sobrevive reinicios; (3) el
     evento guardado: si el host cae entre recibo y procesamiento, se
     redelivera desde el store. Aplicar primero al camino webhook (hoy ya tiene
     `delivery_id`), luego a team y a la mensajería de la fase 4.
  4. `ask_bot`/`message_bot` hoy van con `idempotentHint: false`: pasarlos a
     `true` con `command_id` por `(botId, turnId, callId)`.
- **Clientes:** ninguno (correctitud interna). **Compat:** aditivo (los recibos
  son internos; `command_id` viaja por `data` de entry cuando corresponde).
- **Tests:** unit (mismo `command_id` → una sola vez; crash entre recibo y
  efecto → redelivery). **Esfuerzo:** S-M.

#### 0.4 Holds nombrados + estado de cola en el wire (idea 1, parte mínima)

- **Fuente:** idea 1 (carriles + holds nombrados). Versión mínima: sin carriles
  físicos todavía, el concepto de **hold** y su exposición.
- **Llegada:** `agent/bot/mod.rs` (estado del actor), `api/events.rs` (campo en
  el evento `bot`), `hub.rs` (`Runtime`).
- **Pasos:**
  1. `enum Hold { AgentBusy, OneTurn, UsageLimit, TaskLimit, Hidden, ServerBusy,
     WaitingPerson }` (nombres de wire del deep dive: `agent-busy`, `one-turn`,
     `usage-limit`, `task-limit`, `hidden`, `server-busy`).
  2. `Runtime` (hub) gana `holds: Vec<(lane_key, Hold)>`; cada delivery en cola
     lleva el `hold` que la detiene (calculado en `next_in_queue` antes de
     arrancar).
  3. El evento SSE `bot` (que ya porta `status`/`activity`/`lane`) añade
     `queue: [{id, source, hold, position}]`. Los clientes viejos ignoran el
     campo: aditivo.
  4. Los holds que existen hoy de hecho (`AgentBusy`, `OneTurn`,
     `WaitingPerson` = `NeedsInput`) se declaran; los demás se conectan cuando
     llegan (1.2, 2.1, 2.10).
  5. Plegar aquí el detalle de UX #2712 del commit actual de Openstark: el chat
     se mantiene en el último mensaje mientras corre un turno (política de
     scroll en los 4 clientes).
- **Clientes (los 4):** fila de cola con motivo visible — "esperando: el bot
  está respondiendo otro hilo" / "waiting: usage limit (resets 14:32)".
  Escritorio + iPhone + Android: fila en la vista de cola del bot; TUI: la
  línea de estado del bot incluye el motivo.
- **Compat:** aditivo. **Tests:** unit (hold correcto por configuración de cola)
  + visual en los 4 clientes. **Esfuerzo:** S.

#### 0.5 Recuperación de arranque (idea 4)

- **Fuente:** idea 4 (boot recovery).
- **Llegada:** arranque del host (`main.rs` / `service/`) + `agent/bot/` +
  `routines/runner.rs`.
- **Pasos:**
  1. Al arrancar, con el store abierto: `deliveries` en `running` →
     `interrupted` (no `failed`); las de `queued`/`starting` se reencolan en el
     actor cuando el bot nace (el orden de 0.2 lo preserva).
  2. Rutinas: `Run` en `Running`/`Starting` → `Recovering` (el estado ya existe)
     y `enqueue` de nuevo si el payload sigue en la fila del run; el flag
     `report_pending` ya cubre "publicar cuando vuelva".
  3. Inflight de turnos: el KV `turn.inflight.{bot}` (hoy solo resume el
     harness) pasa a reconectar delivery ↔ turno.
  4. Ventana de ocupado: mientras el host termina de arrancar sus bots, el hook
     de webhook devuelve 503 `Refused::Busy` **sin escribir recibo** (hoy ya
     existe `Busy`: fijar la regla de "sin recibo" por comentario + test), para
     que el emisor reintente después del reinicio.
  5. Team: `BotRequest` en `Sent` cuyo destinatario reinició → reencolar (máx. 1
     reintento; luego `Failed` con la razón).
- **Clientes:** ninguno directo (los estados ya existen). **Compat:** sin cambio
  de wire. **Tests:** e2e: matar el host con cola + turno corriendo + rutina
  pendiente → arrancar → todo termina, nada duplicado, la UI muestra
  "interrupted" donde aplica. **Esfuerzo:** S (sobre 0.2).

#### 0.6 Delta buffer (idea 14)

- **Fuente:** idea 14 (coalescer deltas; persistir solo el mensaje streaming).
- **Llegada:** `agent/bot/updates.rs` (hoy: flush fijo de 300 ms) +
  `store/entries.rs`.
- **Pasos:**
  1. Acumular chunks por `(thread, turn, entry)` y flush al primero de: 100 ms,
     8 KiB de delta, fin de item, fin de turno.
  2. El flush escribe **solo la entry streaming** (`update_entry`, no
     reescribir la historia del hilo) — verificar que el camino actual ya hace
     esto y medir; el costo de un flush no crece con la historia.
  3. El evento `entry` de delta lleva la `rev` global (ya la lleva): el cliente
     ordena por ella; añadir `revision` por (turno, entry) para saltos de orden
     dentro del mismo item.
- **Clientes:** ninguno (misma forma de evento). **Tests:** carga: N bots
  streamando a la vez → costo de flush plano; eventos en orden en los 4
  clientes. **Esfuerzo:** S.

### Fase 1 — Motor de confianza

Objetivo: que el motor sepa por qué no avanza (holds completos), no dispare
cascadas cuando el plan se agota, no envíe secretos al teléfono, y muestre el
trabajo en curso (plan/subagentes).

#### 1.1 Turn slots + prioridad + descriptor de harness (idea 1, idea 19)

- **Fuente:** idea 1 (DrainScheduler/TurnSlots), idea 19 (descriptores de
  proveedor).
- **Llegada:** `agent/backends.rs` (`HARNESSES`), nuevo `agent/bot/lanes.rs`,
  `queue.rs`.
- **Pasos:**
  1. `HarnessDescriptor { concurrent_turns: bool, reports_usage: bool,
     workspace_mode: Confinement|Self, fast_mode: bool }` como **datos** en
     `HARNESSES`; `turn_slots`, 1.2 y 2.1 leen el campo, no ramifican por
     harness.
  2. Carriles: un carril = `(bot_id, lane_key)` — hoy `Lane {chat, thread}` +
     lo que agreguen 2.1 (tareas) y 4.5 (mensajería). Cada carril corre a lo
     sumo un turno; el bot tiene 1 slot salvo que su harness declare
     `concurrent_turns` (hoy ninguno: 1 slot total).
  3. `next_in_queue` se convierte en scheduler: toma la siguiente delivery por
     prioridad (user > team > routine/webhook > group), comprueba holds (0.4),
     arranca; los que esperan conservan su `hold`.
  4. "No slot waits on another slot" (regla de Openstark): el trabajo de un
     `team` termina el turno que lo envía; la respuesta llega como nueva
     delivery (hoy `BotRequest` ya funciona así: fijar por test).
- **Clientes:** mismos de 0.4 (ahora el motivo de espera es exacto).
- **Tests:** unit (scheduler: prioridades, holds, 1 slot) + e2e con 2 carriles.
- **Esfuerzo:** M.

#### 1.2 Usage-limit gate (idea 5)

- **Fuente:** idea 5 (UsageLimitGate: la cola entera se pausa).
- **Llegada:** `usage.rs` (el dato ya está), nuevo `usage_gate.rs`, `queue.rs`,
  `updates.rs`.
- **Pasos:**
  1. Normalizar la negación por límite de plan en el error del harness: cada
     fuente (Claude `rateLimit` en `usage_update`, el texto de error de
     `usage_update`/respuestas fallidas de Codex, el JSON de `/usage`) se mapea
     a `UsageLimit { provider, account, model, resets_at }`. Lo que no se pueda
     leer, no se mapea ("a missing report is not zero usage").
  2. El gate abre una entrada por clave `provider\0account\0model` (el modelo va
     en la clave); mientras existe, **ningún** turno de esa clave arranca: las
     deliveries quedan en cola con hold `usage-limit` (0.4) en vez de fallar
     una a una.
  3. Cierre: en el reset reportado + 30 s de gracia, tras una lectura de uso
     sin ventana agotada, o al completar un turno. Sin reset reportado: releer
     cada 15 min (una lectura no es un turno). El gate vive en memoria: tras
     reinicio, la primera negación lo reabre.
  4. Exposición: el evento SSE `usage` (existente) añade
     `limited: [{provider, account, model, resetsAt}]`; el método `usage`
     igual.
- **Clientes (los 4):** la pantalla de uso marca "limited, resets 14:32" y la
  fila de cola dice el *why*; cuando el gate se cierra, la cola drena sola (se
  ve sin intervención).
- **Tests:** unit (aprieta/suelta por cada fuente, clave con modelo, gracia de
  30 s) + e2e con Claude agotado: N rutinas → 1 aviso, 0 cascadas de error.
- **Esfuerzo:** M.

#### 1.3 Registro de atención: redacción, acciones privilegiadas, auto-approve (idea 12, S7, idea 27, #2704)

- **Fuente:** idea 12 (attention registry + auto-approve local + redacción en el
  borde), S7 (políticas locales por máquina), idea 27 (solo el estado cruza),
  y el commit #2704 (las preguntas de agente aceptan más de una respuesta y
  sobreviven a reload).
- **Llegada:** `agent/bot/updates.rs` (`on_permission`), `api/bots.rs`
  (`respondPermission`), `store/` (política por bot), `api/devices.rs` (borde).
- **Pasos:**
  1. La card de permiso ya es una entry `permission` con id estable (es el
     "requestId opaco"): fijar por test que una respuesta vieja no puede
     contestar una solicitud nueva (id + turno).
  2. **Redacción en el borde:** el camino dispositivo (canal WS) devuelve el
     detalle del permiso **redactado** cuando el scope no lo justifica; el
     pendiente rico solo viaja al loopback. Los secrets de pregunta (si 2.x
     agrega preguntas secretas) nunca cruzan el canal.
  3. **Acciones privilegiadas:** lista declarada en el crate de contratos
     (tool calls que mueven dinero, publican, borran, autentican) → la card
     lleva `privileged: true`; los clientes viejos la ignoran.
  4. **Auto-approve por política** (S7): hoy `permission` es `ask`|`auto` por
     bot; ampliar a `BotConfig.permissionRules: [{tool, action?, scope,
     decision}]` (p. ej. "allow `web_fetch` always", "ask para `shell` con
     `rm`"), guardada en el store por bot, editable desde los 4 clientes (la UI
     de bot settings existe: añadir la sección). El archivo de versión de
     Openstark se traduce aquí a filas en el store (el store ya es el "archivo
     versionado" de Codync).
  5. `on_permission` consulta la política antes de `ask`; la decisión queda en
     la entry (auditable).
  6. El registro modela **respuesta múltiple y pendiente persistente**
     (#2704): una solicitud puede aceptar más de una respuesta y sigue
     pendiente tras un reinicio (los recibos de 0.3 cubren la persistencia).
- **Clientes (los 4):** sección "approvals" en bot settings (reglas con
  herramienta + decisión); cards con marca de privilegiada; el teléfono sigue
  sin ver el detalle rico.
- **Tests:** unit (política: exacta > parcial; privilegiada; redacción por
  scope; respuesta múltiple) + e2e (el teléfono recibe una card de permiso,
  contesta, la acción pasa una sola vez). **Esfuerzo:** L (primera pasada —
  pasos 1-4 — M).

#### 1.4 Plan y subagentes visibles (idea 10)

- **Fuente:** idea 10 (`turn/plan/updated`, `turn/subagents/updated`, fold de
  Claude).
- **Llegada:** `agent/bot/updates.rs` (el update de `plan` ya se recibe),
  `store` (entry `plan` ya existe).
- **Pasos:**
  1. La lista completa de plan por evento: normalizar estados
     (`inProgress`/`in_progress`/`pending`) y reemplazar la lista por id de
     step.
  2. Subagentes: nuevo campo en la data de la entry `plan` (o en el evento
     `bot`): `subagents: [{id, label, steps: [{name, tool, status}]}]`; cada
     step se deriva de la tool del subagente (Read→read, Grep/Glob→search,
     Edit/Write→edit, Bash→command, Web*→web).
  3. Fold de Claude (si el harness no envía plan): `TodoWrite` reemplaza la
     lista; `TaskCreate`/`TaskUpdate` la mutan; el resultado de `TaskCreate`
     re-etiqueta el paso del tool-use id al id real de tarea.
  4. Un turno nuevo muestra solo las tareas abiertas.
- **Clientes (los 4):** la tarjeta de actividad existente (la UI ya pinta
  entries `plan`/`tool`) renderiza steps con estado y subagentes como
  secciones anidadas; TUI: bloque plegable en el chat.
- **Tests:** fixtures de ACP por harness + visual en los 4. **Esfuerzo:** S-M.

### Fase 2 — Trabajo en segundo plano

Objetivo: el bot trabaja en paralelo y sin ruido: tareas, esperas, runs
silenciosos, preguntas laterales, memoria por dueño y contexto visible.

#### 2.1 Tareas de agente (idea 6)

- **Fuente:** idea 6 (`start_task`, límites, ciclo de vida, error persistente).
- **Llegada:** store v6 (`agent_tasks`), nuevo `agent/bot/tasks.rs`, MCP builtin
  `tasks` (se añade a la lista de `session.rs`), API + campo en el evento `bot`.
- **Pasos:**
  1. Tabla `agent_tasks(id, bot_id, title, brief, state
     [queued|starting|running|completed|failed|stopped|closed], error (≤2000,
     redactado), created_at, started_at, finished_at, session_id)`. Una tarea es
     **otro carril del mismo bot** (1.1): lane `task.{bot}@task-{id}` con su
     propia sesión ACP (`session/new` en el workspace del bot; `session/fork` si
     el harness la ofrece).
  2. Límites (idea 6): `taskLimit` por bot (default 2, configurable en
     `BotConfig`), lista ≤50, el bot puede abrir ≤8 abiertas (el usuario, 50);
     un harness con `concurrent_turns: false` (todos hoy) → la tarea espera con
     hold `task-limit`/`agent-busy` (0.4).
  3. Herramientas MCP: `start_task {title, brief}` (**idempotente por
     (botId, turnId, callId)** — 0.3), `stop_task`, `close_task`, `list_tasks`.
     Los prompts y aprobaciones de una tarea se muestran en el chat principal;
     sus entries de conversación viven en su lane (fuera de la actividad del
     chat).
  4. API: `tasks {botId}`, `task {id}`, `startTask`, `stopTask` (el usuario
     también inicia). El evento `bot` añade `tasks: [{id, title, state}]`.
  5. Ciclo de vida: stop interrumpe el turno; close mueve a "Earlier tasks";
     reopen restaura; el error del turno se guarda en la columna `error` y se
     limpia en el siguiente turno; boot recovery re-registra las sesiones
     (0.5). Cambio de provider/harness con una tarea corriendo → error fijo
     `waitForTasksBeforeProviderChange`.
- **Clientes (los 4):** tarjetas de tarea en la vista del bot (título, estado,
  error, stop/close), sección "Earlier tasks"; TUI: lista con `t`.
- **Tests:** unit (límites, idempotencia, error persistente) + e2e (2 tareas en
  cola con 1 slot → holds visibles → dreno). **Esfuerzo:** L.

#### 2.2 Esperas de agente (idea 7)

- **Fuente:** idea 7 (`await_event`/`close_wait`, semántica de wake, prompt de
  wake).
- **Llegada:** store (`agent_waits`), MCP builtin `tasks` (decidir entre `tasks`
  y `memory` al implementar; propuesta: `tasks`), `routines/runner.rs`.
- **Pasos:**
  1. Tabla `agent_waits(id, bot_id, request_entry_id, request_text, note
     (≤500), routine_id?, created_at, woken_at?)`; ≤20 por bot.
  2. Herramientas: `await_event {note, routineId?}`, `close_wait`.
  3. Wake: un run de la rutina nombrada la despierta; un run sin rutina
     despierta las esperas sin rutina; un `waitId` en el payload de un script
     (4.3) despierta esa espera exacta. La entry del run de wake **repite la
     petición original y la nota** (el contexto pudo compactarse) y añade "To
     wait for the next run again, leave the wait open."
  4. El usuario cancela desde el chat (todos los clientes ven la espera como
     estado del bot: `Runtime.activity` = "waiting for …"; TUI igual).
- **Tests:** unit (semántica de wake por cada vía) + e2e (espera → rutina la
  despierta → el prompt trae la petición original). **Esfuerzo:** M.

#### 2.3 Ejecuciones silenciosas (idea 8)

- **Fuente:** idea 8 (`routine_no_update`, `quiet: true`, sin notificación).
- **Llegada:** `routines/` (MCP `routines`), `remote/push.rs`, evento `entry`.
- **Pasos:**
  1. `Run` gana `quiet: bool` (persistido con el run).
  2. Vía principal: tool `routine_no_update` en el MCP `routines`; vía
     alternativa: la instrucción pide responder solo `[[no-update]]` (el
     detector vive en el host, no en el prompt).
  3. Si todo el turno fue quiet: la respuesta queda como entry con
     `data.quiet = true` (no cuenta como unread, no cambia el preview del bot,
     no genera push — `notify` ya tiene los puntos de supresión: añadir la
     regla "quiet run" junto a `bot.notify==false`); el evento `entry` lleva
     `quiet: true`.
  4. El preview del bot se restaura al estado previo al run (en memoria; tras
     reinicio la entrada sigue visible, como en Openstark). Test runs nunca son
     quiet.
- **Clientes (los 4):** la línea "sin novedades" se dibuja atenuada, sin badge
  de unread y sin sonido; el historial la conserva.
- **Tests:** unit (detector, restauración de preview, sin push) + e2e con una
  rutina cada 1 min: 12 runs quietos → 0 notificaciones. **Esfuerzo:** M.

#### 2.4 Pregunta lateral (idea 11, #2728)

- **Fuente:** idea 11 (`askSideQuestion`, presupuesto, sin herramientas) y el
  commit #2728 (interrumpir al agente por voz).
- **Llegada:** `agent/bot/turn.rs` (`start_turn` con `hidden`), nuevo método de
  API, `chat/context.rs` (presupuesto del contexto).
- **Pasos:**
  1. Método `askSideQuestion {botId, threadId?, question}`.
  2. El host arma el prompt: perfil del bot (name/description), los últimos
     mensajes que cuentan (user/agent con texto, sin thought/tool/permission/
     quiet), presupuesto 60 k chars de chat + 60 k de preguntas laterales
     previas, y la pregunta (topes en 0.1).
  3. El turno corre `hidden` (el flag ya existe) y **sin MCP tools** (el
     `mcp_servers` de la sesión se desactiva para ese turno): "the answer can
     only come from this text". La respuesta es efímera (≤32 k): no se guarda
     como entry del chat; vuelve en la respuesta del método y, si el cliente lo
     pidió, como entry con `data.ephemeral = true` excluida del historial.
  4. #2728: en una llamada de voz (`docs/features/voice-call.md`), la voz del
     usuario interrumpe el turno en curso (el interruptor vive en el host:
     `Cmd::Stop` por fuente de voz), de modo que la conversación puede flotar.
- **Clientes (los 4):** botón "ask about this" en la barra de contexto de un
  mensaje → diálogo con la pregunta → respuesta en una card efímera (no en la
  conversación); TUI: `?` sobre un mensaje.
- **Tests:** unit (presupuesto: 500 k de chat entra acotado) + e2e (la respuesta
  no consta en el chat ni en los clientes remotos). **Esfuerzo:** S.

#### 2.5 Memoria por dueño + esencial/buscalble + staging (idea 15)

- **Fuente:** idea 15 (owner por contexto, essential vs searchable, staging por
  turno, `command_id`).
- **Llegada:** crate `codync-mem` (`host/crates/memory`), MCP `memory`,
  `chat/memory.rs`.
- **Pasos:**
  1. `memory_entries` gana `owner_kind` (bot|group|routine|connection-cuando
     llegue 4.x) y `essential: bool` (las esenciales siempre van al prompt; las
     buscables solo cuando el agente busca). Topes por dueño (idea 15: bot 64 /
     group 32 / connection 32, texto ≤500 — empezar con los de Openstark y
     ponerlos en 0.1).
  2. Staging por turno: las escrituras de memoria de un turno se acumulan en staging y
     se commitan una sola vez al terminar (tabla `memory_staging` o KV por
     `turn.inflight.{bot}` — decidir al implementar; el commit atómico con el
     turno mantiene la propiedad "la memoria no existe antes de que el turno
     termine").
  3. `remember`/`forget` con `command_id` (0.3): un tool reintentado no
     duplica la memoria.
  4. Prompt: `compose` (`turn.rs`) inyecta las esenciales del dueño del
     contexto (hoy la memoria se cita como "dónde se aprendió un hecho":
     conservar).
- **Clientes (los 4):** en la vista de memoria del bot (verificar si existe hoy
  en escritorio/iOS: añadir si no) las entradas marcan esencial/buscalble; el
  cambio es un toggle.
- **Compat:** bump de `codync-mem` 0.x en el mismo release (AGENTS.md).
- **Tests:** unit (staging: crash a mitad de turno → 0 escrituras;
  idempotencia) + e2e. **Esfuerzo:** M.

#### 2.6 Contexto visible (idea 18)

- **Fuente:** idea 18 (compaction ownership, `contextStates`).
- **Llegada:** `compaction.rs` (`track_usage` ya mide tokens), `usage.rs`,
  evento `usage`.
- **Pasos:**
  1. El descriptor (1.1) dice si el harness reporta uso; si lo hace, el evento
     `usage` añade por sesión personal activa `context: {used, limit,
     lastCompaction}`.
  2. Si no reporta: no inventar — el cliente muestra "new session" como la
     alternativa (el `Cmd::NewSession` ya existe).
  3. Se limpia en reset/cambio de provider (los paths ya lo hacen: fijar por
     test).
- **Clientes (los 4):** medidor de contexto en la cabecera del chat (el estado
  de compaction ya se ve por la entry de compaction: ahora con números).
- **Tests:** unit por harness. **Esfuerzo:** S-M.

#### 2.7 Handoff con presupuesto + work steps + history tools (ideas 20-22)

- **Fuente:** ideas 20 (handoff 32 k), 21 (work steps), 22 (history_search /
  history_read).
- **Llegada:** `chat/handoff.rs` (el handoff existe: `draft`/`pending`/`kept`/
  `sent`), nuevo `agent/bot/history.rs`, MCP builtin `memory` (o `history`).
- **Pasos:**
  1. El handoff deja de ser "resumen" y pasa a **presupuesto**: 32 000 chars
     totales = ~24 k conversación reciente + 4 k excerpts clave + 2 k work
     steps + 2 k instrucciones. Los mensajes grandes se cortan **cabeza y cola**.
     Función pura con tests (500 k de conversación entra, salen topes).
  2. Al retirar una sesión: volcar los 60 turnos recientes a
     `provider-work-steps/v2/<sha256(session)>` (modo 0600), redactado **antes**
     de recortar, lectura topada a 10 s. Alimenta los 2 k del handoff y el
     cursor `workStepsOffset` de history.
  3. Tools de solo lectura con topes estrictos (idea 22): `history_search
     {query, limit}` → 5 (máx 10) coincidencias, excerpts 400 chars, ≤8 000
     chars totales; `history_read {limit}` → 10 (máx 20) mensajes, ≤16 000
     chars, cursors `nextOffset`/`workStepsOffset`. La frontera: el marcador de
     reset de contexto más reciente (nada anterior es legible).
- **Clientes:** ninguno (tools del agente). **Tests:** unit (presupuesto,
  redacción antes que recorte, frontera de reset) + e2e (cambio de modelo a
  mitad de sesión → el nuevo turno sabe la pregunta abierta). **Esfuerzo:** M.

#### 2.8 Descubrimiento de tools bajo demanda (idea 23)

- **Fuente:** idea 23 (catálogo de nombres + `tool_search`/`tool_describe`/
  `tool_call`; −80 % del prompt de tools).
- **Llegada:** `agent/bot/session.rs` (los MCP builtin los lanza el host:
  `connectors`, `chat`, `team`, `memory`, `routines`, `computer`, `composio`) y
  el builder de prompt del host.
- **Pasos:**
  1. Medir hoy: cuántos chars de prompt de tools lleva un bot con 3-4 MCP
     conectados (el número de "antes").
  2. El catálogo inicial lista **solo nombres** (una línea por tool); 9
     directas (`remember`, `forget_memory`, `search_memories`, `list_bots`,
     `ask_bot`, `message_bot`, `list_routines`, `run_routine`, + la que llegue)
     + las 3 de descubrimiento: `tool_search {query}` (match local: exactas
     primero, luego por palabra; 5, máx 10), `tool_describe {names}` (schemas
     originales, ≤10, namespaced), `tool_call {name, arguments}` (el router
     solo resuelve lo habilitado para ese bot).
  3. Las tools de descubrimiento viven en un MCP builtin `tools` que el host
     filtra por bot.
  4. Medir de nuevo y registrar el antes/después en la doc de MCP
     (`docs/features/marketplace.md` o una nueva).
- **Clientes:** ninguno. **Tests:** unit (router: un tool deshabilitado no se
  resuelve) + medición. **Esfuerzo:** M.

#### 2.9 Flujos de rutina entre bots (idea 9) — opcional

- **Fuente:** idea 9 (`RoutineFlows`, sweep idempotente, fan-in).
- **Llegada:** `routines/` (nuevo `routines/flows.rs`), store (3 tablas: links,
  node positions, steps), `chat/team.rs` (el handoff es una delivery).
- **Pasos:**
  1. Un link conecta la salida de una rutina con uno o más bots (≤50 links por
     flujo); el canvas por bot (rutas + último run de cada una) es proyección
     de los links.
  2. El service no escucha al provider: en cada evento de turno/cola/rutina
     hace un **sweep** (uno a la vez; otro más si llegó evento durante el
     sweep; arranca con uno tras reinicio).
  3. Fan-in: un bot con varias entradas espera todas y recibe un solo mensaje;
     un bot cuyas entradas fallaron todas se **skipea** (una falla no deja el
     flujo esperando).
  4. Herramientas para que los bots lo editen: `list_routine_flows`,
     `connect_routine_agents`, `disconnect_routine_agents`; el texto que un bot
     dio a otro se guarda en `steps` (fuera del log que nunca se borra).
- **Clientes (los 4):** vista de flujos por bot (canvas simple: nodos +
  flechas + último estado); TUI: lista textual.
- **Tests:** unit (sweep: fan-in, skip por fallas, idempotencia tras evento
  duplicado) + e2e. **Esfuerzo:** L.

#### 2.10 Bot oculto (idea 13)

- **Fuente:** idea 13 (soft-hide: `hiddenAt`, negación en todos los caminos,
  skip-backlog al restaurar).
- **Llegada:** `store/model.rs` (`BotConfig.hidden` ya existe para push:
  ampliar), todos los caminos de entrada.
- **Pasos:**
  1. `BotConfig.hidden_at: Option<i64>` (la supresión de push ya lee
     `bot.hidden`: unificar en un solo campo con timestamp).
  2. Negar mensajes nuevos en **todos** los caminos con
     `error.recipientHidden`: `send`, team (`ask_bot`/`message_bot`), grupos,
     rutinas (no corren), hooks (`Refused::Paused`), automation (4.3).
  3. Su cola espera detrás del hold `hidden` (0.4); un turno que ya corre
     termina. No aparece en listings, roster, conteos ni "needs you".
  4. Al restaurar: marcar los runs perdidos **skipped antes de clearing the
     flag** (el timer no corre un backlog), luego re-arma rutinas y drena.
- **Clientes (los 4):** "hide" en el menú del bot (junto a archive/delete,
  donde estén) y la fila oculta vuelve al restaurar.
- **Tests:** unit (cada camino niega) + e2e (hide → rutina no corre → restore →
  0 backlog). **Esfuerzo:** S-M.

#### 2.11 Base de datos compartida de agentes (idea 16)

- **Fuente:** idea 16 (`agent-data.db` con owner metadata + SQLite
  authorizer + escaneo de SQL).
- **Llegada:** nuevo `agent/data/` en el host; store aparte (fuera del store
  principal y del migrator, como en Openstark): `<CODYNC_HOME>/shared/
  agent-data.db`.
- **Pasos:**
  1. Un solo SQLite donde todos los bots crean tablas; `codync_metadata`
     registra el dueño de cada tabla: el dueño es el único que puede
     DROP/ALTER; los demás leen/escriben filas. La regla se aplica con el
     **authorizer propio de SQLite** (rusqlite lo soporta) + escaneo de strings
     para mensajes útiles ("PRAGMA is not available. Use list_tables").
  2. Rechazos con texto: ATTACH/DETACH/VACUUM/PRAGMA/ANALYZE/REINDEX y toda
     transacción manual (cada call corre una statement en su propia
     transacción). Booleans a 1/0; parámetros con cotas (cantidad y bytes, en
     0.1).
  3. Tools: `query_data` (solo SELECT) y `execute_data` en un MCP builtin
     `data`. Las tablas sobreviven a la eliminación de su dueño; el usuario las
     borra desde settings (nuevo método + UI).
- **Clientes (los 4):** sección "shared data" en settings: tablas, dueños,
  tamaño, borrar.
- **Tests:** unit (authorizer: un bot ajeno no DROPa; escaneo: PRAGMA
  rechazado con mensaje) + e2e (dos bots comparten una tabla de tareas).
- **Esfuerzo:** M-L.

#### 2.12 Artefactos visibles solo tras commit (idea 17)

- **Fuente:** idea 17 (staging de adjuntos generados; outbox de borrado).
- **Llegada:** `agent/bot/files.rs` + `chat/files.rs` + `chat/uploads.rs`.
- **Pasos:**
  1. Los archivos que un bot genera se stagingean en
     `<workspace>/.staging/`; pasan a visibles **solo después** del commit del
     turno (entry + store en la misma transacción).
  2. Borrado de bot/archivos: los borrados de disco se registran en la misma
     transacción y un `file_deletion_outbox` reintenta los que fallaron
     (retry con backoff; el outbox vive en `kv`).
  3. El módulo de archivos no toca el store (copia, límites, hashes,
     manifest): mantener la propiedad actual (fijar por test).
- **Clientes:** ninguno directo. **Tests:** unit (crash entre staging y commit
  → el archivo no es visible) + e2e. **Esfuerzo:** M.

#### 2.13 Política de comunicación entre bots (idea 28)

- **Fuente:** idea 28 (developer instructions: formas fijas, delegar first,
  tabla de escenarios).
- **Llegada:** `chat/prompt.rs` / `chat/profile.rs` (el prompt del equipo).
- **Pasos:**
  1. Bloque estable inyectado en el prompt de bots que colaboran (texto fijo,
     no improviso): empezar a trabajar sin preámbulos; reportar solo progreso
     con sustancia; no rehusar antes de intentar; `list_bots` primero y
     **delegar al especialista** (la respuesta del peticionario empieza con el
     nombre del compañero).
  2. Forma fija del handoff: **Status / Result / Evidence / línea
     "Unblock:"**.
  3. Frase fija para plugin/feature ausente (el modelo no improvisa textos de
     error).
  4. La tabla de escenarios (escenario × comportamiento esperado) se guarda
     como test de regresión (test de prompt con respuestas fake).
- **Clientes:** ninguno. **Tests:** la tabla de escenarios. **Esfuerzo:** S.

### Fase 3 — Navegador y computadora

Objetivo: observación y acción del mundo fuera del terminal, con topes y
propiedad. Codync ya tiene remote screen (WebRTC) y el MCP `computer`; esta fase
añade la capa **semántica** (snapshot) y endurece el daemon.

#### 3.1 Snapshot semántico acotado (idea 30)

- **Fuente:** idea 30 (`browser-cdp-snapshot`, topes duros, refs por revisión).
- **Llegada:** nuevo `browser/` en el host (si se construye la capa CDP) o sobre
  el MCP `computer` si el harness trae su propio navegador: decidir primero **dónde
  vive el navegador** (recomendado: un browser CDP por bot en el host, como en
  Openstark).
- **Pasos:**
  1. Contrato de snapshot: `{elements: [{ref, role, name, value}], text
     (≤20 000 chars, viewport first), truncated, revision}`; topes duros: ≤200
     elementos (se escanean 2×), serializado ≤128 k, ≤10 000 nodos, valores ≤2 k
     (idea 30).
  2. Las refs mueren con la revisión: una acción que nombre una ref caducada
     falla limpio (error tipo `ref.stale`).
  3. **Cada acción devuelve un snapshot fresco** (el costo se paga en cada paso
     — es el diseño); un snapshot truncado lo dice.
- **Tests:** fixtures de página (viewport first, truncado, ref caducada).
- **Esfuerzo:** L.

#### 3.2 Propiedad de tabs + ciclo de vida (idea 31, S6)

- **Fuente:** idea 31 (ownership per tab, idle download, UUID ids) y S6
  (state files versionados).
- **Llegada:** `browser/tabs.rs`; estado del navegador en archivo versionado:
  `<CODYNC_HOME>/browser-state/v1.json`.
- **Pasos:**
  1. Cada tab: `ownerBotId`/`ownerLane`; las tools de un bot solo tocan sus
     tabs (check duro, no aviso).
  2. `close_tab` es el único camino de limpieza; borrar un bot barre sus tabs;
     ids de tab son UUIDs.
  3. Ociosidad: 30 min (5 min con RAM baja) descarga la página y conserva el
     tab.
  4. El estado (tabs, viewports) vive **fuera del store** en archivo
     versionado (S6): la versión en el archivo, siempre se reescribe en la
     versión actual, cada bound se revalida al leer, y un campo ilegible
     degrada campo a campo (perder el viewport de un tab no lo borra: "losing
     the user's open tab is worse than losing an emulated viewport").
- **Tests:** unit (propiedad, ociosidad, degradación campo a campo).
- **Esfuerzo:** M.

#### 3.3 `fill_fields` por lotes (idea 32)

- **Fuente:** idea 32 (1-16 campos, re-check por campo, never submits).
- **Llegada:** `browser/tools.rs`.
- **Pasos:**
  1. Una tool llena 1-16 campos nativos desde **un único snapshot fresco**
     (valores ≤16 KiB totales, deadline 30 s, un solo lease de CDP).
  2. Antes de cada campo re-verificar: handle vivo, huella coincide, valor
     original esperado, documento/origen sin cambio.
  3. Resultado: `confirmed: [índices]`, `failedIndex`, `uncertainIndex`. Nunca
     envía el form ni presiona Enter: los controles son acciones separadas.
- **Tests:** fixtures (campo que se mueve a mitad de lote → `uncertainIndex`).
- **Esfuerzo:** M.

#### 3.4 Entrega segura de secrets (idea 33)

- **Fuente:** idea 33 (el secret nunca pasa por tool arguments).
- **Llegada:** `browser/secrets.rs` + MCP `computer`/`browser`.
- **Pasos:**
  1. Canal de capacidad aparte: `enter_secret {fieldRef}` toma el valor por
     un parámetro que el host redacta en logs, prompt y snapshot (no viaja como texto del argumento del
     tool).
  2. Flujo: detener captura (si hay screen recording/live view: invalidarla),
     teclear el valor, **esperar 5 s y vaciar el campo**, escanear el
     mundo de automatización (buffers, previews, adjuntos) antes de conservar el
     documento generado: si el valor apareció en alguna parte, el documento se **reemplaza**.
  3. Fallback: la persona hace takeover (remote screen). **Sin reintento** (un
     reintento re-expone el secret).
- **Tests:** unit (redacción en logs/prompt/snapshot) + e2e con screen on.
- **Esfuerzo:** M.

#### 3.5 Browser view gateway (idea 34)

- **Fuente:** idea 34 (frames by bytes over websocket, input events,
  capabilities).
- **Llegada:** `remote/channel.rs` (el canal existe: añadir capabilities
  `browser-view`, `-cursor`, `-context-menu`, `-viewport`, `-clipboard`) +
  frames fuera de la cola de operaciones por tab.
- **Pasos:**
  1. Frames CDP van al cliente **como bytes** (no base64 en JSON) junto con
     eventos de entrada.
  2. El streaming va fuera de la cola de operaciones del tab (no se ahogan
     entre sí).
  3. Negociación por capabilities (6.1 las formaliza; empezar con el enum).
- **Clientes (los 4):** el teléfono/escritorio ve y dirige el navegador en vivo
  (el remote-screen UI ya existe: reutilizar el panel). **Esfuerzo:** L.

#### 3.6 Computer use: un solo daemon, proxies delgados (idea 35)

- **Fuente:** idea 35 (daemon + proxy por CLI, spawn directo, stable socket,
  digest allowlist).
- **Llegada:** `screen/` (Codync Screen ya es el daemon) + `agent/backends.rs`.
- **Pasos:**
  1. Fijar la división: Codync Screen corre como el daemon largo; cada harness
     recibe un proxy MCP fino que apunta a él (la identidad del proxy no
     importa).
  2. El binario del screen helper se **fija por versión y se permite por
     digest** (el mismo patrón que el instalador de escritorio); la ruta del socket
     es estable (menos de 104 bytes en POSIX; named pipe en Windows) porque la
     ruta es parte de la huella del proceso.
  3. El daemon se lanza directamente (no vía shell/open), para que el OS
     atribuya Screen Recording/Accessibility a **Codync** (el proceso
     responsable), no a un helper.
- **Tests:** e2e por OS (permisos atribuidos a Codync; crash del daemon → proxy
  error limpio). **Esfuerzo:** S-M.

#### 3.7 Respuesta visual: HTML en sandbox (idea 36)

- **Fuente:** idea 36 (`html_render`, origen inventado, CSP sandbox, preview
  PNG).
- **Llegada:** nuevo kind de adjunto `html` en `chat/files.rs` + renderers por
  cliente.
- **Pasos:**
  1. Tool `html_render {html, height}` guarda un adjunto `text/html`; the
     conversación recibe un item `visual-reply:<height>`.
  2. El host lo sirve desde un origen inventado `codync-visual:` con
     `CSP: sandbox allow-scripts allow-forms` y **origen opaco** (los scripts
     corren pero no alcanzan el storage ni la red de la app).
  3. Previews: registro topado a 8 páginas / 8 MB por conversación;
     `html_preview` renderiza el mismo HTML a **PNG + log de consola** (los clientes
     headless lo ven; los errores quedan visibles).
- **Clientes (los 4):** escritorio con webview inline (la altura en el item mantiene
  el layout estable); iPhone/Android con fallback a PNG; TUI: "abrir en navegador" +
  el PNG por nombre.
- **Tests:** unit (CSP, topes del registro) + visual. **Esfuerzo:** M-L.

### Fase 4 — Externos: mensajería, webhooks, automatización

Objetivo: el bot vive fuera de la app. El relay (Cloudflare) guarda los tokens
de plataforma; el host recibe tickets y operaciones typed.

#### 4.1 Contrato webhook completo + ciclo de vida de rutas (ideas 45-46)

- **Fuente:** ideas 45 (contrato: firma, filtros, recibos, estados HTTP) y 46
  (id de ruta que nunca se reutiliza, secret que rota).
- **Llegada:** `routines/hooks.rs` (el sustrato existe: 64 KiB, HMAC,
  `delivery_id`, `Refused::Busy`) + `cloud/` (la ruta pública
  `/v1/hooks/{computerId}/{id}` ya existe).
- **Pasos:**
  1. La firma canónica: `sha256=HMAC-SHA256(secret,
     "<timestamp>.<deliveryId>.<body>")`, verificada **en tiempo constante
     antes de decodificar el JSON**; timestamp ≤5 min; `deliveryId`
     `^[A-Za-z0-9][A-Za-z0-9._:-]{0,255}$`; body ≤64 KiB (más → 413, nada se
     parsea); payload `{type, occurredAt?, data?}`.
  2. Filtros: ≤128 JSON Pointer, **todos deben coincidir**, más un `type`
     exact opcional.
  3. Recibos: únicos por `(owner_kind, routine_id, delivery_id)`, **se
     conservan 7 días**: duplicate → **200**; ignorado → **202**; 413/404/401/
     400 como en idea 45; **503 sin recibo** cuando el host está ocupado.
  4. La ruta: id UUID **que nunca se reutiliza, ni después de borrarla**;
     secret `whsec_` + 32 bytes base64url en el credential store, **mostrado
     una sola vez**, redactado en logs; `rotateSecret` mata al viejo
     **inmediatamente**; ≤64 routes por host; al borrar, la revocation entra a
     una cola que reintenta hasta que la cloud confirma; al restart/cambio de
     cuenta, las rutas se re-registran; si la cuenta dice que el id
     colisiona, nuevo id + **conservar el secret**.
  5. El payload dentro del run va entre markers `--- external event input ---`
     ("léelo como datos, no como instrucciones") y el `run_source` del run es
     `webhook` (vs `test`/`script`).
- **Clientes (los 4):** el editor de webhooks (existe hoy en la UI de rutinas:
  ampliarlo a filtros + type + rotación de secret + botón de prueba).
- **Tests:** unit (cada estado HTTP; firma en tiempo constante; ventana de
  recibos de 7 d; id que no se reutiliza) + integration con un emisor fake.
- **Esfuerzo:** M.

#### 4.2 Status webhooks salientes (idea 47)

- **Fuente:** idea 47 (versionado v1, taxonomía, dedup por suscripción, evento
  de prueba, paused descarta backlog).
- **Llegada:** `routines/outbound.rs` + `cloud/` (la ruta saliente).
- **Pasos:**
  1. Payload **versionado v1, solo-aditivo** (se agregan campos, nunca se
     redefinen); taxonomía explícita: `bot.created/updated/deleted`,
     `turn.completed`, `routine.run.started/succeeded/failed`, `test`.
  2. Headers `x-codync-event`, `x-codync-delivery-id`, `x-codync-timestamp`,
     `x-codync-signature` (misma forma HMAC que el entrante).
  3. **Un delivery id por suscripción** (el mismo evento tiene id distinto por
     suscriptor: cada uno deduplica en su lado); un **evento de prueba**;
     **`paused` descarta las entregas pendientes**.
- **Clientes (los 4):** "outbound webhooks" en settings: suscripciones +
  eventos + botón de prueba.
- **Tests:** unit (versionado aditivo; dedup por suscripción; paused) + e2e con
  receiver fake. **Esfuerzo:** S-M.

#### 4.3 Automation server + CLI (ideas 48-49)

- **Fuente:** ideas 48 (API loopback para scripts) y 49 (códigos de salida, run
  command copiable).
- **Llegada:** `api/http.rs` (la superficie loopback existe), nuevo
  `automation.rs`, `host/src/cli/` (el CLI existe: `list`/`run`/`send`/events —
  ampliarlo).
- **Pasos:**
  1. `BotConfig.allow_automation: bool` (por bot, **solo el usuario lo
     cambia**: el parser general niega que un agente se lo autoasigne; duplicar
     un bot no copia el flag).
  2. Mientras algún bot tenga `allow_automation`, el host escribe
     `<CODYNC_HOME>/automation/` con `url`, `token`, `headers` (carpeta 0700,
     archivos 0600) y los borra al parar; **token nuevo + puerto nuevo en cada
     arranque** (un script muerto no puede retener un token vivo).
  3. Guardia: cualquier request con header `Origin` o un `Host` ajeno → **403 antes
     de checar el token** (una página web no puede alcanzarlo).
  4. Topes de run: **32 KiB de cuerpo, payload de 4 000 chars, 30 runs por bot
     por hora**; 202 = encolado; un `waitId` en el payload despierta esa
     espera exacta (2.2). Questions secret y takeovers por HTTP son
     **notice-only** (dirigen a la persona a la app).
  5. El agente recibe la **ruta del archivo, nunca el token** (el script y el agente
     comparten la API sin que el agente memorice una credencial).
  6. CLI (`codync`): `list`, `run --agent <id|name> --message …`, `status`,
     `--wait` (timeout por defecto 30 min; **detiene la espera, no el trabajo** —
     el run sigue; el client se cansó de esperar), `--id` (reintento: mismo
     id = mismo run), `--json` (emite un solo objeto incluso en fallo). Códigos de
     salida: **0 terminado / 1 falló-rechazado-desconocido / 2 nombre inválido-
     ambiguo / 3 esperando a la persona / 4 timeout de espera / 5 host no
     running**. Lee el archivo de credenciales en cada run (un reinicio del host no
     rompe un script a mitad de sesión).
  7. Cada routine gana un **"Copy run command"** (curl/PowerShell) en los 4
     clientes.
  8. Quitar `routineCall` de `LOCAL_ONLY` (el teléfono/script puede invocar
     rutinas por la vía de dispositivo correcta, con el mismo presupuesto).
- **Clientes (los 4):** el toggle "allow scripts" en bot settings + "copy run
  command" en la fila de la routine.
- **Tests:** unit (guardia Origin/Host; presupuestos; rotación de token) + e2e (un
  script de shell mueve un bot a través de un reinicio del host). **Esfuerzo:**
  M.

#### 4.4 Relay con route tickets (idea 40)

- **Fuente:** idea 40 (el token vive en el relay; el host recibe un JWT de 5
  min; presupuesto 16 req / 128 KiB; 503; nunca guardar el body).
- **Llegada:** `cloud/` (nuevas rutas del worker) + host (verificador de
  tickets).
- **Pasos:**
  1. La plataforma habla con **una URL pública** del relay worker; el worker
     guarda el token de la plataforma.
  2. El host recibe un **route ticket**: JWT ES256, TTL 5 min, nombra solo los
     workspaces ligados. Las revocaciones viven en memoria con una **ventana de
     verificación de 5 min** (un ticket viejo sigue sirviendo un momento durante
     la rotación).
  3. El host lleva un **presupuesto de pendiente: 16 requests / 128 KiB**;
     pasarlo o tardar >2.5 s devuelve **503** (la plataforma reintenta; el host
     está sobrecargado, no roto).
  4. **El cuerpo del mensaje nunca se guarda ni se loguea** en el relay —
     es un tubo.
- **Tests:** cloud (expiración del ticket, presupuesto 503, el cuerpo nunca persiste) +
  host (verificación de ticket, ventana de revocación). **Esfuerzo:** M.

#### 4.5 Enlace de plataformas: Telegram primero (ideas 41, 39)

- **Fuente:** idea 41 (flujo de link: código one-time, sin privileged intents) e
  idea 39 (hilo de ejecución por conversación externa, prompt reconstruido por
  turno).
- **Llegada:** `cloud/` (la Telegram Bot API en el worker), nuevo
  `host/src/messaging/`, lanes nuevos.
- **Pasos:**
  1. Telegram: **un solo bot para todos los usuarios**; el token vive solo en el
     relay. Ligar un workspace: un **código one-time** entregado como
     `t.me/<bot>?startgroup=<code>` (el deep link propio de la plataforma); el
     bot publica el link solo cuando el workspace está `linked: true`.
  2. `TelegramChatState` en memoria (la Bot API no da historial en que
     confiar: el host reconstruye contexto de lo que ve).
  3. Cada conversación externa (hilo de Telegram) tiene su **row de link** y su
     propio **execution thread**: lane `msg.telegram@<workspace>/<conversation>`;
     el trabajo ocurre ahí, oculto de la vista de chat pública (los approvals sí
     llegan al host/app).
  4. El prompt del agente se **reconstruye en cada turno** desde: el framing
     del thread (dónde es, quién habla), los mensajes externos recientes,
     los hechos de la conexión — el context siempre es fresco; el historial externo
     nunca se vuelca raw al context.
  5. Las salidas del bot a la plataforma: operaciones typed al relay worker
     (post/edit/react) con el ticket bearer; la **response chain se ancla al
     first message** de la conversación.
- **Clientes (los 4):** "messaging connections" en settings: link (deep link),
  status, unlink.
- **Tests:** cloud (link code one-time, unlink revokes) + e2e (un mensaje en
  Telegram → el bot responde en el thread, oculto del chat de la app salvo
  approvals). **Esfuerzo:** M-L.

#### 4.6 Orquestador por workspace (idea 38)

- **Fuente:** idea 38 (una app por plataforma + orquestador por
  workspace, `messagingReturn`).
- **Llegada:** `messaging/orchestrator.rs`.
- **Pasos:**
  1. Por workspace, el host crea un **orquestador**: un bot normal cuyo
     encargo vive en su descripción, con **5 fact memories** del workspace
     (2.5).
  2. Las conversaciones nuevas llegan al orquestador; él **delega al
     especialista** con `ask_bot` + `messagingReturn` (la respuesta vuelve a
     la misma conversación) y entretanto publica "A teammate is working on
     it.".
  3. Desconectar el workspace **borra los orquestadores que no se usen** (son
     estado derivado, no datos de usuario).
- **Tests:** e2e (un workspace con 3 bots: el orquestador delega, la respuesta
  vuelve al thread). **Esfuerzo:** M (sobre 4.5).

#### 4.7 Memoria de mensajería + status post (ideas 42, 43)

- **Fuente:** ideas 42 (hechos por conexión, tools de hilo) y 43 (un solo status
  post, reemplazado in-place, Stop con ownership).
- **Llegada:** `messaging/memory.rs`, `messaging/post.rs`.
- **Pasos:**
  1. Cada **conexión** (no conversación) tiene **32 fact memories** (≤500
     chars); **no hay hechos de DMs** (los DMs son context efímero).
  2. Tools: `messaging_remember` / `forget_memory` / `history` son **de
     hilo** — llamar a `history` desde otro thread se niega (un bot de Slack
     no debe leer Discord). Las salientes `messaging_list_places` /
     `messaging_post` funcionan **desde cualquier thread** (el agente puede publicar en
     un canal en el que no está), después de **redacción** (los secrets no salen
     del host).
  3. Cuando un bot arranca trabajo largo en un canal externo, publica **un
     status post** con un **botón Stop**; cuando llega la respuesta, **el
     post se reemplaza por la respuesta** (no hay "done ✓" + mensaje nuevo).
     Los posts se **serializan por conversación** (uno a la vez). Los botones approval/
     stop llevan un **token aleatorio en memoria** (un botón rasurado no
     puede re-emit) y **solo el usuario que arrancó el trabajo** puede
     responderlo o detenerlo (no el canal entero).
- **Tests:** unit (scope por thread; propiedad del Stop) + e2e. **Esfuerzo:**
  S-M.

#### 4.8 Discord + Slack (idea 41)

- **Fuente:** idea 41 (gateway de Discord en el relay sin privileged intents;
  events de Slack).
- **Llegada:** `cloud/` (el gateway socket per connection) + `messaging/`.
- **Pasos:**
  1. Discord: el relay mantiene el **socket de Gateway** (sin privileged
     intents — la app no necesita leer todo el history); el host llama
     **operaciones typed** (`post`, `edit`, `react`, …) con un bearer por
     socket; la **response chain se ancla al first message** (los threads de
     Discord son por mensaje).
  2. Slack: events al worker → tickets (4.4) → host; la cadena de respuesta por
     `thread_ts`.
  3. El orquestador (4.6) y la memoria (4.7) son los mismos por plataforma.
- **Tests:** como 4.5 por plataforma. **Esfuerzo:** M por plataforma.

#### 4.9 Sitios hosteados (idea 37)

- **Fuente:** idea 37 (`publish_site` / `replace_site` / `delete_site`, R2 +
  worker site-router, og:image, deep link que no instala).
- **Llegada:** `cloud/` (R2 + worker site-router: el worker no guarda contenido,
  enruta) + tool en el host.
- **Pasos:**
  1. Tools `publish_site` / `replace_site` / `delete_site` (cada una con su
     **approval card** — publicar es un acto visible; 1.3).
  2. Los archivos del agente suben a un subdominio público (`codync.site`); cada
     site recibe un **`og:image` 1200×630** (el link compartido se ve bien).
  3. El deep link `codync://sites/<id>` **no instala nada**: abre la
     página pública en la app, y la app decide.
- **Clientes (los 4):** la card del sitio en la conversación (thumbnail + abrir +
  reemplazar/borrar). **Esfuerzo:** M (con la nube existente).

### Fase 5 — Portabilidad y móvil

#### 5.1 Importación de agentes: zip + token one-time (ideas 50, 53)

- **Fuente:** ideas 50 (formato de export, rollback por agente, path guards) y 53
  (triage por ítem: ready/approve/skipped con motivo).
- **Llegada:** `host/src/import.rs` (nuevo) + `store/`.
- **Pasos:**
  1. Export: `codync-import.json` + `agents/<key>/` (instructions, skills,
     routines, avatar, membresías de grupo mapeadas). La descarga lleva un
     **one-time token + SHA-256** (el import verifica la integridad del
     archive y el token no se puede reusar).
  2. Guardia de extracción: no `../`, no absolutos, **letras de unidad de Windows
     rechazadas**.
  3. Import con **rollback por agente**: si el agente 3 de 5 falla, 1-2 quedan,
     el 3 se borra (incluidas revisiones publicadas de skill), 4-5 continúan.
  4. El triage por ítem: cada ítem termina en `ready | approve | skipped`
     con su motivo (`unsafeFile`, `invalidSkill`, `duplicateSkill`,
     `builtIn`); los valores de `.env`/`auth.json` **nunca se preview** (serían
     secrets en la UI); los clientes **nunca reciben paths** (el host guarda
     los perfiles bajo un token y responde por ítem — la UI no puede ser
     engañada para leer un archivo).
- **Clientes (los 4):** el diálogo de import (archivo → lista de triage → confirmar); TUI:
  import guiado por teclado.
- **Tests:** unit (rollback, guards, triage) + e2e. **Esfuerzo:** M.

#### 5.2 Transferencia de instancia (idea 51)

- **Fuente:** idea 51 (manifest versionado, no secrets, `cleared` array, lista de
  sign-in al final).
- **Llegada:** `host/src/transfer.rs` + `store/`.
- **Pasos:**
  1. Manifest `codync-instance-transfer` **versión 1**: agregar campos
     opcionales es gratis; cambiar el significado sube la versión; **una versión más
     alta se rechaza** (no "best effort").
  2. Los secrets no viajan: los valores/headers de MCP se **vacían**, las claves de custom-provider
     se descartan, el sign-in de provider viaja como **ids `signedIn`** solo;
     los args/URLs con **nombres que huelen a secret** (user/password/token en
     query/fragment) se **limpian** y se listan en un array `cleared` (el
     importador **vuelve a preguntar por cada uno**); las direcciones MCP conservan solo el origen.
  3. **Excluido por diseño**: historial de chat, conexiones, approvals, datos compartidos de agentes
     (la transferencia es un setup, no un clon).
  4. El zip se escribe como `.zip.part` y se renombra **solo al completo**;
     topes <2 GB y 50 000 archivos. Los MCP importados se añaden **APAGADOS**; un model
     ausente → **default + aviso**; el import termina con una **lista de botones de
     sign-in** (lo que el usuario debe hacer después).
- **Tests:** unit (rechazo de versión, cleared array, rename solo al completo).
- **Esfuerzo:** M-L.

#### 5.3 Plantillas por link (idea 52)

- **Fuente:** idea 52 (template = link público a una spec; secret scan; ≤5
  publicados verificados dentro de la sentencia de escritura).
- **Llegada:** `cloud/` (D1: una fila por (account, bot local)) + host (export
  de la spec).
- **Pasos:**
  1. Un template copia name/title/instructions/avatar/routines/skill
     references — **no** archivos de workspace, no memorias, no estado.
  2. Publicar corre a **secret-likeness scan** (un template que huela a
     secret no se publica).
  3. Re-publicar actualiza el mismo link; despublicar **conserva la fila**
     (el link sigue válido, marcado despublicado). Una cuenta puede tener ≤5
     publicados, verificados **dentro de la sentencia de escritura** (seguro bajo
     concurrencia).
  4. La página del link es una **card 1200×630 como og:image**; `codync://bots/
     <id>` abre la card (no instala; la app muestra el diálogo de install).
- **Tests:** cloud (concurrencia del top-5, secret scan) + e2e. **Esfuerzo:** M.

#### 5.4 Skills con revisiones inmutables (idea 54)

- **Fuente:** idea 54 (revisions inmutables, staging + rename, expected-revision
  check, install serializado por target).
- **Llegada:** `market/` (el marketplace existe: extenderlo).
- **Pasos:**
  1. Las skills instaladas son **revisions inmutables**:
     `local-skills/<uuid>/<revision>/bundle.zip`.
  2. Una versión nueva se escribe en una **staging path y se renombra solo al
     completo** (un crash no deja un bundle a medias).
  3. El update exige un **expected-revision check** (no ABA); los installs
     **per bot se serializan** (dos bots no entrelazan un skill swap); un pack es
     un bundle con varias skills y una sola lock entry (fijada por hash).
- **Clientes (los 4):** la card de skill muestra la revision + chip de update.
- **Tests:** unit (rename solo al completo; ABA; serialización) + e2e.
- **Esfuerzo:** S-M.

#### 5.5 Catálogo de plugins: una sola fuente (idea 55)

- **Fuente:** idea 55 (plugin = app de server MCP + skills fijas + listing page;
  un solo catálogo estático; el link abre, install es un verbo).
- **Llegada:** `market/` + `web/` (el sitio).
- **Pasos:**
  1. Un plugin = **app de server MCP + skills fijadas + página de listing**,
     distribuido como un **static set of catalog files** (un solo catálogo es
     la source of truth para el sitio y el marketplace de la app — no dos stores
     que se desfasen).
  2. La app cachea el catálogo en user data; un share link `codync://plugins/
     <slug>` abre la listing, **nunca instala** (el acto explícito del
     usuario instala).
- **Tests:** unit (paridad catálogo site/app). **Esfuerzo:** S.

#### 5.6 Higiene de push: 410 + canales por sonido (idea 56)

- **Fuente:** idea 56 (el teléfono crea un secret per-host; 410 borra la
  registry; canales per-sound en Android).
- **Llegada:** `remote/push.rs` + `relay/` + app Android.
- **Pasos:**
  1. El teléfono crea un **per-host secret**; los push payloads se sellan con una
     key derivada de HMAC(el secret del teléfono, server id) — un push que pasa por el
     relay es indecipherable para cualquiera más. (Codync ya sella per device
     con X25519: el delta es la **key derivada del teléfono** + el rechazo para otro
     server.)
  2. iOS: la Notification Service Extension abre el sello con la key de App Group
     y **rechaza contenido sellado para otro server**.
  3. El relay worker **no guarda ni loguea nada**; una respuesta **410 borra la
     registry** (token rotado/desconocido → dejar de push). Android: lo mismo
     vía FCM, donde `UNREGISTERED`/`SENDER_ID_MISMATCH` también mapean a 410.
  4. Android: **canales de notificación por sonido** (cada agente/sonido tiene su
     canal Android para poder silenciarlo por separado).
- **Tests:** relay (flujos de 410) + e2e iOS (sello de otro server rechazado) +
  Android (canal por bot). **Esfuerzo:** M.

#### 5.7 Una sola política de notificaciones (idea 57)

- **Fuente:** idea 57 (`notificationForAgentEvent`: server level × agent toggle
  × quiet; supresión mientras el desktop está activo; prioridades de Live Activity).
- **Llegada:** `host/src/policy.rs` (función pura, en el crate de protocol para
  los clientes) + `remote/push.rs` + clientes.
- **Pasos:**
  1. Una sola función decide: **server level** (`all` | `needs-me`) ×
     **toggle per agent** × **quiet runs** (2.3).
  2. Supresión mientras **el desktop del miembro está activo** (input en los últimos 30
     s, o focus de ventana, o otra sesión del mismo usuario activa — el teléfono no pica
     por lo que tu Mac ya está mostrando): el host ya trackea presence
     (`registerActivity` / `activity_tickets`): conectar los dos.
  3. Live Activity: **cambio de estado = prioridad 10**, update dentro del estado
     = **prioridad 5 con espera de 5 s**, un estado sin cambios se **reenvía
     cada 10 min** (para que la stale mark avance), y **idle termina la
     activity** (topes de iOS a activity en 8 h).
- **Clientes (los 4):** los notification settings (level + por bot); TUI: no
  push, pero la status line respeta la misma policy cuando el terminal está
  activo.
- **Tests:** unit (la pure function: cada combinación) + e2e (desktop activo →
  el teléfono no pica). **Esfuerzo:** S-M.

#### 5.8 Edición de cola con hold (idea 58)

- **Fuente:** idea 58 (queue-edit-v1: editar el mensaje antes de que el bot lo lea;
  save transaccional; el borrador sobrevive).
- **Llegada:** `agent/bot/queue.rs` (sobre 0.2) + API + capability
  `queue-edit-v1`.
- **Pasos:**
  1. Un usuario puede **editar una queued delivery** antes de que el bot lo procese.
  2. La delivery en hold conserva una **edit mark en el snapshot público** (los
     otros clientes ven "editando…", no un message fantasma); la identidad del
     edit es private.
  3. **Guardar** commitea el replacement **y** libera el hold **en una sola
     transacción del store** (no hay estado donde el text es new y el hold es
     old); **borrar** cancela la delivery y **libera la ownership de los
     attachments**.
  4. El **borrador del edit sobrevive un restart** (un "cancel" perdido no
     destruye el backup del usuario). Una edición completada registra
     **quién/qué la completó**.
- **Clientes (los 4):** la fila de la queue gana edit (el teléfono primero: es donde la
  cola es más larga); TUI: `e` en una queued row.
- **Tests:** unit (transaccional; el borrador sobrevive un restart) + e2e.
- **Esfuerzo:** M.

### Fase 6 — Protocolo y endurecimiento (6.1-6.6)

#### 6.1 Capacidades + codecs congelados (S2)

- **Fuente:** "capabilities" en `host/src/protocol/capabilities.rs`; `wire.rs:791` `const SUPPORTED_PROTOCOL_VERSION: i64 = 1;`; los codecs congelados: `agent_message_codec.rs` (schema JSON), `session_events_codec.rs` (NDJSON), `agent_session_codec.rs` (Zod), `mcp_client.rs` (JSON-RPC 2.0).
- **Codync hoy:** `host/src/protocol/mod.rs` (`ProtocolVersion = 1`) + `host/src/protocol/agent_message.rs`; sin negociación de capacidades; `host/src/api/handshake.rs` envía `hello` con `minApp`/`minHost` (compat.rs:31-32) y nada más.
- **Llegada:** un crate `host/crates/protocol` nuevo que contenga: (a) el set de **capabilities**: el `hello` del host lleva `capabilities: [str]` ordenado (campo aditivo; un host antiguo envía la lista vacía); cada cliente oculta en UI lo que su host no soporta; (b) **codecs congelados**: un schema JSON canónico por variante de `AgentMessage`, un codec NDJSON para los eventos de sesión, y un campo `schema: "<nombre>/v1"` en cada evento de alambre; el contrato lo afirma `handshake.rs` y lo prueban `cloud/test/` y los tests del host.
- **Pasos:** 1. Enumerar las variantes de `AgentMessage` de Codync (turno, pensamiento, delta, llamada/resultado de tool, plan, progreso, permiso, pregunta, uso, error, handoff…) y escribir el schema JSON de cada una. 2. `encode(msg) -> bytes` y `decode(bytes) -> msg` puros, con test de ida y vuelta, y `variante desconocida -> AgentMessage::Unknown { kind, raw }` (los clientes antiguos se saltan lo que no entienden sin romperse). 3. El `hello` lleva `capabilities`; el cliente guarda la lista al conectar. 4. Los stores de los 4 clientes exponen `soporta(cap)`; cada vista de una feature nueva consulta su capability y muestra "no disponible en este host" en su lugar. 5. Las adiciones al alambre siempre añaden campos opcionales y nunca reutilizan nombres; el decodificador tolera campos desconocidos. 6. Test de congelación: un golden-file con los bytes actuales de cada variante de mensaje; si cambia sin bump de `schema`, CI falla.
- **Clientes:** los 4 leen `capabilities` en el `hello` (store iOS en `Kit/Sources/CodyncKit/`, store de Android en `app`, store del escritorio en `src/renderer/`, TUI en `host/src/tui/`); cada UI de feature consulta su capability y degrada con un mensaje, no con un fallo.
- **Compat:** puramente aditivo; cliente nuevo + host antiguo = capabilities vacías = todo gateado fuera (degradación documentada, no error); sin bump de `MIN_APP`.
- **Tests:** ida y vuelta golden por variante; matriz capabilities (host viejo × cliente nuevo y viceversa); campo `schema` en eventos nuevos.
- **Esfuerzo:** L.

#### 6.2 Catálogo de modelos + precio por modelo (S1, #34)

- **Fuente:** `agent/model_catalog.rs` (metadatos por proveedor/modelo: contexto, tokens máximos, modalidades, `input_cost_per_million`/`output_cost_per_million`, `cache_input_cost`, `cache_output_cost`, `supports_thinking`); `model_metadata.rs` (precios de respaldo); `provider/agent_model_registry.rs` (resuelve lo que cada proveedor expone).
- **Codync hoy:** `host/src/agent/bot/usage.rs` lleva `tokensIn/tokensOut` y un `Model` por bot; "solo para mostrar"; el evento SSE `usage` existe (`host/src/api/events.rs:210`); no hay catálogo ni costos.
- **Llegada:** el host sirve `models` (proveedor → modelo: ventana de contexto, salida máxima, modalidades, costo por millón de tokens) desde un registro de proveedores + tabla curada (`host/src/agent/model_catalog.rs`); el `usage` ya lleva `model`; la configuración del bot gana `maxOutputTokens` y nivel de `thinking` opcional; las UIs muestran el catálogo en la configuración del bot (selector de modelo iOS en `Kit/Sources/CodyncUI/`, diálogo de Android, configuración del escritorio, `model <bot> <nombre>` en la TUI).
- **Pasos:** 1. Módulo de catálogo con una tabla por defecto (proveedores/modelos mayores: contexto, salida máxima, costo USD por millón) + sobreescritura por proveedor cuando el ACP lo ofrezca. 2. Método API `models list` + evento SSE de catálogo (poco frecuente). 3. Config por bot: `model` (ya existe) + `maxOutputTokens` + `thinkingLevel` opcionales. 4. Los clientes calculan costo = tokens × tarifa del catálogo y lo muestran junto a los tokens ("≈ $X / $Y"); la moneda local usa el `locale` que el teléfono ya envía en el `hello` (devices.rs:59, compat.rs:59). 5. Render en los 4 clientes (selector de modelo con costo; TUI `model <bot> <nombre>`).
- **Compat:** aditivo (`models` + campos opcionales de `usage`); sin bump de `MIN_APP`.
- **Tests:** lookup del catálogo; cálculo de costo por moneda; sobreescritura por proveedor; comando `model` de la TUI.
- **Esfuerzo:** M.

#### 6.3 Gestor de runtimes: pin/verificar/instalación escalonada + estado de credenciales (S3, #26)

- **Fuente:** `agent/runtime.rs` — `RuntimeId { kind, version, sha256 }`; `RuntimeStatus::Available | Downloading { progress } | Verifying { progress } | Error`; versión fijada + `sha256` antes de instalar; instalación escalonada en un directorio gestionado por la app, sin ensuciar el PATH del sistema; máquina de estados de credenciales `missing|saved|unreadable` (deep dive §12.2).
- **Codync hoy:** la app de escritorio gestiona la instalación del binario del host (auto-update, `tools/native-host.mjs`); los harnesses de agentes (Claude Code, Codex, OpenCode, Gemini CLI…) los instala el usuario; no hay concepto de runtime gestionado ni estado de credenciales.
- **Llegada:** el host gana `host/src/agent/runtimes.rs`: tabla de runtimes de agentes soportados (cada uno: origen, `version` fijada, `sha256`, destino de instalación bajo el directorio de datos gestionado, `minOs`) + `RuntimeStatus` por runtime; "instalar" = descargar (progreso) → verificar (sha256) → escalar (mover a su sitio) → disponible; los clientes muestran el estado y ofrecen instalar/actualizar; el estado de credenciales por runtime/agente (`missing | saved | unreadable`) se muestra en el flujo de conexión del agente para que el usuario sepa por qué hace falta un login.
- **Pasos:** 1. `RuntimeDescriptor { id, kind (binario|bundle), versions: [ { version, sha256, url, tamaño, minOs } ] }` en el store (tabla `runtimes`). 2. Motor de descarga/verificación/instalación en el host (evento `runtime: { id, estado }` con progreso). 3. El lanzador de agentes prefiere el runtime gestionado (el directorio gestionado va primero en el PATH de lanzamiento); el usuario puede seguir usando su instalación del sistema. 4. Sonda de credenciales: ejecutar la comprobación de autenticación no interactiva del agente y mapear a `missing|saved|unreadable`; exponerla en la UI de agentes (iOS/Android: ajustes de agente; escritorio: pantalla de agentes; TUI: `agents estado`). 5. Desinstalar/purgar: borrar el directorio escalonado; `RuntimeStatus` vuelve a `missing`.
- **Compat:** aditivo (nuevos eventos `runtime` + métodos API; los clientes viejos lo ignoran).
- **Tests:** sha256 incorrecto → estado de error (sin instalación a medias); secuencia escalar-entonces-verificar; transiciones del estado de credenciales; eventos de progreso de instalación.
- **Esfuerzo:** L.

#### 6.4 Agentes ACP a medida: chequeo de prueba + PATH del shell de login + confinamiento (S4, #26)

- **Fuente:** `agent/agent_runtime.rs` — `spawnAgentRuntime` resuelve el comando desde el PATH del shell de login (no desde el env del proceso servidor); `agent_probe.rs` — antes de confiar en un agente registrado, lanza una sesión de prueba y exige que devuelva un handshake `initialize` válido o un `AuthRequired` tipado; `agent_session_confinement` — el agente corre dentro de una sesión acotada (cwd bajo el workspace, env acotado).
- **Codync hoy:** los harnesses de agentes los lanza el host (`host/src/agent/`); cada harness tiene comando de lanzamiento + env; el login va por el CLI propio del harness (OAuth de Claude Code, etc.); no hay sonda de prueba.
- **Llegada:** (a) "agregar agente" acepta un comando a medida (ruta a binario o argv) + un comando `chequear` opcional; antes de registrar, el host lanza una ejecución de prueba en un directorio desechable y exige una respuesta `initialize` válida del ACP (o un `auth requerido` tipado); si no, rechaza el registro con el stderr capturado; (b) el lanzamiento resuelve PATH desde el shell de login del usuario (ejecutar el shell de login una vez, cachear la salida; en Windows, leer las claves de registro `PATH` de usuario + máquina) para que un binario instalado por el usuario fuera del env del host se encuentre; (c) la sesión del agente a medida va confinada: `cwd` = directorio de workspace del bot, env = set mínimo + variables del usuario, sin secretos heredados.
- **Pasos:** 1. Ampliar la config de `AgentHarness` con `command: [str]` (argv) + `chequear: str?` + `env: [str]`. 2. Resolvedor de PATH del shell de login en el host (caché, se refresca con `agents refresh`). 3. La sonda: lanzar con timeout de 15 s en `data/agent-probe/<id>/`; parsear la respuesta `initialize` del ACP; guardar el resultado como evidencia del registro. 4. Confinamiento: el lanzador fija `cwd` + lista de permisos de env; los archivos del agente caen en el workspace. 5. UI: formulario "agregar agente a medida" en la pantalla de agentes del escritorio + iOS/Android (comando + campo de chequeo), con el resultado de la sonda en vivo (spinner → "handshake OK (v…)" o extracto del stderr).
- **Compat:** aditivo (nuevos campos de config de agente; los clientes viejos usan los harnesses integrados).
- **Tests:** sonda de un binario falso de agente que imprime el handshake del ACP; sonda fallida con captura de stderr; resolución de PATH en Windows (claves de máquina + usuario); confinamiento (un agente de prueba que escribe un archivo lo pone en el workspace).
- **Esfuerzo:** L.

#### 6.5 Ciclo de vida hosteado: reconciliación deseado/observado, wake sellado, guardia de memoria (S6, #59)

- **Fuente:** `host/manager.rs` — máquina de estados `HostLifecycle` `stopped → starting → ready | unavailable | error` con intenciones `start/stop` y `HostWake { hostKey, payloadRef }` ("host wakes are sealed and delivered to exactly one host"); `host/session_recovery.rs` (`HostSessionRecovery`); guardia de memoria (el host valida el payload de memoria antes de aplicarlo al contexto del agente; deep dive §12.7).
- **Codync hoy:** el relay de la nube + el Durable Object ya ejecutan el modo remoto del host; el evento SSE `screen` y los helpers `screen-macos`/`screen-linux` implican un modelo de "host en una máquina"; `host_status`/`screen_url` (api/cloud.rs:503-509) es lo más cercano a un ciclo de vida; no hay reconciliación deseado/observado, ni wake sellado, ni guardia de memoria.
- **Llegada:** el Durable Object gana una tabla `hosts`: por host `deseado: stopped|running` + `observado: offline|starting|ready|unavailable` + `lastSeen`; una intención `start` fija `deseado=running` y emite un wake (un push con un nonce de un solo uso — sellado, consumido una vez, entrega a un solo host); el host ACKea el wake e informa `ready` (los eventos `screen`/estado existentes amplían para llevar el ciclo de vida); la **guardia de memoria**: un payload de memoria que llega de la nube se valida (firma + esquema, formato de alambre de `codync-mem`) antes de aplicarse al contexto de un bot, y un payload no válido se cuarentena con un evento (el teléfono sigue siendo la fuente de verdad; la guardia es el host rechazando memoria no validada).
- **Pasos:** 1. Tabla `hosts` + reductor del ciclo de vida en el Durable Object (función pura; un test por transición). 2. El wake: push con `{ hostKey, nonce, payloadRef }`; el host consume el nonce exactamente una vez (tabla `woken` en el store) e informa `ready`/`unavailable` (con motivo). 3. La guardia de memoria en el camino de alambre de `crates/memory`: validar antes de aplicar; evento `memoria.rechazada` (motivo). 4. Render en clientes: chip de ciclo de vida del host (iOS/Android en inicio, escritorio en bandeja + ventana principal, TUI en cabecera) con `offline|starting|ready|unavailable` + motivo (el `screen_url` ya se muestra donde hay nube configurada). 5. Camino `deseado=stopped`: el host para en limpio (volca el store, cierra las sesiones de agentes) e informa `stopped` — sin estado a medias.
- **Compat:** aditivo (la nube es un deploy separado; `MIN_APP` sin cambios; cliente nuevo + nube vieja = solo `observado`).
- **Tests:** transiciones del reductor; wake consumido una sola vez (test de entrega doble); rechazo de la guardia de memoria (firma inválida / deriva de esquema); parada limpia (sin procesos de agente huérfanos).
- **Esfuerzo:** L. **Solo si Codync va a hostear agentes en un servidor**; si no, este ítem se reduce a 0.5 (recuperación al arrancar) + 2.1 (estado).

#### 6.6 Facturación: recibos idempotentes + deducciones seguras a reintentos (S7, #61)

- **Fuente:** `billing/` — operaciones idempotentes: "billing operations are safe to retry"; cada cargo lleva una clave de idempotencia generada por el cliente; `ledger.ts`/recibos son de solo añadir.
- **Codync hoy:** ninguna (sin facturación; la app es autoalojada/local).
- **Llegada:** solo si Codync añade facturación (créditos, nube de pago): el host (o la nube) mantiene una tabla `billing_events` de solo añadir; cada mutación (carga, deducción, reembolso) lleva una `claveIdempotencia` del cliente (UUID generado una sola vez en el teléfono al formular la intención); una mutación reenviada con la misma clave es un no-op que devuelve el recibo guardado; el teléfono guarda el último recibo por clave para que una caída de red a mitad de la confirmación no doble-cargue; el `hello` lleva `billing: { plan, creditos, … }` para que la UI muestre el saldo desde el host (el host posee la lógica).
- **Pasos:** 1. Tabla `billing_events` (solo añadir, `claveIdempotencia` UNIQUE) + store. 2. Método API `billing: { op: "cargar"|"usar"|"reembolsar", claveIdempotencia, … }` → recibo; clave duplicada → recibo guardado (200, no 409). 3. UI de cartera del teléfono: saldo + estado pendiente mientras la mutación vuela; al confirmarse, el recibo se cachea localmente (Keychain/archivo cifrado) con su clave. 4. Lógica de reintento: el teléfono reintenta la mutación al fallar la red con la misma clave hasta obtener un recibo (la cola del host, 4.8, la hace durable). 5. Auditoría: método `billing historial` para los clientes (lista de solo lectura de recibos).
- **Compat:** subsistema nuevo; sin cambio de `MIN_APP` (una capability `billing` gatea la UI).
- **Tests:** clave duplicada devuelve el mismo recibo (idempotencia); fallo a medias (carga aplicada, recibo perdido) se recupera al reintentar con la misma clave; saldo consistente (suma de eventos = créditos mostrados).
- **Esfuerzo:** L. **Solo si Codync añade facturación**; si no, saltar (la app se mantiene gratis/local).

---

## 6. Transversal (X1–X5)

- **X1. Atribución a upstream.** El repo es PolyForm Noncommercial 1.0.0; la estrategia es reimplementar los contratos, no copiar código (decisiones §1.1). Crear `docs/reference/upstream-attribution.md`: repo, commit (`9491b02…`), licencia, qué se adopta por contrato (la lista de 61 ideas + S1-S7), y la nota "no code copied". Actualizar el commit cada vez que se vuelva a consultar el repo.
- **X2. Documentos vivos.** (a) El mapa de propiedad de estado (S2): cada registro/estado nuevo de esta planificación se registra en una tabla `estado → dueño (host/store/teléfono/cliente) → API que lo muta → evento que lo emite`, dentro de `docs/architecture/`. (b) La regla de S5 "una preocupación, un registro": la cola (0.1), los holds (0.2), el dedup (0.3) y el buffer de deltas (0.6) son registros separados aunque vivan en la misma base de datos; el documento prohíbe fundirlos. (c) Actualizar la lista de docs de feature en el mismo cambio que los hace obsoletos: `docs/architecture/overview.md` (cola persistente, holds, dedup, planificador), `docs/architecture/desktop-app.md` (webhooks, agentes a medida), `docs/guides/desktop-styles.md` (UI de cola/pausa, chip de atención), `docs/reference/compatibility.md` (capabilities en el `hello`, campos opcionales de `usage`), `apps/ios/WhatsNew.md` (lo que el usuario de iPhone nota por fase).
- **X3. Versionado según AGENTS.md.** Cada ítem navega en un release (bump de `MARKETING_VERSION` en `apps/project.yml` + `version` en `host/Cargo.toml`/`Cargo.lock` + `apps/desktop/package.json`/`package-lock.json`, `xcodegen generate`, commit `build: bump version to X.Y.Z`). Las fases aquí son aditivas → sin bump de `MIN_APP`/`MIN_HOST` salvo: 0.3 (si la ventana de dedup se vuelve obligatoria), 4.1 (cambio de esquema de webhook solo si se renombra algo ya emitido) y 6.1 (si el `schema` del alambre deja de ser opcional). El cambio de 2.5 (etapas de memoria) sube la versión 0.x de `codync-mem` una vez por release (CI bloquea un PR a `main` que se la olvide). Para ítems visibles en iPhone (1.3 atención, 2.4 pregunta lateral, 3.x tools de navegador, 4.4 Telegram, 4.8 cola editable, 5.3 plantillas), escribir la sección de esa versión en `apps/ios/WhatsNew.md` (zh-Hant + en-US).
- **X4. Verificación por fase.** Cada fase termina con: `cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`; `cd apps/desktop && npm run typecheck && npm test`; `cd apps/ios/Kit && xcodebuild test -scheme CodyncKit-Package -destination 'platform=iOS Simulator,name=<any iPhone>'`; `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug`; `cd cloud && npm ci && npm test && npm run typecheck` (fases 4/6). E2E por fase: instalar el build nuevo (matar el anterior primero) y recorrer el flujo cambiado: la app de escritorio con Computer Use, el iPhone en el simulador vía Device Hub. E2E mínimo = la plataforma de la máquina de test; e2e completo (macOS, Linux, Windows) para las fases grandes 3/4/6 cuando los VMs estén disponibles.
- **X5. Los 3 commits de Openstark desde el deep-dive** (`b4cb1bce` → `9491b02`). Ya integrados: #2704 (preguntas de agente multi-respuesta → paso 6 de 1.3), #2728 (interrupción por voz → paso 4 de 2.4), #2712 (fijar el chat al último durante el turno → paso 5 de 0.4). Si el repo sigue moviéndose, re-difearlo antes de arrancar cada fase y actualizar la lista de ideas en la misma fecha.

---

## 7. Hitos (M0–M6)

| Hito | Contenido (fases) | E2E que lo demuestra |
|---|---|---|
| **M0** | Fase 0: crate de contratos, store v5, holds nombrados, dedup a 3 capas, recuperación al arrancar, buffer de deltas | La cola sobrevive un reinicio; un hold pausado detiene el bot y sigue el turno; un reenvío duplicado se dedupea; los deltas llegan en orden tras una conexión lenta |
| **M1** | Fase 1: slots de turnos, límites de uso, registro de atención, planes/subagentes | Dos bots trabajando a la vez; un rechazo por límite de uso con motivo; la insignia `needsInput` se muestra en los 4 clientes; el stepper de plan avanza con el plan |
| **M2** | Fase 2: tareas + esperas, ejecución tranquila, pregunta lateral, memoria con propietario, contexto visible, handoff con presupuesto, tool discovery, flujos de rutinas, bot oculto, base compartida, staging de artefactos, política de mensajería | Una tarea con espera se reanuda sola; una ejecución tranquila en canal sin ruido; una pregunta lateral no rompe el turno; un round-trip de staging de memoria; una rutina con flujo completo; un handoff respeta el presupuesto de tokens |
| **M3** | Fase 3: herramientas de navegador (snapshot, tabs, `fill_fields`, secretos, vista, daemon, sandbox HTML) | Un bot llena un formulario con un secreto del vault; el snapshot se vuelve a tomar en el turno siguiente; la vista remota se sirve en la sandbox HTML |
| **M4** | Fase 4: contratos de webhook + ciclo de vida de rutas, webhooks de estado, servidor de automatización + CLI, tickets de relay, Telegram, orquestador, memoria de mensajería + publicación de estado, Discord/Slack, sitios hosteados | Un POST externo inicia una tarea; una conversación de Telegram de ida y vuelta; una cadena de handoff entre bots con memoria de mensajería; una publicación de estado en el canal correcto |
| **M5** | Fase 5: importar zip, transferencia, plantillas, revisiones de skill, catálogo de plugins, higiene del 410 de push, política de notificaciones, cola editable | Un zip importado crea un bot completo; una plantilla instancia un bot con overrides; la puerta "permitir scripts" bloquea un script no aprobado; la cola se reordena y pausa desde el teléfono |
| **M6** | Fase 6: capabilities + codecs congelados, catálogo de modelos + precios, gestor de runtimes + estado de credenciales, agentes ACP a medida [, ciclo de vida hosteado, facturación — solo si se elige] | La UI gateada por capability se degrada con un host viejo; el selector de modelo muestra el costo; un runtime se instala con pin/verificación y el login se marca `saved`; un agente a medida pasa la sonda de prueba |

---

## 8. Riesgos y qué NO copiar

**Riesgos**

1. **El repo se mueve rápido.** El deep-dive tiene 4 commits de atraso (3 en un día) cuando se redactó esta planificación. Mitigación: re-difear antes de cada fase; citar el commit en cada doc; tratar esta planificación como contrato, no como port.
2. **Normalización de errores de harness (1.2).** Los 5 harnesses (Claude Code, Codex, OpenCode, Gemini, + ACP) reportan errores de forma distinta; mapearlos a un solo `turnError` arriesga veredictos falsos de "recuperable". Mitigación: tabla por harness; por defecto `fatal` si es desconocido; un test por harness.
3. **Cambio de store v5→v6 (0.1–0.3 + 2.6) vs. la invariante de `entries` append-only + `emit_lock`.** El teléfono renderiza `entries`; cola/holds son del host. Mitigación: versión de esquema por tabla + un `emit_lock` monótono por bot (los `entries` de un bot solo se emiten tras liberar el lock); test de que un reinicio nunca re-emite ni pierde un `entry`.
4. **Riesgo de relay de la fase 4** (webhooks + Telegram + orquestador = endpoints públicos nuevos + superficie de credenciales nueva). Mitigación: worker de relay con allow-list + límite de tasa; secretos por la rotación de claves existente (X.25519); clasificación `LOCAL_ONLY` por método nuevo; revisión de seguridad antes de publicar.
5. **Costo de 4 clientes por ítem de UI.** Todo ítem con UI toca iOS + Android + escritorio + TUI (regla de cambios de UI entre plataformas de AGENTS.md). Mitigación: agrupar la UI por fase (un PR por plataforma por fase, no por ítem); la TUI recibe primero una superficie mínima viable; usar las librerías de componentes compartidas (iOS `CodyncUI/Controls.swift`, Android `design/Controls.kt`, escritorio `components/`).

**Qué NO copiar**

- **El stack Electron/Bun/SolidJS.** Codync es Rust + SwiftUI + Compose + Electron-React; lo que se transfiere es el contrato, no el código.
- **Signal/WebRTC como servicio.** El canal teléfono↔host de Codync es propio; "el teléfono como remoto de primera clase" es la idea a adoptar, la criptografía no.
- **Chats de canal** (Discord/Slack como superficie de grupo de primera clase). La política de mensajería (2.13) basta.
- **El negocio "boat"** (app store / Stripe / modelo de suscripción). 6.6 solo si Codync añade facturación.
- **Aprobaciones por defecto desactivadas.** Los agentes de Openstark corren autónomos por defecto; Codync mantiene `ask|auto` por bot y la puerta `notify` — la autonomía es opt-in.
- **El límite de 100k caracteres como tal.** Codync elige el suyo; lo importante es "un límite, hecho visible", no el número.
- **El runtime Effect-TS.** El host Rust tiene `tokio`; la disciplina de concurrencia estructurada es la idea, no la librería.
- **El modelo de equipos multi-cuenta/host compartido.** Codync es de un solo propietario por host; el modelo de equipo es una decisión de producto, no un port.
