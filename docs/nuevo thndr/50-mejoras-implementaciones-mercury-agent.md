# 50 Mejoras e Implementaciones Extraídas de mercury-agent para Codync

**Repositorio de origen:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent` (`cosmicstack-labs/mercury-agent`)  
**Licencia:** MIT (© Cosmic Stack; revisado sobre `fa3eeaa`, v1.4.2; el commit `397200d` citado antes no existe)  
**Destino en Codync:** `host/` (Rust daemon, `crates/memory`), `apps/desktop/` (Electron/React/TS), `apps/android/` (Kotlin/Compose), `apps/ios/` (SwiftUI), `apps/screen-*`.  
**Fecha de elaboración:** 2026-10-25

---

## Estructura por Bloques Temáticos

- **Bloque 1: Implementación de Lógica Core, Memoria Probabilística y Orquestación de Flota (Ideas 1 – 15)**
- **Bloque 2: UI / UX, Canales Interactivos, Kanban y Visualización (Ideas 16 – 30)**
- **Bloque 3: Rendimiento, Gobernanza, Seguridad y Resiliencia (Ideas 31 – 40)**
- **Bloque 4: Adición de Opciones, Ajustes de Usuario, Automatizaciones y Herramientas (Ideas 41 – 50)**

---

## Bloque 1: Implementación de Lógica Core, Memoria Probabilística y Orquestación de Flota

### 1. Tabla de Creencias Probabilísticas con Contadores de Evidencia a Favor y en Contra
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\memory\beliefs.ts` (Líneas 1-70) y `docs/owner-model.md` (ADR-026).
- **Qué hace el código:** Modela cada recuerdo no como una cadena estática, sino como una creencia (`Belief`) con `support: number` (sesiones donde se verificó) y `against: number` (veces que el usuario contradijo la afirmación).
- **Qué hacer con ese código en Codync:** Crear la tabla `beliefs` en `host/crates/memory/src/schema.rs`. En lugar de sobreescribir memorias en `codync-mem`, registrar cada observación incrementando `support` o `against` según la retroalimentación del usuario.

### 2. Inferencia de Rasgos Funcionales de Trabajo (`TRAIT_DIMENSIONS`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\memory\beliefs.ts` (Líneas 24-32).
- **Qué hace el código:** Define dimensiones fijas que alteran directamente el comportamiento del modelo: `brevity` (`terse` | `balanced` | `thorough`), `autonomy` (`ask-first` | `ask-when-risky` | `just-do-it`), `decision-style` (`options` | `recommendation` | `decide-for-me`), `formality` (`casual` | `neutral` | `formal`), `feedback-style` y `rhythm`.
- **Qué hacer con ese código en Codync:** Implementar el extractor de rasgos en `host/crates/memory/src/traits.rs`. El resultado calibra dinámicamente el system prompt del host inyectando la sección compacta "Cómo trabajar con este usuario".

### 3. Rasgos de Trabajo con Ámbito de Dominio (`SCOPED_DIMENSIONS`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\memory\beliefs.ts` (Líneas 33-38).
- **Qué hace el código:** Permite registrar rasgos específicos por tecnología o área: `expertise:<dominio>` (`novice` | `competent` | `expert`) y `risk:<dominio>` (`cautious` | `normal` | `bold`). Por ejemplo: `expertise:rust = expert`, pero `expertise:kubernetes = novice`.
- **Qué hacer con ese código en Codync:** Incluir en `host/crates/memory/src/traits.rs`. Cuando el usuario trabaja en un repositorio de Rust, el agente adopta explicaciones avanzadas; en un dominio donde es novato, provee explicaciones didácticas.

### 4. Máquina de Estados del Ciclo de Vida de Creencias
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\memory\beliefs.ts` (Líneas 20-22 y 50-65).
- **Qué hace el código:** Estados formales: `candidate` (observado 1 vez), `held` (observado en >= 2 sesiones independientes), `confirmed` (validado por el usuario), `contested` (evidencia contradictoria), `retired` y `forgotten` (el dueño pidió olvidarla).
- **Qué hacer con ese código en Codync:** Implementar la máquina de estados en `host/crates/memory/src/lifecycle.rs`. Solo las creencias en estado `held` o `confirmed` alteran el comportamiento del agente, evitando que comentarios aislados deformen el sistema.

### 5. Registro Bitemporal con Timestamps de Validez y Sin Borrado Destructivo
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\memory\beliefs.ts` (Líneas 57-65).
- **Qué hace el código:** Campos `validFrom`, `validTo` y `supersededBy`. Cuando una creencia queda desfasada, no se elimina; se fija su `validTo` y se apunta a la nueva creencia que la sustituye.
- **Qué hacer con ese código en Codync:** Integrar campos bitemporales en las tablas SQLite de `host/crates/memory`. Esto mantiene una auditoría histórica perfecta de la evolución de las preferencias del usuario.

### 6. Notificacin Transparente de Aprendizaje en Chat con Opcin Deshacer
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\DECISIONS.md` (ADR-026, Líneas 245-251).
- **Qué hace el código:** Cuando una creencia pasa a `held` o se actualiza un rasgo que cambia la conducta, el agente emite una notificación de una sola línea al final del mensaje: *"He asumido que prefieres respuestas concisas en terminal (deshacer)"*.
- **Qué hacer con ese código en Codync:** En `host/src/agent/delivery.rs`, verificar si el turno produjo una promoción de creencia y anexar el aviso con una acción rápida de revocación para los clientes Desktop y Móvil.

### 7. Tareas Tipadas y Estructuradas para Flotas de Bots (`BotTask`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\tasks.ts` (Líneas 1-60) y ADR-021.
- **Qué hace el código:** Modela unidades de trabajo delegadas: `id`, `requester`, `assignee`, `goal`, `acceptanceCriteria`, `status` (`queued` | `running` | `done` | `failed`), `jobId` y `result` estructurado.
- **Qué hacer con ese código en Codync:** Crear el subsistema de tareas de flota en `host/src/fleet/tasks.rs`. Reemplazar los mensajes de texto libre entre agentes por contratos de tareas tipadas con criterios de aceptación explícitos.

### 8. Despacho y Fan-In de Lotes con Poltica de Despertar nica
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\tasks.ts` (Líneas 70-130) y `src\bots\bot-manager.ts:58-66`.
- **Qué hace el código:** Permite que un bot coordinador envíe N tareas con `wakePolicy: 'all'` y un tiempo límite. El coordinador se suspende y **despierta una sola vez** cuando todos los bots subordinados han concluido, evitando N turnos intermedios.
- **Qué hacer con ese código en Codync:** Implementar `BatchDispatch` en `host/src/fleet/batch.rs`. Ahorra entre 70% y 85% de tokens en flujos de trabajo multi-bot complejos.

### 9. Compilador de Resumen Agregado de Lotes (`renderBatchDigest`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\tasks.ts` (Líneas 140-180).
- **Qué hace el código:** Genera un resumen compacto que recopila los entregables, el estado de cada tarea y las conclusiones de los subordinados, entregándoselo en bandeja al bot lder para su sntesis final.
- **Qué hacer con ese código en Codync:** Portar la función de generación de digests a `host/src/fleet/digest.rs`.

### 10. Pipelines de Flota Basados en Datos Puros (`manifest.pipeline.stages`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\bot-manager.ts` (Líneas 207-211) y ADR-021.
- **Qué hace el código:** Define secuencias lineales de bots donde la salida de una etapa se conecta automáticamente como entrada de la siguiente (`input: "{{previous}}"`), sin turnos de coordinación en el medio.
- **Qué hacer con ese código en Codync:** Modelar pipelines declarativos en `host/src/fleet/pipeline.rs`. Permite encadenar flujos (ej. Bot Investigador -> Bot Redactor -> Bot Revisor) de forma 100% automatizada.

### 11. Auditora de Traza y Veredicto Antialucinacin (`claimedWithoutAction`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\bot-turn.ts` (Líneas 1-80) y ADR-020.
- **Qué hace el código:** Clasifica la traza real de herramientas en: `deliverable`, `action`, `delegated`, `message` o `none`. Si el bot dice que completó la tarea pero el resultado es `none`, detecta la alucinación y fuerza un reintento.
- **Qué hacer con ese código en Codync:** Integrar el auditor de traza en `host/src/agent/verdict.rs`. Evita que el agente cierre turnos falsamente sin haber escrito los archivos o corrido los comandos requeridos.

### 12. Pausa Preventiva de Rutinas Cron tras Racha de Inaccin
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\bot-manager.ts` (Líneas 190-195) y ADR-020.
- **Qué hace el código:** Si una rutina programada por cron ejecuta 3 turnos seguidos clasificados con veredicto `none`, el gestor suspende la rutina para evitar consumo intil de crditos y notifica al usuario.
- **Qué hacer con ese código en Codync:** Guardar el contador de inaccin en `host/src/routines/state.rs`.

### 13. Gestor de Bloqueo Concurrente de Archivos Multi-Lector / Escritor nico (`FileLockManager`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\file-lock.ts` (Líneas 1-70).
- **Qué hace el código:** Implementa bloqueos reader-writer sobre rutas de archivo normalizadas. Varios agentes pueden leer simultneamente, pero una escritura exige exclusividad absoluta.
- **Qué hacer con ese código en Codync:** Crear `host/src/fs/file_lock.rs` utilizando `tokio::sync::RwLock` indexado por `PathBuf` cannico.

### 14. Arquitectura de Runtime nico con Terminal Acoplado (`bootDaemonThenAttach`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\DECISIONS.md` (ADR-024, Líneas 213-219).
- **Qué hace el código:** Garantiza que un solo proceso del sistema operativo sea el runtime propietario de la base de datos, bots y canales. Cualquier terminal adicional se acopla como cliente va socket/IPC sin competir por la base de datos ni por los puertos.
- **Qué hacer con ese código en Codync:** Reforzar el inicio del CLI y de la app de escritorio en `host/src/main.rs` para detectar demonios locales activos y desacoplar la interfaz del bucle de ejecucin.

### 15. Deteccin y Recuperacin de Cadas Inesperadas (`crash-flag.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\crash-flag.ts` (Líneas 1-50).
- **Qué hace el código:** Mantiene un archivo de bandera durante la ejecucin. Si el proceso termina abruptamente (reinicio del sistema o kill forzoso), al volver a arrancar detecta la bandera, limpia bloqueos hurfanos y reanuda las tareas incompletas.
- **Qué hacer con ese código en Codync:** Integrar en el ciclo de arranque de `host/src/lifecycle.rs`.

---

## Bloque 2: UI / UX, Canales Interactivos, Kanban y Visualización

### 16. Tablero Kanban Transaccional en Tiempo Real con Server-Sent Events
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\web\api\kanban.ts` (Líneas 1-120) y `src\core\board-manager.ts`.
- **Qué hace el código:** Expone una API completa de tableros Kanban (`pending`, `running`, `paused`, `done`, `failed`) con streaming de eventos va SSE (`/api/kanban/events`).
- **Qué hacer con ese código en Codync:** Exponer `/api/kanban` en `host/src/api/kanban.rs`. Crear la vista interactiva `apps/desktop/src/renderer/views/KanbanView.tsx` y en Android `apps/android/app/src/main/java/app/codync/app/kanban/KanbanScreen.kt`.

### 17. Vinculacin Bidireccional de Tarjetas Kanban con Sub-Agentes
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\web\api\kanban.ts` (Líneas 45-70).
- **Qué hace el código:** Mantiene mapas en memoria `cardAgentMap` y `agentCardMap`. Mover una tarjeta a la columna "En Progreso" despierta a un sub-agente asignado; cuando el sub-agente termina su turno, la tarjeta se mueve automticamente a "Hecho".
- **Qué hacer con ese código en Codync:** Conectar el dispatcher del Kanban con el supervisor de sub-agentes en `host/src/kanban/agent_bridge.rs`.

### 18. Grafo Visual de Dependencias entre Tarjetas (`dependsOn`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\web\api\kanban.ts` (Líneas 31-33).
- **Qué hace el código:** Permite definir que la Tarjeta B depende de la Tarjeta A. El agente asignado a B espera pasivamente hasta que A cambie a estado `done`.
- **Qué hacer con ese código en Codync:** Agregar la resolucin de dependencias en `host/src/kanban/scheduler.rs` y renderizar lneas de conexin visuales en la interfaz Desktop.

### 19. Alerta y Pausa Visual en Tarjetas por Agotamiento de Presupuesto (`pausedForTokens`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\web\api\kanban.ts` (Lnea 25).
- **Qué hace el código:** Si una tarjeta alcanza su lmite de tokens fijado (`tokenBudget`), se congela automticamente mostrando una insignia mbar de advertencia y un botn para conceder ms presupuesto.
- **Qué hacer con ese código en Codync:** Renderizar el estado de pausa en `KanbanCard.tsx` permitiendo al usuario reanudar la tarjeta con un clic.

### 20. Visualizador Grfico de la Red de Creencias y Personas (`Graphology`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\web\api\brain.ts` (Líneas 1-100) y `src\web\static\vendor\graphology.min.js`.
- **Qué hace el código:** Genera una red de nodos y relaciones entre hechos, creencias, proyectos y personas vinculadas al usuario, sirviendo los datos para renderizado de grafos en 2D interactivo.
- **Qué hacer con ese código en Codync:** Crear `host/src/api/brain_graph.rs` y el componente de visualizacin en `apps/desktop/src/renderer/components/BrainGraphView.tsx`.

### 21. Botones Interactivos en Canales Mapeados a Comandos (`sendWithActions`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\channels\telegram.ts` (Líneas 1-80) y ADR-023.
- **Qué hace el código:** Enva mensajes con botones interactivos cuyos callbacks contienen tokens cortos (`ba:<token>`). Al ser pulsados, el canal los traduce internamente a un comando normal (`/bots stop ceo`), permitiendo operar la flota desde el mvil sin teclear comandos.
- **Qué hacer con ese código en Codync:** Adoptar la abstraccin `sendWithActions` en los canales remotos del host (`host/src/channels/`).

### 22. Entrega y Descarga Directa de Archivos al Chat Mvil (`setSendFile`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\bot-manager.ts` (ADR-023, Líneas 9-12).
- **Qué hace el código:** En lugar de devolver una ruta local inalcanzable (`/Users/admin/reporte.pdf`), el host despacha el binario real del archivo generado como adjunto descargable directamente al chat.
- **Qué hacer con ese código en Codync:** Modificar la entrega de artefactos en `host/src/tools/deliver.rs` para que adjunte el archivo fsico en las notificaciones mviles y push.

### 23. Indicador de Pulso y Latido en Segundo Plano (`progress-pulse.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\progress-pulse.ts` (Líneas 1-40).
- **Qué hace el código:** Emite seales peridicas de bajo impacto ("pulso") durante pasos largos de ejecucin, manteniendo al usuario informado de que el agente est trabajando y no colgado.
- **Qué hacer con ese código en Codync:** Integrar en el stream SSE de eventos del host para alimentar el indicador de estado en la barra de tareas.

### 24. Tema Terminal Adaptativo para TUI con Renderer Patcheado
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\utils\terminal-theme.ts` y ADR-019.
- **Qué hace el código:** Deteccin automtica de fondo oscuro/claro en terminal y paletas adaptativas de color ANSI para renderizado de alta fidelidad.
- **Qué hacer con ese código en Codync:** Mejorar la interfaz de lnea de comandos en `host/src/tui/`.

### 25. Vista de Novedades y Registro de Cambios (`whats-new.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\utils\whats-new.ts` (Líneas 1-60).
- **Qué hace el código:** Compara la versin instalada contra la ltima ejecucin y muestra automticamente un dilogo elegante con las novedades ms destacadas de la versin.
- **Qué hacer con ese código en Codync:** Integrar en `apps/desktop/src/renderer/components/WhatsNewModal.tsx` leyendo de `apps/ios/WhatsNew.md`.

### 26. Modo de Programacin Automtico (`coding-task-detect.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\programming-mode.ts` (`coding-task-detect.ts` solo existe como `.test.ts`; no hay fuente con ese nombre).
- **Qué hace el código:** Detecta si la peticin involucra cdigo fuente y activa automticamente un modo estricto de desarrollo (baja temperatura, formato de diffs, anlisis esttico antes de confirmar).
- **Qué hacer con ese código en Codync:** Integrar en el clasificador de intenciones del host (`host/src/agent/intent.rs`).

### 27. Modo de Investigacin Exhaustiva (`research-intent.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\research-intent.ts` y `research-mode.ts`.
- **Qué hace el código:** Ajusta las herramientas y el prompt hacia la sntesis de fuentes, extraccin de citas y verificacin cruzada de hechos.
- **Qué hacer con ese código en Codync:** Agregar a la seleccin automtica de modo de ejecucin en `host/src/agent/mode.rs`.

### 28. Monitor de Tareas de Fondo con Gestin de Procesos Hijos (`background-tasks.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\background-tasks.ts` (Líneas 1-80).
- **Qué hace el código:** Registra procesos lanzados en background con asignacin de ID, envo de entradas va stdin y captura de logs en tiempo real.
- **Qué hacer con ese código en Codync:** Mejorar el administrador de background tasks del host en `host/src/tasks/manager.rs`.

### 29. Puerta de Notificaciones Filtrada contra Saturacin (`alert-gate.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\alert-gate.ts` (Líneas 1-50).
- **Qué hace el código:** Algoritmo que evala la urgencia y relevancia de un mensaje antes de emitir una notificacin al sistema operativo, filtrando alertas redundantes o triviales.
- **Qué hacer con ese código en Codync:** Aplicar sobre el sistema de notificaciones en `host/src/notifications/gate.rs`.

### 30. Controlador de Ambiente y Msica de Trabajo Spotify
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\web\api\spotify.ts` (Líneas 1-90).
- **Qué hace el código:** Permite al agente gestionar la msica de fondo del usuario (reproducir listas de concentracin, pausar, saltar canciones) segn la fase de trabajo en curso.
- **Qué hacer con ese código en Codync:** Opcional: Integrar como plugin o conector multimedia en `host/src/plugins/spotify.rs`.

---

## Bloque 3: Rendimiento, Gobernanza, Seguridad y Resiliencia

### 31. Demonio Nativo en Rust Optimus para Companion de Escritorio
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\plugins\optimus\companion\` (Crates Rust nativos).
- **Qué hace el código:** Demonio de bajo nivel con APIs directas para captura de pantalla, inyeccin de eventos de teclado/ratn y streaming de audio de ultra baja latencia.
- **Qué hacer con ese código en Codync:** Incorporar la base de Optimus en `host/src/companion/` para dotar a Codync de un companion universal en Windows, macOS y Linux.

### 32. Parada Fsica Inmediata al Detectar Movimiento de Ratn de Hardware
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\plugins\optimus\companion\src\motion.rs`.
- **Qué hace el código:** Monitoriza el delta de movimiento del puntero del ratn. Si el usuario mueve el ratn fsico mientras el agente est operando, interrumpe el ciclo de Computer Use instantneamente y devuelve el control al humano.
- **Qué hacer con ese código en Codync:** Integrar en el bucle de Computer Use de `host/src/screen/motion_guard.rs`.

### 33. Barrera Infranqueable de Credenciales Sensibles en Pantalla
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\plugins\optimus\companion\src\ops.rs` y README.
- **Qué hace el código:** Inspecciona los campos de formulario antes de teclear. Si detecta campos de contraseas, tarjetas de crdito o cdigos 2FA, se niega categricamente a introducirlos y solicita al usuario que los teclee manualmente.
- **Qué hacer con ese código en Codync:** Integrar en la herramienta de accin de pantalla en `host/src/screen/act.rs`.

### 34. Canal de Audio Dplex en Tiempo Real con Modelos Multimodales
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\plugins\optimus\companion\src\links\realtime.rs` y `gemini.rs`.
- **Qué hace el código:** Conexin WebSocket persistente con endpoints de voz en tiempo real (Gemini Realtime API / Cartesia), gestionando cancelacin de eco y chunking de audio en formato PCM 24kHz.
- **Qué hacer con ese código en Codync:** Implementar el cliente de audio bidireccional en `host/src/voice/realtime.rs`.

### 35. Tecla Global Push-to-Talk con Sealizacin Acstica No Invasiva
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\plugins\optimus\companion\src\hotkey.rs` y `audio.rs`.
- **Qué hace el código:** Captura el atajo global (ej. `Ctrl+Alt+M`) en cualquier aplicacin del sistema operativo y reproduce chimes suaves (inicio de escucha, fin de sesin, tic suave de trabajo de fondo).
- **Qué hacer con ese código en Codync:** Implementar el gestor de atajos globales en `apps/desktop/src/main/hotkey.ts` y los efectos de audio en el proceso principal.

### 36. Capturas de Pantalla Efmeras sin Retencin en Disco ni Historial
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\plugins\optimus\README.md`.
- **Qué hace el código:** Las imgenes de pantalla capturadas durante la navegacin se envan nicamente al paso actual del modelo y se descartan inmediatamente sin guardarse en logs persistentes ni historiales de chat.
- **Qué hacer con ese código en Codync:** Aplicar la poltica de efimeridad en `host/src/screen/capture.rs` para proteger la privacidad visual del usuario.

### 37. Watchdog Antiparada para Herramientas Bloqueadas (`StallWatchdog`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\stall-watchdog.ts` (Líneas 1-60).
- **Qué hace el código:** Temporizador de vigilancia que monitoriza llamadas a herramientas. Si una herramienta no emite seales ni concluye tras un tiempo mximo configurable, cancela la ejecucin y libera el hilo.
- **Qué hacer con ese código en Codync:** Integrar en el ejecutor asncrono de herramientas en `host/src/tools/watchdog.rs`.

### 38. Gobernador de Memoria y Lmites Dinmicos de Contexto
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\memory-governor.ts` y `context-window.ts`.
- **Qué hace el código:** Monitoriza el crecimiento del contexto del modelo en cada turno. Si el contexto o los resultados de herramientas superan un presupuesto (`CONVERSATION_TOOL_BUDGET_CHARS` = 512 KiB; conserva los 8 resultados recientes, `TOOL_RESULT_KEEP_RECENT`), resume lo antiguo y sigue en vez de abortar. (Corregido: el original decía "80% de la ventana"; el 80% es el aviso del presupuesto diario de tokens, idea 47.)
- **Qué hacer con ese código en Codync:** Incorporar en `host/src/agent/context_governor.rs`.

### 39. Lmite Presupuestario de Archivos en la Carpeta Compartida (`_shared/`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\retention.ts` (ADR-020).
- **Qué hace el código:** Limita las escrituras en carpetas compartidas entre bots: avisa a partir de 2 archivos / 32KB y deniega tajantemente superados los 6 archivos / 128KB, evitando que los bots saturen el disco con notas intermedias.
- **Qué hacer con ese código en Codync:** Aplicar cuotas de escritura sobre carpetas compartidas en `host/src/fs/quota.rs`.

### 40. Interruptores de Circuito y Aislamiento para Plugins
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\DECISIONS.md` (ADR-025, Líneas 220-244).
- **Qué hace el código:** Si un plugin falla 5 veces consecutivas o provoca 2 fallos de arranque seguidos, el sistema lo desactiva automticamente mediante un circuit breaker y arranca en modo seguro.
- **Qué hacer con ese código en Codync:** Implementar la proteccin de extensiones en `host/src/plugins/circuit_breaker.rs`.

---

## Bloque 4: Adicin de Opciones, Ajustes de Usuario, Automatizaciones y Herramientas

### 41. Sntesis Autnoma de Habilidades tras Flujos Complejos Verificados
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\skill-synthesis.ts` (Líneas 1-80) y ADR-027.
- **Qué hace el código:** Cuando un turno involucra >= 3 herramientas (`MIN_TOOLS_FOR_SYNTHESIS = 3`) o resuelve con xito una correccin del usuario, sintetiza automticamente un archivo de habilidad reutilizable.
- **Qué hacer con ese código en Codync:** Crear el mdulo de auto-sntesis en `host/src/skills/synthesis.rs`.

### 42. Cuarentena y Auto-Retiro de Habilidades Autogeneradas
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\skills\self-made.ts` (`DORMANT_AFTER_DAYS = 30`, `MAX_SELF_MADE = 40`) y `DECISIONS.md` (ADR-027).
- **Qué hace el código:** Guarda las habilidades en `skills/self-made/` separadas de las oficiales; si no se usan en 30 das pasan a estado inactivo (`.disabled`) y mantiene un tope mximo de 40 habilidades autogeneradas activas.
- **Qué hacer con ese código en Codync:** Implementar la poltica de higiene de habilidades en `host/src/skills/hygiene.rs`.

### 43. Paquetes Portables de Bots (`mercury-bot-bundle` v1)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\bundle.ts` (Líneas 1-80).
- **Qué hace el código:** Empaqueta en un nico archivo JSON o zip la configuracin, persona, permisos y habilidades de un bot o flota completa (omitiendo secretos locales), listo para compartir o respaldar.
- **Qué hacer con ese código en Codync:** Implementar exportacin e importacin de paquetes en `host/src/fleet/bundle.rs` y un botn "Exportar bot / equipo" en la interfaz.

### 44. Doctor de Flota para Diagnstico Integral de Bots (`runBotDoctor`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\doctor.ts` (Líneas 1-70).
- **Qué hace el código:** Inspecciona manifiestos rotos, permisos inconsistentes, errores de configuracin y rutinas crnicas desalineadas, emitiendo un reporte con cdigo de salida.
- **Qué hacer con ese código en Codync:** Crear el comando `codync doctor --bots` y el endpoint `/api/doctor` en `host/src/api/doctor.rs`.

### 45. Asistente Interactivo de Incorporacin de Flotas (`fleet-onboarding.ts`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\fleet-onboarding.ts` (Líneas 1-90).
- **Qué hace el código:** Conversa con el usuario sobre sus metas y propone automticamente una tripulacin optimizada (ej. Coordinador, Investigador, Creador) asignando modelos ligeros a roles auxiliares.
- **Qué hacer con ese código en Codync:** Integrar en el flujo de creacin inicial de equipos en `apps/desktop/src/renderer/views/TeamSetupWizard.tsx`.

### 46. Modo Ahorro de Tokens y Batera (`SaverMode`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\core\saver-mode.ts` (Líneas 1-50).
- **Qué hace el código:** Conmutador que cambia a modelos ms rpidos y econmicos, compacta el contexto y reduce pasos innecesarios cuando el porttil est con batera baja o se activa el modo ahorro.
- **Qué hacer con ese código en Codync:** Implementar el ajuste en `host/src/config/saver.rs` con un toggle en la barra de estado de Desktop y Android.

### 47. Presupuesto Diario de Tokens por Bot y Flota con Auto-Pausa
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\bot-manager.ts` (Líneas 70-85) y ADR-020.
- **Qué hace el código:** Permite definir lmites diarios por bot (ej. 5M tokens) y a nivel de flota (ej. 20M tokens). Al superar el 80% emite un aviso; al alcanzar el 100% pausa el trabajo del da sin morir.
- **Qué hacer con ese código en Codync:** Agregar la gestin de presupuestos en `host/src/governance/token_budget.rs`.

### 48. Interruptor de Parada Global de Flota (`stopAll` / `startAll`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\DECISIONS.md` (ADR-022, Líneas 1-6).
- **Qué hace el código:** Comando o botn que detiene o reanuda inmediatamente todos los bots subordinados y lderes de la flota en una sola operacin rpida.
- **Qué hacer con ese código en Codync:** Exponer `/api/fleet/stop_all` y `/api/fleet/start_all` en `host/src/api/fleet.rs` y un botn de emergencia en el panel de control.

### 49. Depurador y Limpiador Selectivo de Dead-Letter Queue (`clearDlq`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\DECISIONS.md` (ADR-022, Lnea 4) y `src\bots\queue.ts`.
- **Qué hace el código:** Permite listar e inspeccionar las tareas que fallaron permanentemente y purgarlas por bot o de forma masiva sin alterar el historial saludable.
- **Qué hacer con ese código en Codync:** Integrar la administracin de la DLQ en `host/src/fleet/dlq.rs`.

### 50. Jerarqua Estandarizada de Carpetas de Entregables (`deliverablesDir`)
- **Origen local:** `C:\Users\Edwin Villeda\Documents\Repositorios\mercury-agent\src\bots\store.ts` (ADR-020, Líneas 193-197).
- **Qué hace el código:** Organiza los archivos finales generados en `Documents/Codync Deliverables/<Nombre del Bot>/`, manteniendo un archivo `README.md` indexador automtico de todos los entregables.
- **Qué hacer con ese código en Codync:** Implementar la generacin y mantenimiento automtico del ndice de entregables en `host/src/fs/deliverables.rs`.

---

## Auditoría contra Codync (2026-10-25)

Estado: **Existe** (ya cubierto, no portar), **Parcial** (hay base; falta lo específico), **Nuevo** (sin equivalente). La ruta propuesta en cada idea es orientativa: Codync no tiene `host/src/{fleet,kanban,governance,plugins,skills,fs,companion}/` ni `lifecycle.rs`; las rutas reales están abajo. Cada cambio de UI toca iOS, Android, escritorio y TUI (CLAUDE.md, "Cross-platform UI changes"); la lógica va en el host.

### Correcciones al contenido

- Idea 26: la fuente es `programming-mode.ts`; `coding-task-detect.ts` solo existe como test.
- Idea 38: el gobernador de mercury limita por presupuesto de caracteres de resultados de herramientas, no por 80% de la ventana.
- Idea 41: el umbral es 3 herramientas, no 4. Idea 42: la política está en `skills/self-made.ts`.
- Idea 4: falta el estado `forgotten`. Idea 30: la fuente principal es `src/capabilities/spotify/`.
- Idea 39: los límites 2/32 KB y 6/128 KB no se encontraron en `retention.ts` (archiva a `_shared/.archive/`); **verificar en `DECISIONS.md` ADR-020 antes de usar**.
- Ideas 36, 23: afirmaciones no verificadas en el código fuente.
- Encabezado: el commit citado no existe. Varias palabras del texto perdieron tildes/eñes (p. ej. "Notificacin"); conviene un repaso ortográfico.

### Estado por idea

| # | Estado | Qué hay en Codync / ruta real si se implementa |
|---|---|---|
| 1-5 | Nuevo | `host/crates/memory` guarda `memory_entries`, tombstones y dreams (`schema.rs`, `dream/`); no hay creencias, rasgos ni bitemporalidad. Extender `schema.rs`/`store.rs` con migración; `evidence.rs` ya define "palabras del usuario". |
| 6 | Parcial | `chat/instruction_history.rs` y `chat/memory/api.rs` (olvidar). El aviso con "deshacer" iría como notice de chat (`chat/handoff.rs` es el patrón), no en `agent/delivery.rs` (no existe). |
| 7 | Parcial | `chat/team.rs` y `chat/team/requests.rs` (delegación y grafo de espera). Faltan tareas tipadas con criterios de aceptación. |
| 8 | Parcial | `team/requests.rs` gestiona admisión; falta despertar único tras N tareas. |
| 9, 10 | Nuevo | Sin digest ni pipelines. Las rutinas (`routines/`) son la base natural de pipelines. |
| 11 | Implementado (sin compilar): `agent/bot/outcome.rs` + `watchdog.rs` | `agent/bot/stall.rs`, `watchdog.rs` y `turn.rs` detectan bucles; no clasifican resultado ni "dijo que lo hizo sin hacer". |
| 12 | Implementado (sin compilar): `routines/idle.rs`, pausa tras 3 runs sin trabajo | `routines/retry.rs` y `agent/coordinator.rs` (tope de reintentos); falta pausa por racha sin resultado. |
| 13 | No aplica: los agentes escriben sus archivos directamente; el host no los intermedia | Sin locks por ruta. |
| 14, 15 | Parcial | `service/mod.rs` ya tolera lock files viejos tras un crash; `store/inflight.rs` guarda trabajo en vuelo. Falta bandera de caída y reanudación explícita. |
| 16-19 | Nuevo | No hay Kanban en ningún cliente (es un feature completo ×4 clientes; ver `docs/plans`). |
| 20 | Nuevo | Sin grafo de memoria. |
| 21 | Parcial | `remote/push.rs` (`AlertKind`), `slack/`; no hay botones-comando. No existe `host/src/channels/`. |
| 22 | Parcial | `chat/files.rs`, `chat/uploads.rs`, `agent/bot/files.rs` ya mueven archivos; revisar entrega al teléfono. |
| 23 | Por verificar | `stall.rs` y eventos de turno ya reportan progreso. |
| 24 | Parcial | `tui/appearance.rs` ya tiene Light/Dark; falta autodetección. |
| 25 | Parcial | iOS tiene `apps/ios/WhatsNew.md`; no hay equivalente en escritorio/Android. |
| 26, 27 | Nuevo | `agent/config.rs` solo trata Fast mode; los modos viven en el agente ACP. Bajo valor/riesgo de heurísticas. |
| 28 | Parcial | `background.rs`, `agent/term.rs`, `hub/terminal.rs`. |
| 29 | Parcial | `remote/push.rs` ya limita por `(bot, AlertKind)` (`throttled`) y se calla con la app conectada. |
| 30 | Nuevo | Fuera de alcance del producto; descartar. |
| 31 | Existe | `screen/` y `computer/` ya implementan helper, input y captura; no importar el companion de mercury. |
| 32 | Nuevo | Sin parada por movimiento físico del ratón (`screen/computer.rs`, `cua.rs`). Candidato de seguridad. |
| 33 | Existe | `screen/protected_entry.rs`, `protected_browser/`, `type_login` (`docs/features/computer-use.md`). |
| 34 | Existe | `voice.rs` + `voice/endpoint.rs`: OpenAI Realtime y Gemini Live con claves del usuario. Descartar. |
| 35 | Parcial | `apps/desktop/src/main/island.ts` ya usa atajos globales; falta push-to-talk con sonidos. |
| 36 | Existe en el host: `screen/snapshots.rs` guarda solo IDs de captura, nunca imágenes | `screen/snapshots.rs`: comprobar si persisten capturas. |
| 37 | Implementado (sin compilar): aviso de herramienta colgada (30 min) en `agent/bot/watchdog.rs`; no la cancela | `stall.rs` y `watchdog.rs` cortan bucles; no hay tiempo máximo por herramienta colgada. |
| 38 | Existe | `agent/bot/compaction.rs` + `codync_mem::compaction` + `chat/handoff.rs`. Descartar. |
| 39 | No aplica: Codync no tiene carpeta `_shared/` entre bots | `chat/memory/budget.rs` limita `MEMORY.md`, no carpetas compartidas. Límites por verificar (ver arriba). |
| 40 | Parcial | Aquí "plugins" son conectores MCP (`market/`); falta circuit breaker por conector. |
| 41 | Parcial | `chat/skills.rs`: el bot guarda skills con `save_skill`; falta la síntesis automática. |
| 42 | Nuevo | Sin higiene/caducidad de skills. |
| 43 | Implementado (sin compilar): `bundle.rs`, API `exportBot`/`importBots` (sin UI aún) | `market/` instala, no exporta bots/equipos. |
| 44 | Implementado (sin compilar): `doctor.rs`, `codync-host doctor`, API `doctor` | No existe `doctor` (`host/src/cli/`). |
| 45 | Nuevo | Sin asistente de equipos. |
| 46 | Nuevo | Sin modo ahorro. `usage.rs` da base de datos de uso. |
| 47 | Implementado (sin compilar): `budget.rs`, `agent/bot/spend.rs`, API `tokenBudgets`/`setTokenBudget` (sin UI aún) | `usage.rs` mide uso local; faltan topes diarios y auto-pausa. |
| 48 | Implementado y probado en host, escritorio y Android (`hub/stop.rs`) | Sin "parar todo". Pequeño y de alto valor: `api/bots.rs` + botón en los 4 clientes. |
| 49 | Implementado (sin compilar): `routines/failed.rs`, API `clearFailedRuns` (sin UI aún) | No hay DLQ; el equivalente son las ejecuciones fallidas de `routines/`. |
| 50 | Nuevo | Sin carpeta estándar de entregables. |

### Candidatos recomendados para implementar primero

1. **48 Parar/reanudar todo**: pequeño, host + 4 clientes.
2. **44 `codync-host doctor`**: solo host/CLI, sin UI.
3. **11 Veredicto de turno sin resultado + 12 pausa de rutinas**: extienden `stall.rs`/`routines/`.
4. **32 Parada por movimiento del ratón**: seguridad de computer use, un solo módulo de host.
5. **47 Presupuesto diario de tokens**: base en `usage.rs`.

### Avance de implementación (2026-10-25)

Alcance acordado: host, Android y escritorio Windows. iOS y la TUI no se han tocado.

- **Compilado y probado:** 48 (prueba del host, `fmt`, `clippy -D warnings`, typecheck y `npm test` del escritorio, compilación de Android).
- **Escrito sin compilar ni probar:** 44, 11, 12, 47, 49, 43 y 37. Necesitan `cargo test -j 2 --bin codync-host` (filtros `doctor`, `outcome`, `idle`, `budget`, `failed`, `bundle`, `watchdog`) y `clippy` antes de darse por buenas. 47, 49 y 43 aún no tienen UI en ningún cliente.
- **No aplican:** 13 y 39 (ver tabla). La 32 queda sin hacer: el ratón lo maneja un driver externo y no se puede conectar con seguridad sin probarlo en Windows.
