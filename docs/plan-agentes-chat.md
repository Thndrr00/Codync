# Plan: los siguientes bloques de trabajo

Lo que sigue a los once patrones de agentes.chat. Cada item es una unidad de un día o menos; los
bloques están ordenados por dependencia, no por atractivo.

Convenciones: **host** es `host/` (Rust, SQLite, API); **clientes** son escritorio, TUI, Android e
iOS; **ADR** son las decisiones de agentes.chat (`docs/adr` de su repo) que hay que leer antes de
diseñar.

---

## Bloque 0 — Cerrar lo abierto (20 items)

El trabajo de los once patrones está escrito y sin verificar. Nada de lo demás debería empezar antes.

1. `cargo check --all-targets` en `host/` y corregir lo que salga.
2. `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings`.
3. `cargo test` completo (1136 pruebas) y revisar las que fallan por entorno.
4. `cd apps/desktop && npm run typecheck`.
5. `cd apps/desktop && npm test`.
6. `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug`.
7. `cd apps/ios/Kit && xcodebuild test -scheme CodyncKit-Package`.
8. E2E escritorio: tarjeta aprobada dos veces desde dos dispositivos.
9. E2E escritorio: documento con `[[enlaces]]` y backlinks.
10. E2E escritorio: versiones de un archivo publicado dos veces.
11. E2E escritorio: lección propuesta tras dos rechazos y aceptada.
12. E2E escritorio: base de datos de solo lectura y una escritura rechazada.
13. E2E escritorio: tablero (`botTasks`) con una rutina en curso.
14. E2E escritorio: grafo con tres páginas enlazadas.
15. E2E Android: tarjeta, versiones y lecciones.
16. E2E iOS Simulator: tarjeta y versiones.
17. Sección de esta versión en `apps/ios/WhatsNew.md` (zh-Hant + en-US).
18. Sección de esta versión en `apps/android/WhatsNew.md`.
19. Bump de versión en los cuatro sitios y `xcodegen generate --spec apps/project.yml`.
20. Push de la rama y borrado del worktree `Codync-agentes-chat`.

---

## Bloque 1 — Misiones (20 items)

**Meta:** un objetivo que sobrevive a los turnos. Hoy una rutina termina y se olvida; una misión
recuerda qué se hizo, qué falta y por qué se paró.

1. Leer los ADR de misiones y de runs de agente.
2. Definir el modelo: misión, paso, estado, motivo de parada.
3. Tabla `missions` y `mission_steps` en SQLite, con migración numerada.
4. API `createMission`, `missions`, `mission`, `updateMission`.
5. Estados: `proposed`, `active`, `waiting`, `done`, `abandoned`.
6. Vincular una misión con el run de rutina que la ejecuta.
7. Vincular una misión con la aprobación que la bloquea.
8. Checkpoints: qué se hizo y qué falta, escritos por el bot.
9. Reanudar una misión tras reiniciar el host.
10. Cancelar con motivo, sin borrar el historial.
11. Herramienta MCP `update_mission` para que el bot avance su propio estado.
12. Límite de misiones activas por bot y por flota.
13. Vista de misiones en escritorio (lista + detalle con pasos).
14. Vista en TUI (hoja con pasos y teclas de avanzar/abandonar).
15. Vista en Android.
16. Vista en iOS.
17. Push al completarse o al bloquearse.
18. Búsqueda de misiones por título y por bot.
19. Tests de host: transiciones válidas e inválidas, reinicio, cancelación.
20. Documento `docs/features/missions.md`.

---

## Bloque 2 — Búsqueda y contexto (20 items)

**Meta:** recuperar con evidencia. Hoy el contexto del turno es lo reciente; con búsqueda es lo
relevante, con citas.

**Estado:** 2, 3 (también al editar y al borrar), 6, 8, 9, 10, 11, 16, 17, 19 y 20 hechos. El índice
es el mismo que ya usan las cuatro apps para la búsqueda de historial; falta el boceto de resultados
por bot en escritorio, TUI, Android e iOS (12–15), el peso por recencia (7), los filtros de 5 y el
índice de memoria (4).

1. Leer los ADR de búsqueda y de contexto.
2. Índice FTS5 sobre `entries` (texto del usuario, del agente y de las herramientas).
3. Indexado incremental al insertar y al editar.
4. Índice separado para documentos y para memoria.
5. Búsqueda por bot, canal, autor y rango de fechas.
6. Citas: `entryId`, fragmento y fecha en cada resultado.
7. Ranking: BM25 con peso por recencia.
8. Contexto automático del turno: los N resultados más relevantes.
9. Tope de tokens del contexto, configurable por bot.
10. Excluir mensajes internos y trazas de herramientas del índice por defecto.
11. API `search {botId, query, filters, limit}`.
12. UI de búsqueda en escritorio (con resaltado y salto al mensaje).
13. UI en TUI (línea de búsqueda y resultados navegables).
14. UI en Android.
15. UI en iOS.
16. Privacidad: qué nunca se indexa (secretos, adjuntos binarios).
17. Backfill del historial existente, reanudable.
18. Invalidación al borrar un bot o un canal.
19. Tests: ranking, filtros, citas, backfill parcial.
20. Documento `docs/features/search.md`.

---

## Bloque 3 — Cerebros: memoria por proyecto (18 items)

**Meta:** un tercer ámbito de memoria. Hoy hay `MEMORY.md` por bot y memoria compartida; falta el
ámbito de proyecto, que varios bots comparten sin mezclarse con lo personal.

1. Leer los ADR de cerebros y de memoria.
2. Definir ámbitos: personal (bot), compartido (flota), proyecto (elegido).
3. Modelo: `brains` con id, nombre y miembros.
4. Resolución del ámbito por bot: personal → proyecto → compartido.
5. API `brains`, `createBrain`, `joinBrain`, `leaveBrain`.
6. Herramientas del bot para leer y escribir en su cerebro de proyecto.
7. Migración de la memoria compartida actual a un cerebro "Flota".
8. Compactación por ámbito, sin mezclar.
9. Dream por ámbito, con evidencia del ámbito.
10. Límites por cerebro y totales.
11. UI de ajustes (elegir cerebro) en escritorio.
12. UI en TUI.
13. UI en Android.
14. UI en iOS.
15. Búsqueda dentro de un cerebro (depende del bloque 2).
16. Tests: resolución de ámbito, aislamiento entre proyectos.
17. Export e import de un cerebro completo.
18. Documento `docs/features/brains.md`.

---

## Bloque 4 — GitHub App (16 items)

**Meta:** webhooks por repositorio sin el token de una persona, para que A3 dispare rutinas de
verdad.

1. Leer el ADR de GitHub App y el módulo `host/src/github`.
2. Registrar la App y documentar el manifiesto.
3. Instalación por repositorio y por organización.
4. Webhook firmado (HMAC) con verificación y ventana anti-replay.
5. Tokens de instalación con caducidad y renovación.
6. Mapeo repositorio → bot (uno o varios).
7. Permisos mínimos: contents, pull requests, checks.
8. Eventos soportados: `pull_request`, `pull_request_review`, `check_run`, `issues`.
9. Deduplicación por `delivery` id.
10. Entrega a rutinas por evento, con filtros por repo y por rama.
11. UI de conexión en escritorio (instalar, elegir repos, mapear bots).
12. Estado en TUI.
13. Estado en Android (solo lectura).
14. Estado en iOS (solo lectura).
15. Tests: firma inválida, replay, evento sin bot, token caducado.
16. Documento `docs/features/github-app.md`.

---

## Bloque 5 — Módulos y extensiones (20 items)

**Meta:** el punto de extensión que hoy no existe. Es el bloque más caro y el que más cambia la
arquitectura; no empezar sin los cuatro anteriores cerrados.

1. Leer los ADR de ediciones y módulos, y el `packages/modules` de agentes.chat.
2. Definir el contrato: qué puede un módulo (herramientas, vistas, eventos).
3. Definir qué no puede: permisos, aprobaciones, red sin declarar.
4. Carga de módulos desde disco con manifiesto firmado.
5. Aislamiento de procesos para módulos (fuera del actor del bot).
6. Registro de herramientas del módulo en el servidor MCP del bot.
7. Registro de vistas del módulo en los clientes.
8. Declaración de permisos y su aprobación por el dueño.
9. Cuotas por módulo: tiempo, memoria, llamadas.
10. Ciclo de vida: instalar, actualizar, desinstalar, deshabilitar.
11. Compatibilidad: versión del contrato y rechazo de módulos nuevos.
12. Registro de eventos que un módulo puede escuchar.
13. Registro de eventos que un módulo puede emitir.
14. UI de módulos en escritorio (instalar, permisos, cuotas).
15. UI en TUI.
16. UI en Android.
17. UI en iOS.
18. Sandbox de sistema operativo por plataforma.
19. Tests: módulo que pide de más, que se cuelga, que no cierra.
20. Documento `docs/architecture/modules.md`.

---

## Bloque 6 — Deuda y calidad (18 items)

**Meta:** lo que se acumuló mientras se construía.

1. `fileVersions`: paginar en vez de mirar 500 mensajes.
2. `documentGraph`: paginar nodos y aristas.
3. Ledger: retención y poda verificadas con datos reales.
4. Ledger: decidir si las reglas automáticas dejan registro (hoy no).
5. `sql`: probar el límite de tiempo con una consulta lenta de verdad.
6. `sql`: soportar más de un motor o documentar por qué solo SQLite.
7. A4: export a Word/PDF con una cadena de conversión.
8. A5: adaptador S3 con firma SigV4.
9. A9: estado de toma ("quién la tiene, desde cuándo, por qué") y pausa del agente.
10. A10: un lector por origen nuevo, empezando por el que más se pida.
11. Grafo en Android.
12. Grafo en iOS.
13. Notas de versión por app, como paso del release.
14. Un `AGENTS.md` más corto: mover detalle a `docs/`.
15. Revisar los 40 ADR y anotar en `docs/` lo que cambie decisiones nuestras.
16. Prueba real en Codync Dev: tres agentes, correo aprobado una vez, rutina por PR.
17. Informe por patrón en `docs/features/agentes-chat.md`.
18. Limpiar los archivos sueltos de la raíz del checkout.

---

## Orden y esfuerzo

| Bloque | Depende de | Esfuerzo |
|---|---|---|
| 0 Cerrar lo abierto | — | 3–5 días |
| 1 Misiones | 0 | 2 semanas |
| 2 Búsqueda y contexto | 0 | 1,5 semanas |
| 3 Cerebros | 0, 2 | 1,5 semanas |
| 4 GitHub App | 0 | 1 semana |
| 5 Módulos | 0–4 | 4–6 semanas |
| 6 Deuda | 0 | 1 semana, repartida |

Dos reglas para no repetir el error del documento original: **verificar antes de estimar** (su
inventario estaba desfasado en 4 de 11) y **no abrir un bloque sin compilar el anterior**.
