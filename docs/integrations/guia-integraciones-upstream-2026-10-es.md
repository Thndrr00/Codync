# Guía y revisión de las diez mejoras integradas en octubre de 2026

> **Seguimiento de implementación (Android y Windows):** las secciones 1–17
> conservan la revisión original de `774da120`. Para las correcciones aplicadas
> después y su validación, consultar el [seguimiento del fork](<../nuevo thndr/seguimiento-android-windows-2026-10-es.md>).
> macOS e iOS quedan fuera de esta intervención por indicación del propietario.

Fecha de revisión: **20 de octubre de 2026**. Repositorio revisado: `C:\Users\Edwin Villeda\Documents\Codync`. Rama local: `main`. Commit examinado: `774da12025d0cfcd0074715f0b9c2dc83232dd5e`.

Esta guía explica qué se incorporó, dónde encontrarlo, cómo utilizarlo, cómo funciona internamente y qué falta para considerar cada mejora aceptada. Incluye problemas comprobados y procedimientos de solución **propuestos, no aplicados**. En esta revisión no se modificó código, configuración, versiones ni el documento original del lote.

## Índice

1. [Alcance, procedencia y lectura de los resultados](#1-alcance-procedencia-y-lectura-de-los-resultados)
2. [Mapa para orientarse en el repositorio](#2-mapa-para-orientarse-en-el-repositorio)
3. [PR 113: búsqueda de modelos](#3-pr-113-búsqueda-de-modelos)
4. [PR 133: SSH en Windows](#4-pr-133-ssh-en-windows)
5. [PR 157: Hermes nativo mediante ACP](#5-pr-157-hermes-nativo-mediante-acp)
6. [PR 156: resultados de rutinas y no leídos](#6-pr-156-resultados-de-rutinas-y-no-leídos)
7. [PR 160: envío no interactivo desde la terminal](#7-pr-160-envío-no-interactivo-desde-la-terminal)
8. [PR 105: cuentas Antigravity aisladas](#8-pr-105-cuentas-antigravity-aisladas)
9. [PR 135: servicios compatibles de voz](#9-pr-135-servicios-compatibles-de-voz)
10. [PR 90: propiedad de procesos y sesiones](#10-pr-90-propiedad-de-procesos-y-sesiones)
11. [PR 40: candidatos ICE incrementales](#11-pr-40-candidatos-ice-incrementales)
12. [PR 132: visor de pantalla por SSH/VNC](#12-pr-132-visor-de-pantalla-por-sshvnc)
13. [Problemas y soluciones guiadas](#13-problemas-y-soluciones-guiadas)
14. [Pruebas realizadas y evidencias anteriores](#14-pruebas-realizadas-y-evidencias-anteriores)
15. [Guía de aceptación de las diez mejoras](#15-guía-de-aceptación-de-las-diez-mejoras)
16. [Cómo localizar funciones, cambios y documentación](#16-cómo-localizar-funciones-cambios-y-documentación)
17. [Orden recomendado para cerrar los pendientes](#17-orden-recomendado-para-cerrar-los-pendientes)

## 1. Alcance, procedencia y lectura de los resultados

### 1.1 Qué se integró realmente

El [registro original del lote](upstream-pr-batch-2026-10.md) describe **diez pull requests del proyecto upstream Codync**, adaptados a este fork. No acredita la incorporación de diez repositorios independientes. Algunos añadidos utilizan proveedores o bibliotecas externas, como Hermes, Antigravity, noVNC y `ws`.

La rama de integración fue `integrate/upstream-agent-desktop-screen`; la base del fork fue `e60772d2`. El historial local contiene:

- `ed6a070b`: integración de mejoras de agentes y pantalla.
- `38b763f9`: actualización declarada a `2.20.0`.
- `1e8f1cb8`: documentación de planes Android y escritorio.
- `774da120`: integración en `main`.

No se importó toda la ascendencia de las ramas upstream. Según el registro, quedaron fuera el reemplazo del almacén de credenciales y el historial ajeno de PR 105, las subidas de versión upstream y los cambios de firma/empaquetado de PR 132. Esos elementos no deben buscarse como funciones entregadas por este lote.

### 1.2 Qué significa cada valoración

- **Implementación localizada:** se encontraron los archivos y el recorrido de código que materializan la mejora. No equivale a comprobarla en una aplicación instalada.
- **Prueba actual aprobada:** comprobación ejecutada durante esta revisión, con su alcance explícito.
- **Validación histórica:** resultado informado en el documento original; no se presenta como una ejecución nueva.
- **Discrepancia confirmada por inspección:** diferencia demostrable entre documentos, manifiestos o comportamiento del código. Cuando no se ejecutó visualmente se indica.
- **Pendiente de aceptación:** necesita dispositivos, servicios, permisos, compilaciones o entornos que no se probaron aquí.
- **Limitación de diseño:** comportamiento explícito del añadido, como la ausencia de Realtime en un servicio de voz personalizado o de captura nativa Windows. No es por sí solo una regresión.

Esta revisión se concentra en el checkout local identificado arriba. No verifica el estado remoto de GitHub Actions, etiquetas publicadas, tiendas, proveedores ni instalaciones del usuario. No se sincronizó con `origin/dev`, porque la tarea pide documentar sin alterar código y el checkout está en `main`; una sincronización con rebase/autostash cambiaría el objeto revisado. Los comandos de reparación de esta guía son instrucciones para una intervención posterior.

## 2. Mapa para orientarse en el repositorio

### 2.1 Quién hace qué

- **`host/`:** daemon Rust. Posee bots, sesiones, procesos ACP, cuentas locales de agentes, persistencia, conteos de no leídos, políticas de voz y coordinación de pantalla.
- **`apps/desktop/`:** Electron, React y TypeScript. El renderer presenta pantallas; preload expone el puente; el proceso principal administra SSH y el visor VNC.
- **`apps/ios/Kit/Sources/CodyncKit/`:** modelos y transporte iOS; `CodyncUI/` contiene vistas y stores.
- **`apps/android/core/`:** modelos y transporte Kotlin; `apps/android/app/` contiene editor, controles, estado y visor.
- **`host/src/tui/`:** interfaz de terminal. Usa el host, permite editar modelos/cuentas y no tiene una superficie de vídeo.
- **`apps/screen-macos/` y `apps/screen-linux/`:** auxiliares nativos de captura y entrada para WebRTC móvil.
- **`cloud/` y `relay/`:** transporte remoto. El lote reutiliza sus mensajes y suscripciones cifradas; no instala por sí mismo un servicio TURN.

El recorrido habitual es **control de la interfaz → store/cliente → método del host → módulo responsable → estado/eventos → interfaz**. SSH/VNC tiene un recorrido específico dentro de Electron, explicado en las secciones 4 y 12.

### 2.2 Diferencias que evitan confusiones

1. La cuenta de acceso a Codync y una cuenta Google aislada de Antigravity son identidades distintas. Añadir la segunda no cambia la primera.
2. SSH conecta la aplicación de escritorio a otro ordenador; el CLI `send` usa una API local autenticada. No es un nuevo cliente SSH.
3. La pantalla móvil usa WebRTC y auxiliares nativos. El nuevo visor de escritorio usa SSH, RFB/VNC, WebSocket y noVNC.
4. Instalar Codync no significa que Hermes, Antigravity, OpenSSH, un servidor RFB o un servicio compatible de voz estén instalados y configurados.
5. Historial visible y contexto interno del agente no son lo mismo: cambiar cuenta o modelo puede abrir otra sesión conservando los mensajes guardados.

### 2.3 Cómo interpretar las rutas

Todas las rutas de código de esta guía son relativas a la raíz del repositorio. Por ejemplo, `host/src/cli/send.rs` corresponde a `C:\Users\Edwin Villeda\Documents\Codync\host\src\cli\send.rs`. Los enlaces principales abren esos archivos desde esta carpeta de documentación.

Los datos de ejecución se resuelven mediante `service::data_dir()` en [host/src/service/mod.rs](../../host/src/service/mod.rs): `CODYNC_HOME` puede sustituir la carpeta del entorno. La carpeta habitual es `~/.codync`, y la variante Dev usa su aislamiento propio. No conviene asumir que un perfil de producción y uno de desarrollo comparten tokens, cuentas o claves de confianza.

## 3. PR 113: búsqueda de modelos

### 3.1 Qué añade y cómo encontrarlo en la aplicación

Reduce el tiempo de selección cuando un agente anuncia muchos modelos. En el editor de un bot, elegir el agente y abrir **Model**. Escritorio ofrece un menú con búsqueda; iOS y Android presentan una hoja buscable. El campo permanece sobre la lista y solo las opciones se desplazan.

El catálogo procede del agente instalado en el ordenador seleccionado. No es una lista universal de modelos garantizados por Codync. Si no existe catálogo o se usa un agente personalizado, se puede escribir un identificador manual o conservar **Default**.

### 3.2 Dónde está y qué responsabilidad tiene cada pieza

- [AgentModelPicker.tsx](../../apps/desktop/src/renderer/views/bots/AgentModelPicker.tsx): construye opciones, aplica búsqueda por ID/nombre/descripción, conserva el modelo elegido y presenta opciones específicas.
- [agent-models.ts](../../apps/desktop/src/renderer/views/bots/agent-models.ts): `useAgentModels` carga `agentModels` con `backend` y `account`; separa el catálogo por contexto, ordenador, agente y cuenta, y descarta respuestas obsoletas.
- `apps/desktop/src/renderer/components/Controls.tsx`: `ChoicePicker` y `DropdownMenu` reciben búsqueda y limpian la consulta al abrir.
- `apps/desktop/src/renderer/components/Overlay.tsx` y `overlay.css`: presentación del menú y separación de buscador/lista desplazable.
- [AgentModelPicker.swift](../../apps/ios/Kit/Sources/CodyncUI/Bots/AgentModelPicker.swift): `modelChoices`, `requestKey`, `load()` y opciones/Fast mode en iOS.
- [SearchableChoicePicker.swift](../../apps/ios/Kit/Sources/CodyncUI/SearchableChoicePicker.swift): hoja reutilizable, buscador fijo, selección y cierre.
- [AgentModelPicker.kt](../../apps/android/app/src/main/kotlin/com/codync/android/AgentModelPicker.kt): `AgentModelPicker` y `SearchableModelPicker`; búsqueda local dentro de una hoja Compose.
- `apps/android/core/src/main/kotlin/com/codync/android/core/AgentSettings.kt`: `AgentModels.options()` y `normalize()`, con la representación común del catálogo.
- `apps/android/app/src/main/kotlin/com/codync/android/BotEditor.kt`: recrea el selector por generación, ordenador, agente y cuenta para aislar cargas.
- [auth.rs](../../host/src/agent/auth.rs): `models()`, `model_catalog()`, `model_config()` y `model_options()` convierten lo anunciado por ACP.
- `host/src/api/host.rs`: entrada `agentModels`.
- `host/src/tui/app/model.rs` y `host/src/tui/app/forms.rs`: edición/ciclo de modelos en terminal; no se añadió una hoja gráfica a la TUI.

### 3.3 Cómo debe funcionar

1. Conectar con el ordenador y abrir el bot.
2. Cargar modelos del agente y de la cuenta seleccionada.
3. Buscar sin distinguir mayúsculas por nombre, ID o descripción.
4. Mantener disponible **Default** y conservar la elección guardada aunque no coincida con el filtro.
5. Al escoger otro modelo, reiniciar las opciones específicas que ya no correspondan.
6. Guardar el bot; el cambio de modelo inicia una sesión nueva y conserva el historial.
7. Mostrar carga, recarga, errores y alternativa manual. Fast mode se mantiene cuando el agente lo anuncia o ya está activado, para poder desactivarlo.

### 3.4 Cómo es realmente

Escritorio e iOS agregan **Default** antes de los resultados filtrados y vuelven a añadir el seleccionado si la búsqueda lo oculta. Android construye bien las opciones iniciales, pero `SearchableModelPicker` filtra **todas** con `choices.filter`. Una consulta que no coincide puede ocultar tanto **Default** como el seleccionado. La elección no se borra y el botón exterior conserva su etiqueta; la diferencia está en la lista visible.

**Valoración:** implementado en los tres clientes gráficos; discrepancia funcional Android confirmada por lectura, pendiente de reproducción visual y corrección. Véase [P-02](#p-02-android-oculta-default-y-el-seleccionado-al-buscar).

**Pruebas relacionadas:** `apps/ios/Kit/Tests/CodyncKitTests/AgentModelsTests.swift`, `apps/android/core/src/test/kotlin/com/codync/android/core/AgentSettingsTest.kt`, pruebas de catálogo dentro de `host/src/agent/auth.rs`. Las pruebas de modelos de escritorio ejecutadas aquí verifican modelos de datos, cuentas y Fast mode; no prueban visualmente el buscador React.

## 4. PR 133: SSH en Windows

### 4.1 Qué añade y cómo utilizarlo

Permite administrar conexiones SSH desde la aplicación Windows mediante OpenSSH nativo. Abrir **Settings → Computers → Over SSH**, añadir o editar el perfil y conectar. El acceso remoto necesita el host Codync del otro ordenador disponible y autenticación SSH no interactiva.

El puerto SSH y el puerto del host Codync son campos distintos. La clave de identidad es una ruta local absoluta; en Windows se admiten rutas nativas, incluidas rutas con espacios. El proceso usa `BatchMode=yes`: no abre un diálogo para pedir contraseñas o frases de paso. Una clave cifrada requiere un mecanismo de autenticación ya operativo, como el agente SSH apropiado.

### 4.2 Archivos y funciones

- [ssh-process.ts](../../apps/desktop/src/main/ssh-process.ts): `sshTools()` resuelve `ssh.exe`, `ssh-keygen.exe` y `ssh-keyscan.exe` bajo `%SystemRoot%\System32\OpenSSH`; `run()` oculta consolas y consume stdout/stderr.
- En ese mismo archivo, `listenerCommand()` y `sshListens()` comprueban que el puerto pertenece al PID del SSH creado; `ownsWindowsTunnel()` y `killOrphanTunnels()` limitan la limpieza a túneles de esta variante.
- [ssh-command.ts](../../apps/desktop/src/main/ssh-command.ts): `knownHostsFiles()`, `hostKeyOptions()`, `resolveArguments()`, `infoArguments()` y `tunnelArguments()` construyen argumentos; se desactivan reenvíos de agente/X11 y se evita compartir un master SSH.
- [ssh-profile.ts](../../apps/desktop/src/main/ssh-profile.ts): `isAbsoluteKeyPath()`, `validate()` y `authRefused()` validan perfiles y clasifican rechazos.
- [ssh.ts](../../apps/desktop/src/main/ssh.ts): coordinación de conexión, adjuntos, puertos, reconexión y desconexión.
- `apps/desktop/src/shared/ssh.ts`, `ssh-startup.ts` y `src/preload/ssh.ts`: contratos y puente con el renderer.
- `apps/desktop/src/renderer/views/settings/ComputersView.tsx` y `ssh-model.ts`: controles y estado de perfiles.
- `apps/desktop/tools/ssh-e2e.mjs`: herramienta existente para comprobaciones SSH; revisar sus requisitos antes de ejecutarla.

### 4.3 Flujo y protección esperados

Resolver perfil → verificar confianza del servidor → consultar identidad/estado del host remoto → abrir túnel local → comprobar propietario del listener → adjuntar ordenador. El token del host no debe enviarse a un proceso ajeno que haya ocupado el puerto local.

Los archivos de confianza combinan el `known_hosts` del usuario y `ssh_known_hosts` de la carpeta Codync de esa variante. Los argumentos mantienen comillas para rutas con espacios y `StrictHostKeyChecking=yes`. Cerrar la conexión debe liberar sus túneles; no debe matar conexiones de producción desde un sandbox o a la inversa.

### 4.4 Estado real y límites

La implementación y las pruebas de rutas Windows están presentes. En esta revisión aprobaron los cuatro casos de `ssh-platform.test.ts`, incluido el caso de propiedad del listener. Las pruebas Unix del comando SSH se omiten en Windows. Esto no acredita una conexión real con un servidor ni la interfaz completa.

**Dependencia operativa:** OpenSSH Client debe existir en la ruta esperada. **Límite importante:** el comando remoto de información usa `sh -lc` y ubicaciones de host Unix/macOS. El soporte de un **cliente Windows** no demuestra soporte general de un **destino Windows** sin ese entorno remoto. No se identifica aquí como una regresión; es una frontera que debe probarse si se necesita ese destino.

iOS, Android y TUI no administran estos perfiles de Electron. No necesitan un editor SSH equivalente para consumir las otras mejoras del host. Diagnóstico y solución en [P-06](#p-06-ssh-no-conecta-o-rechaza-la-identidad).

## 5. PR 157: Hermes nativo mediante ACP

### 5.1 Qué añade

Hermes se registra en el catálogo compartido del host y se inicia mediante su propio comando ACP. Evita tratarlo como un adaptador de registro ajeno. Todos los clientes que consumen ese catálogo pueden ofrecerlo como agente.

### 5.2 Dónde está

- [backends.rs](../../host/src/agent/backends.rs): entrada con `id: "hermes"`, nombre `Hermes Agent`, binario `hermes`, lanzamiento local ACP y comando de configuración `{bin} acp --setup`.
- En el mismo archivo: rutas conocidas de instalación, resolución de candidatos y pruebas `hermes_uses_native_acp` y `hermes_install_dirs_are_searched`.
- `host/src/agent/auth.rs`: inicialización y consulta de autenticación/modelos.
- `host/src/api/host.rs`: `agentSetup`, `agentAuth`, `agentAuthenticate` y `agentModels`.
- `host/src/agent/bot/launch.rs`: arranque real del agente del bot.

### 5.3 Uso esperado y estado actual

1. Instalar Hermes en el ordenador que ejecuta el host, siguiendo una fuente oficial verificada.
2. Ejecutar allí `hermes acp --setup` y completar proveedor, modelo y autenticación.
3. Volver al editor del bot, seleccionar Hermes y cargar modelos.
4. Enviar una petición sencilla; comprobar respuesta y posterior reanudación según las capacidades anunciadas.

El catálogo incluye instrucciones de instalación Unix/Windows, pero su presencia no ejecuta instaladores ni autentica al usuario. El host busca ubicaciones conocidas, incluidas `%LOCALAPPDATA%/hermes/bin` y una ubicación de instalaciones fuente Windows. Que `hermes` funcione en una terminal no basta si el servicio del host tiene otro entorno de búsqueda.

**Valoración:** registro y lanzamiento nativo localizados; prueba real de instalación/autenticación pendiente según el lote. Si no aparece o falla, comprobar el ejecutable con el mismo usuario del servicio, ejecutar configuración y revisar el error de inicio del host. No cambiar a mano el comando ACP sin entender la versión instalada.

## 6. PR 156: resultados de rutinas y no leídos

### 6.1 Problema que corrige

Una rutina puede producir una respuesta interna en su hilo y publicar un resultado al usuario. Contar ambos como novedades duplica el indicador. La mejora identifica el origen de la respuesta para contar el resultado una sola vez y mantener como no leídas las respuestas normales posteriores.

### 6.2 Dónde está y cómo funciona

- [turn.rs](../../host/src/agent/bot/turn.rs): al cerrar el turno marca la entrada final y añade `routineRunId` asociado a la ejecución activa.
- [bots.rs](../../host/src/store/bots.rs): `unread()` consulta SQLite y distingue respuestas internas, resultado publicado y seguimiento ordinario. También conserva tratamiento para registros antiguos sin la marca.
- En ese mismo archivo: `unread_roots()` y pruebas de no leídos/hilos. `routineRunId: null` no se confunde automáticamente con una marca ausente de un registro antiguo.
- [routines_e2e.rs](../../host/tests/routines_e2e.rs): comprueba conteo, lectura y una respuesta posterior normal dentro del hilo.
- [routines.md](../features/routines.md): referencia de producto y comportamiento de rutinas.

### 6.3 Cómo se encuentra y se comprueba

Usar una rutina de un bot, dejar que termine y observar el contador del bot. Leer el resultado debe limpiarlo según el ámbito de lectura. Abrir el hilo y enviar un seguimiento normal debe generar una nueva respuesta contabilizable: la deduplicación no debe ocultar toda la conversación de ese hilo.

**Estado:** corrección central en el host, con regresiones localizadas e incluidas en la validación histórica. Escritorio, iOS, Android y TUI consumen el contador del host; no se agregaron cuatro algoritmos diferentes. Aquí no se disparó una rutina real ni se verificaron sus indicadores visuales.

Si aparece un número duplicado, revisar primero que la aplicación utiliza el host nuevo y que el caso corresponde a un único resultado. No editar directamente SQLite ni forzar a cero sus marcas: eliminaría evidencia y puede desincronizar las lecturas.

## 7. PR 160: envío no interactivo desde la terminal

### 7.1 Qué añade y dónde encontrarlo

Agrega `codync-host send` para scripts, automatizaciones y terminales. Utiliza la API autenticada del host local sin apropiarse de la ejecución del bot ni detenerla al terminar la espera.

- [send.rs](../../host/src/cli/send.rs): `Args`, `run()`, `execute()`, `read_message()`, `resolve_bot()` y `Report`.
- [wait.rs](../../host/src/cli/send/wait.rs): `reply()`, `remaining()`, `finish()`, `history()` y límites de espera.
- [api.rs](../../host/src/cli/api.rs): `Api::new()`, `send()`, `events()` y `redact()`; lectura del token local, API, SSE y reintento con el mismo nonce.
- `host/src/cli/events.rs`: `BotFeed` y seguimiento del bot, turno, estado y aprobaciones.
- `host/src/cli/mod.rs` y `host/src/main.rs`: registro y despacho del subcomando.
- [send_cli_e2e.rs](../../host/tests/send_cli_e2e.rs), su carpeta `send_cli_e2e/` y `fixtures/send_cli_agent.py`: cobertura de integración con agente de prueba.

### 7.2 Ejemplos de uso en PowerShell

Requieren un binario instalado o compilado que incluya el subcomando y un host de la misma variante ya en ejecución. Sustituir `mi-bot` por un ID o nombre existente. Los ejemplos de envío sí generan trabajo; no se ejecutaron durante esta revisión.

```powershell
codync-host send --help
codync-host send --bot "mi-bot" "Resume el estado del proyecto"
codync-host send --bot "mi-bot" --wait --json "Resume el estado del proyecto"
codync-host send --bot "mi-bot" --file ".\peticion.txt" --wait --json
"Explica los resultados" | codync-host send --bot "mi-bot" --file - --wait
codync-host send --bot "mi-bot" --thread "ID-DE-ENTRADA-RAIZ" --wait "Continúa este hilo"
codync-host send --bot "mi-bot" --wait --timeout 120 "Analiza el fallo"
```

`--file` exige texto UTF-8; `--file -` lee stdin. No se combina archivo con texto posicional. El nombre se compara exactamente sin distinguir mayúsculas; si hay dos bots con el mismo nombre, usar el ID. `--port` selecciona el puerto del host, cuyo valor predeterminado depende del entorno de compilación; no resuelve por sí solo una carpeta de token equivocada.

### 7.3 Qué entrega

Sin `--wait`, imprime `seq=<n> turn=<n>` tras aceptar el envío. Con `--wait`, espera la respuesta final y que el turno se estabilice. No debe devolver como resultado una respuesta anterior ni dar por acabado un turno que todavía trabaja.

Con `--json`, el informe contiene `botId`, `entrySeq`, `turn`, `status`, `notices`, `elapsedSeconds`, `waitedForApprovalSeconds` y, cuando existen, `stopReason` y `reply`. El estado inicial aceptado se serializa como `completed` incluso sin esperar: **en ese modo significa aceptación, no trabajo terminado**.

Los mensajes de diagnóstico/aprobación van a stderr; el objeto JSON se emite tras aceptar el mensaje. Un error anterior a la aceptación puede no producir JSON. El consumidor debe revisar el código de salida antes de intentar interpretar una respuesta como éxito.

### 7.4 Códigos de salida y espera

- `0`: completado, o aceptado cuando no se pidió esperar.
- `1`: detenido, rechazado, límite del agente o fallo de ejecución.
- `2`: argumentos inválidos, bot inexistente o nombre ambiguo.
- `3`: host no disponible o petición API rechazada; también puede ocurrir después de aceptar el envío.
- `4`: venció el plazo; **el bot sigue trabajando**.

El plazo por defecto es 1800 segundos y excluye el tiempo pendiente de aprobación. Por ello, `--timeout 120` no garantiza que el proceso CLI termine en 120 segundos de reloj. Las aprobaciones se atienden desde una interfaz autorizada; `send` no las aprueba automáticamente. Para un presupuesto externo estricto, la automatización debe controlar su propio plazo y reconciliar el estado del bot antes de reenviar.

`Api::send()` reintenta determinadas pérdidas de respuesta con el mismo `clientNonce`. Esto evita duplicación dentro de ese intento lógico, pero ejecutar después otro comando independiente genera un nonce nuevo. Ante timeout o error API posterior a aceptación, revisar el chat antes de repetir.

**Valoración:** implementación y 13 casos de integración localizados en `send_cli_e2e.rs`, con cobertura adicional de transporte. El registro histórico informa aprobación; aquí no hay binario en `host/target/debug/codync-host.exe` y no se ejecutó el flujo real. No necesita botones nuevos en los clientes gráficos.

## 8. PR 105: cuentas Antigravity aisladas

### 8.1 Qué añade para el usuario

Permite asignar distintas cuentas Google de Antigravity a distintos bots del mismo host. Las credenciales permanecen en el ordenador que ejecuta los agentes. Elegir la cuenta predeterminada conserva el perfil normal del ordenador.

En el editor del bot seleccionar **Antigravity** y buscar **Account / Google account**. En escritorio usar **Sign in to another account**; en iOS, **Sign in with another account**; Android ofrece el control equivalente. La autorización se completa en el navegador **del ordenador host**, aunque se haya iniciado desde un teléfono.

### 8.2 Mapa de implementación

- [account.rs](../../host/src/agent/account.rs): `AgentAccount`, `accounts_dir()`, `list()`, `validate()`, `env()`, `authenticate()` y `delete()`.
- Funciones internas del mismo archivo: `checked_id()`, `ordinary()`, `profile_dir()`, `read_account()`, `profile_env()`, `isolate_process_env()`, `configure_personal_auth()`, `personal_auth_method()` y `PendingProfile`.
- `host/src/api/host.rs`: métodos `listAccounts`, `accountAuthenticate`, `deleteAccount` y parámetro `account` de `agentModels`.
- `host/src/store/model.rs` y `store/bots.rs`: configuración/persistencia del bot con selección de cuenta.
- `host/src/agent/auth.rs`: descubrimiento de modelos con entorno de cuenta.
- `host/src/agent/bot/session.rs` y `lifecycle.rs`: entorno del proceso y cambio de sesión al modificar la cuenta.
- [AgentAccountPicker.tsx](../../apps/desktop/src/renderer/views/bots/AgentAccountPicker.tsx), `BotEditorView.tsx`, `drafts.ts` y `store/bot-store.ts`: controles, borrador y llamadas del escritorio.
- [AgentAccountPicker.swift](../../apps/ios/Kit/Sources/CodyncUI/Bots/AgentAccountPicker.swift), `BotEditorView.swift` y `Store/BotStore+AgentAccounts.swift`: presentación y llamadas iOS.
- [AgentAccountPicker.kt](../../apps/android/app/src/main/kotlin/com/codync/android/AgentAccountPicker.kt), `BotEditor.kt` y `AppStoreBots.kt`: equivalente Android.
- `host/src/tui/manage/accounts.rs`, `app/forms.rs`, `view/forms.rs` y `net.rs`: selección y autenticación en la interfaz de terminal.

### 8.3 Qué se guarda y cómo se aísla

`accounts_dir()` utiliza `<directorio de datos del host>/accounts`. Cada perfil tiene ID `acc_<uuid>`; el validador admite también el formato corto de ocho caracteres hexadecimales. `account.json` contiene ID, nombre, backend y fecha. La configuración inicial se escribe en `antigravity-acp/settings.json` con `auth.type = oauth-personal`.

Para el proceso dedicado se inyectan:

- `GEMINI_HOME`: directorio de ese perfil.
- `AGY_ACP_FORCE_FILE_STORAGE=1`: obliga al adaptador a usar almacenamiento por archivos para evitar reutilizar una entrada compartida del llavero macOS.

Se eliminan del proceso variables heredadas o guardadas que podrían cambiar la identidad, entre ellas `GEMINI_API_KEY`, `GOOGLE_API_KEY`, `GOOGLE_APPLICATION_CREDENTIALS`, `GEMINI_CLI_HOME`, opciones Vertex y variables de proyecto/cuota de Google. El perfil predeterminado no recibe esta limpieza dedicada.

Se validan IDs, correspondencia entre metadatos y directorio, backend y estructura del perfil; se rechazan enlaces simbólicos y puntos de reanálisis Windows. Unix crea directorios privados `0700` y JSON `0600`. Estas medidas Unix no son una afirmación de ACL equivalentes verificadas en Windows.

### 8.4 Ciclo esperado

1. Solicitar autenticación desde un cliente autorizado.
2. Crear un perfil provisional y arrancar el adaptador con su entorno.
3. Exigir que anuncie `oauth-personal`; si no lo soporta, informar que debe actualizarse.
4. Completar el consentimiento en el navegador del host.
5. Guardar metadatos solo al terminar con éxito. El guardián provisional intenta limpiar perfiles incompletos y cerrar su proceso.
6. Seleccionar esa cuenta para el bot y recargar modelos en ese contexto.
7. Al cambiar de cuenta, abrir sesión nueva con contexto actualizado; conservar el historial visible.

El host impone un único plazo de 600 segundos. Escritorio y Android esperan 660 000 ms; iOS usa 660 s. El desfase original entre clientes y host ya figura corregido y se corroboró en el código. La lista queda vinculada al ordenador/contexto de edición para no mezclar respuestas tardías.

### 8.5 TUI, borrado y plantillas

En la TUI, situarse en el campo **Account**: flechas izquierda/derecha cambian cuenta, `i` inicia sesión y `r` recarga. La opción vacía usa la cuenta predeterminada. Cambiar cuenta limpia el modelo y pide un catálogo nuevo.

El host expone `deleteAccount {accountId}` y rechaza borrar un perfil usado por un bot no eliminado. Los selectores revisados sirven para seleccionar, autenticar y recargar; no debe suponerse que incluyen un administrador gráfico completo de borrado.

Las plantillas portables excluyen la cuenta local. Revisar `apps/desktop/src/shared/bot-template.ts`, `apps/ios/Kit/Sources/CodyncUI/Bots/BotTemplateView.swift` y `apps/android/core/src/main/kotlin/com/codync/android/core/BotDraft.kt`. Importar una plantilla en otro ordenador exige seleccionar allí la cuenta. No se exportan credenciales ni se reemplaza el vault existente del fork.

**Estado real:** aislamiento, controles y protección localizados. Aquí aprobaron las pruebas de modelos/plantillas de escritorio que conservan la cuenta durante edición y la excluyen al exportar. La autenticación real con dos cuentas continúa sin aceptación documentada. Solución de incidencias en [P-07](#p-07-antigravity-no-autentica-o-parece-usar-la-cuenta-equivocada).

## 9. PR 135: servicios compatibles de voz

### 9.1 Qué añade y dónde se configura

`CODYNC_OPENAI_BASE_URL` permite que las llamadas de transcripción y síntesis del host apunten a un servidor compatible con esos endpoints. Es una variable del **proceso host**, no un campo nuevo del teléfono. Afecta la ruta de voz OpenAI del host; no cambia automáticamente Gemini ni el motor de voz del dispositivo.

- [endpoint.rs](../../host/src/voice/endpoint.rs): `openai_base()`, `parse_openai_base()`, `openai_url()`, `realtime_url()` y `openai_http()`.
- [voice.rs](../../host/src/voice.rs): `set_key()`, `models()`, `session()`, `transcribe()` y `speak()`.
- `apps/desktop/src/renderer/views/settings/VoiceChatSettingsView.tsx` y `views/call/`: configuración y uso de voz en escritorio.
- `apps/ios/Kit/Sources/CodyncUI/Call/VoiceChatSettingsView.swift` y motores de esa carpeta: configuración/llamada iOS.
- `apps/android/app/src/main/kotlin/com/codync/android/VoiceChatSettings.kt`, `AppStoreVoice.kt` y `core/Voice*.kt`: configuración y motores Android.
- [voice-call.md](../features/voice-call.md): referencia de las modalidades de voz.

### 9.2 Funcionamiento y requisitos

El host normaliza una terminación `/v1` para evitar duplicarla y conserva prefijos de proxy. Consulta `/v1/models`, envía transcripción a `/v1/audio/transcriptions` y síntesis a `/v1/audio/speech`. Para base personalizada solicita WAV; en la ruta predeterminada solicita AAC.

Se admite HTTPS o HTTP solo en `localhost`/IP de loopback. Una dirección HTTP como `192.168.1.10` se rechaza, aunque esté en la LAN. También se rechazan credenciales embebidas, query y fragmento. El cliente HTTP no sigue redirecciones, para no reenviar la clave guardada a otro destino.

La clave suministrada se comprueba contra `/v1/models` del servicio configurado. Un servidor local sin autenticación no queda automáticamente soportado dejando la clave vacía: una clave vacía significa quitar la configuración. Además, la clasificación del catálogo utiliza patrones de nombres de modelos; “API compatible” no garantiza que cualquier ID arbitrario aparezca en las listas de voz. Conviene comprobar los IDs concretos de transcripción y síntesis que el servicio anuncia.

### 9.3 Ejemplo controlado para una prueba futura

Tras detener la variante del host que se vaya a sustituir, un arranque manual desde PowerShell podría usar:

```powershell
$env:CODYNC_OPENAI_BASE_URL = 'http://127.0.0.1:8890/v1'
codync-host serve
```

Esto requiere que el servicio compatible ya esté escuchando. La variable solo se hereda en ese proceso y sus hijos; no reconfigura un servicio Windows que ya estaba ejecutándose. Para una instalación persistente, revisar cómo arranca esa variante y configurar su entorno real. No guardar claves en la URL.

### 9.4 Qué debe funcionar y qué se bloquea

Debe funcionar **Speech**, es decir, transcribir una intervención y sintetizar una respuesta mediante el servidor elegido. No se ofrece **Realtime** con base personalizada: el catálogo no anuncia esos modelos y `realtime_url()` devuelve error explícito. No hay un desvío silencioso hacia el proveedor predeterminado con esa misma clave.

**Estado:** política y pruebas unitarias localizadas; el registro histórico informa pruebas del host aprobadas. No se conectó aquí un servicio real ni se comprobó reproducción de audio en clientes. Esta funcionalidad beneficia a los clientes que utilizan esas llamadas del host; la TUI no adquiere captura o reproducción de voz por añadirla. Guía en [P-08](#p-08-la-voz-personalizada-no-funciona).

## 10. PR 90: propiedad de procesos y sesiones

### 10.1 Para qué sirve

Evita que procesos hijos de agentes sigan vivos después de fallos, cancelaciones o abandono de una asignación; también evita crear sesiones nuevas innecesarias después de un fallo parcial al seleccionar modelo. Son mejoras internas: no requieren un botón nuevo en cada cliente.

### 10.2 Dónde está la propiedad de procesos

- [process.rs](../../host/src/agent/process.rs): `Process::new()`, `monitor()`, `begin_stop()` y `kill()`, más `Tree` por plataforma.
- [process_windows.rs](../../host/src/agent/process_windows.rs): `resume()` localiza y reanuda el hilo inicial suspendido.
- `host/src/agent/acp.rs`: creación de procesos ACP y enlace con su propietario.

En Unix el agente se coloca en un grupo de procesos propio; el propietario puede terminar el grupo y procura no liberar prematuramente el PID del líder. En Windows se utiliza un **Job Object** con `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. El hijo se crea suspendido, se asigna al Job y después se reanuda, cerrando la ventana en la que podría crear descendientes fuera de la propiedad prevista.

`monitor()` observa salida del líder y termina descendientes; `kill()` termina el árbol y espera al hijo con un límite de cinco segundos. Estos mecanismos reducen procesos abandonados dentro del árbol administrado; no son una garantía sobre procesos externos que un proveedor gestione por otros medios.

### 10.3 Dónde está la política de sesiones

- [session.rs](../../host/src/agent/bot/session.rs): `ensure_session()`, `session_request()`, `request_with_timeout()`, `set_session()`, `finish_session_model()` y `forget_session()`.
- `host/src/agent/bot/mod.rs`: estado de conexión, asignaciones e instrucciones pendientes.
- `launch.rs`, `lifecycle.rs`, `queue.rs`, `turn.rs`, `compaction.rs` y `fast_mode.rs`: integración con arranque, cola, turnos, contexto y modelo.
- `host/src/chat/context.rs`: instantáneas e instrucciones del contexto.

Una sesión nueva confirma y guarda su ID antes de terminar la configuración de modelo. `pending_models` permite reintentar esa configuración sin asignar otra sesión. Las instrucciones iniciales y de hilo se conservan hasta el primer prompt exitoso. Un hilo puede tener su propio ID y usar `session/fork` si el agente lo admite.

Si una asignación queda incierta, el host distingue agentes reanudables y no reanudables. No debe descartar automáticamente una conversación confirmada de un agente incapaz de reanudar. El reset explícito libera la conexión incierta antes del reemplazo. Las peticiones de sesión tienen un límite de 180 segundos y el host detiene el agente si no responde dentro del plazo.

### 10.4 Estado y diagnóstico

Las regresiones están en `host/tests/agent_allocation_e2e.rs`, `agent_setup_e2e.rs`, `agent_session_policy_e2e.rs` y `agent_fast_mode_lifecycle_e2e.rs`; los agentes simulados están en `host/tests/fixtures/lifecycle_agent.py` y `process_agent.py`. El lote informa aprobación de estas familias en Windows.

**Valoración:** implementación y cobertura de regresión localizadas; no se repitió aquí una prueba real de cierre del árbol en Windows, macOS y Linux. Si una sesión no arranca, distinguir fallo de asignación, fallo de modelo y proceso no reanudable antes de resetear. Un reset puede perder contexto interno aunque el chat permanezca visible. No usar una orden global para matar todos los procesos de agentes de otros bots o variantes.

## 11. PR 40: candidatos ICE incrementales

### 11.1 Qué cambia

La pantalla móvil puede intercambiar candidatos de red WebRTC conforme aparecen, en lugar de esperar siempre a completar el SDP. Esto permite aprovechar candidatos tardíos, incluidos recorridos TURN TCP/TLS. La mejora no elimina permisos, necesidad de conectividad ni configuración de TURN.

SDP describe la sesión multimedia; ICE enumera rutas de conexión; STUN ayuda a descubrir direcciones y TURN retransmite tráfico cuando no hay ruta directa. La señalización coordina el acuerdo; que funcione el chat no prueba que el vídeo pueda pasar.

### 11.2 Archivos y responsabilidades

- [host/src/screen/viewer.rs](../../host/src/screen/viewer.rs): `prepare()` reserva la sesión y negocia `trickle`; `offer()` intercambia SDP con el auxiliar.
- [signaling.rs](../../host/src/screen/signaling.rs): `Candidate`, `Progress`, `Signaling`, `candidates()` y validación/orden/propiedad de la sesión.
- `host/src/screen/helper.rs`, `protocol.rs` y `mod.rs`: conexión local del auxiliar, estado y capacidades.
- `host/src/api/host.rs` y `api/devices.rs`: `screenPrepare`, `screenOffer`, `screenCandidate`, cierre y autorización de pantalla.
- `host/src/remote/channel/subscriptions.rs` y `channel/tests/screen_signaling.rs`: suscripción privada `screenCandidates` sobre canal cifrado.
- `apps/ios/Kit/Sources/CodyncKit/Client/HostClient+ScreenCandidates.swift`, `ChannelTransport+Calls.swift`, `ChannelTransport+Incoming.swift` y `HostTransport.swift`: llamadas y flujo de candidatos iOS.
- `apps/ios/Kit/Sources/CodyncKit/Models/ScreenCandidate.swift`, `ScreenConnection.swift` y `ScreenError.swift`: contrato y errores.
- `apps/ios/Kit/Sources/CodyncUI/Screen/ScreenSession.swift` y `ScreenSignaling.swift`: instalación de respuesta y orden de candidatos nativos.
- `apps/android/core/src/main/kotlin/com/codync/android/core/Screen.kt` y `HostClient.kt`: modelos/transporte Android.
- `apps/android/app/src/main/kotlin/com/codync/android/ScreenSession.kt` y `ScreenSignaling.kt`: negociación, cola, cancelación y ciclo de la conexión.
- `apps/screen-macos/HostLink.swift`, `ScreenHelper.swift`, `Session.swift` y **`SessionCandidates.swift`**: implementación nativa macOS.
- `apps/screen-linux/src/candidates.rs`, `negotiation.rs`, `connection.rs`, `link.rs`, `sessions.rs`, `stream.rs` y `portal.rs`: candidatos, respuesta, captura y cierre Linux.

### 11.3 Secuencia que debe respetarse

1. El móvil solicita `screenPrepare {relay, trickle: true}`.
2. El host comprueba permisos, disponibilidad y capacidad del auxiliar; devuelve ID de sesión y modo negociado.
3. Con trickle, abrir `screenCandidates {session}` y esperar `ready` **antes** de enviar la oferta.
4. Instalar descripciones locales, intercambiar oferta/respuesta sin esperar toda la recolección y continuar reuniendo candidatos.
5. Aplicar la respuesta antes de entregar los candidatos pendientes al extremo correspondiente.
6. Subir/aplicar candidatos de forma serializada, esperando confirmaciones. El marcador `complete` va después de todos los candidatos.
7. Cerrar la tentativa ante pérdida de señalización durante la negociación. Una conexión ya establecida puede desacoplar esa suscripción y continuar con medios/entrada.
8. Recuperar con una sesión nueva, sin repetir una adición incierta de candidatos ni reutilizar una negociación cerrada.

El host limita candidatos a 128 por dirección; cada cadena tiene hasta 4096 bytes, el índice de sección hasta 16 y `sdpMid` hasta 256 bytes. Rechaza formato inválido, saltos de línea, exceso de cola, suscripciones duplicadas y sesiones de otro propietario. El detalle completo está en [remote-screen.md](../features/remote-screen.md).

### 11.4 Compatibilidad y estado

Si no todos anuncian trickle, se mantiene el intercambio con SDP completo. Actualizar solo un teléfono no obliga a que un auxiliar antiguo soporte la mejora. Los auxiliares nuevos anuncian `trickle` en su estado.

No se añadió un auxiliar de captura Windows. Los auxiliares nativos cubiertos son macOS y Linux; escritorio SSH/VNC es una ruta diferente. La TUI no muestra vídeo. Cloud/relay ya transportan las llamadas y suscripciones cifradas y no necesitan cambios de señalización específicos de este lote; TURN sí requiere su configuración operativa previa.

**Pruebas localizadas:** `host/src/screen/signaling/tests.rs`, `host/src/remote/channel/tests/screen_signaling.rs`, `apps/ios/Kit/Tests/CodyncKitTests/ScreenCandidateTests.swift`, `CodyncUITests/ScreenSignalingTests.swift`, `ScreenSignalingLifetimeTests.swift`, Android `ScreenCandidateTest.kt` y `ScreenSignalingTest.kt`, macOS `apps/screen-macos-tests/Tests/HostLinkTests.swift` y Linux `video_tests.rs`.

**Valoración:** protocolo implementado y cobertura localizada; aceptación real entre redes pendiente. Existe además un proyecto Xcode desactualizado que requiere atención antes de compilar el auxiliar; véase [P-01](#p-01-proyecto-xcode-desactualizado-y-disparador-de-release).

## 12. PR 132: visor de pantalla por SSH/VNC

### 12.1 Qué añade y cómo abrirlo

Desde un ordenador conectado **Over SSH**, abrir **Computers → View screen** o el icono de pantalla de **Details → Computer** del bot. Se abre una ventana Electron con noVNC, capaz de enviar clics, arrastres, rueda y teclas mediante RFB.

No instala un servidor VNC ni reutiliza el vídeo WebRTC del teléfono. Que el host remoto responda al chat no implica que exista una pantalla VNC disponible.

### 12.2 Recorrido y archivos

1. `apps/desktop/src/renderer/views/settings/ComputersView.tsx`: `ViewScreenButton` solo aparece si hay un adjunto SSH del ordenador.
2. `apps/desktop/src/renderer/views/thread/DetailsPanel.tsx`: acceso equivalente desde el bot, condicionado también al estado presentado por la sección Computer.
3. [open.ts](../../apps/desktop/src/renderer/views/screen/open.ts): `openComputerScreen()` pide abrir el visor y comunica errores al store.
4. `apps/desktop/src/shared/screen-viewer.ts`, `shared/ipc.ts` y `preload/index.ts`: contratos y puente IPC.
5. [screen-viewer.ts](../../apps/desktop/src/main/screen-viewer.ts): `registerScreenViewer()`, `authorize()`, `rendererLocation()` y `closeComputerScreen()`.
6. [vnc-bridge.ts](../../apps/desktop/src/main/vnc-bridge.ts): `bridgeVnc()` convierte WebSocket local a TCP del túnel mediante `ws`.
7. `apps/desktop/src/main/ssh-command.ts`: `tunnelArguments()` agrega reenvío del puerto remoto `127.0.0.1:5900` a un puerto local reservado.
8. [ScreenWindow.tsx](../../apps/desktop/src/renderer/views/screen/ScreenWindow.tsx): importa noVNC, conecta, escala la imagen y muestra `Connecting`, `Live`, `Disconnected` y `Retry`; `screen.css` define su presentación.
9. `apps/desktop/src/main/index.ts`, `renderer/main.tsx` y `renderer/index.html`: registro, entrada de la página y restricciones del renderer.

### 12.3 Requisito remoto exacto

El prototipo espera **RFB plano sin contraseña en `127.0.0.1:5900`**. SSH protege el trayecto remoto. Un servidor que exija autenticación o negociación TLS no se vuelve compatible solo porque use el puerto 5900. La ventana suministra una contraseña vacía y no presenta un formulario para otra autenticación.

La referencia local incluye una configuración para wayvnc en un compositor wlroots compatible:

```text
address=127.0.0.1
port=5900
enable_auth=false
```

No es una receta universal para cualquier escritorio Linux. La documentación del proyecto excluye GNOME, KDE y Weston de ese ejemplo; otros entornos necesitan un servidor RFB verificado aparte. No exponer ni publicar 5900 en la red o en el host Docker para hacer funcionar este prototipo. Los procesos locales del ordenador remoto también pueden acceder a su loopback, por lo que este diseño presupone un equipo remoto confiable.

### 12.4 Protección local y ciclo de vida

El proceso principal resuelve el puerto desde el adjunto SSH activo; ignora nombres y puertos inventados por el renderer. Solo el frame principal autorizado del chat puede abrir la ventana y solo esa ventana obtiene la URL privada de conexión.

El puente escucha en `127.0.0.1`, genera una capacidad aleatoria de 32 bytes en la ruta, verifica ruta exacta, `Host` y `Origin`, rechaza HTTP ordinario y limita mensajes WebSocket a 1 MiB. Usa el parser mantenido de `ws` y tuberías con contrapresión. La ventana desactiva integración Node, usa aislamiento de contexto/sandbox y bloquea navegación y ventanas ajenas.

Al cerrar la ventana se cierra su puente; al desconectar SSH se cierra el visor asociado. La interfaz contempla un plazo de conexión de 15 segundos y **Retry**. Una reconexión SSH que reserve otro túnel requiere resolver de nuevo el adjunto vigente.

**Valoración:** prototipo implementado con endurecimiento respecto al parser inicial. No hay aceptación real VNC registrada. La prueba actual de `vnc-bridge.test.ts` no pudo cargar por ausencia de `ws`; no demuestra un fallo del parser. Véanse [P-03](#p-03-prueba-vnc-bloqueada-por-dependencias-no-instaladas) y [P-09](#p-09-el-visor-abre-pero-no-muestra-imagen).

## 13. Problemas y soluciones guiadas

### P-01. Proyecto Xcode desactualizado y disparador de release

**Tipo:** inconsistencia confirmada; prioridad alta antes de publicar o dar por aceptado macOS/iOS.

**Evidencia local:** `apps/project.yml`, `host/Cargo.toml` y `apps/desktop/package.json` declaran `2.20.0`, pero `apps/Codync.xcodeproj/project.pbxproj` contiene cuatro asignaciones `MARKETING_VERSION = 2.19.1;`. Además, no hay referencia a `SessionCandidates.swift` en ese proyecto generado, aunque el auxiliar tiene ese archivo nuevo y usa referencias explícitas a fuentes como `Session.swift`.

La ausencia de archivos iOS del paquete Swift dentro del pbxproj no sería por sí sola un defecto: el paquete puede descubrir sus fuentes. La observación relevante es la fuente nueva del auxiliar macOS y la versión generada. No se ejecutó Xcode para reproducir el error de compilación que pudiera resultar.

**Impacto del flujo de publicación:** `.github/workflows/auto-tag.yml` se activa en `main` por cambios en **`apps/Codync.xcodeproj/project.pbxproj`** y extrae la versión de ese archivo. El diff `e60772d2..774da120` no lo incluye. Por tanto, ese mecanismo concreto no recibe el cambio de ruta que espera para este lote y, si leyera el archivo actual, vería `2.19.1`. La frase del registro que atribuye al merge de `2.20.0` el disparo del pipeline necesita esta salvedad. No se puede concluir que exista un release publicado; tampoco se descarta una publicación manual o por otro flujo remoto no inspeccionado.

**Solución propuesta:**

1. En un Mac, coordinar la intervención porque regenerar afecta archivos compartidos.
2. Conservar `apps/project.yml` como fuente de verdad y ejecutar desde la raíz `xcodegen generate --spec apps/project.yml`.
3. Revisar el diff generado: versión `2.20.0`, pertenencia de `SessionCandidates.swift` al target Screen y ausencia de cambios ajenos.
4. No editar a mano `project.pbxproj` ni `CURRENT_PROJECT_VERSION`.
5. Ejecutar las pruebas del paquete y auxiliar macOS, construir e instalar la variante correspondiente con el proceso anterior detenido.
6. Probar el flujo de pantalla en las aplicaciones reales.
7. Después de autorizar la entrega de esa corrección, comprobar el workflow Auto Tag, la etiqueta, los artefactos y los workflows de publicación; no asumir éxito a partir del manifiesto.

**Cierre:** proyecto regenerado/versionado coherentemente, fuentes incluidas, compilaciones/pruebas aprobadas y evidencia de los artefactos si se va a declarar un release.

### P-02. Android oculta Default y el seleccionado al buscar

**Tipo:** discrepancia funcional confirmada por inspección; prioridad media. No se reprodujo en un dispositivo durante esta revisión.

**Debería:** conservar esas opciones aun cuando no coincidan con la consulta, como hacen escritorio e iOS y describe el lote. **Actualmente:** `SearchableModelPicker` filtra toda la lista por coincidencia, sin excepción para la opción predeterminada o seleccionada. No pierde la configuración guardada, pero sí la opción visible dentro de esa búsqueda.

**Reproducción propuesta:**

1. En Android, abrir un bot con catálogo y elegir un modelo.
2. Abrir Model y buscar una cadena que no coincida con ese modelo ni con Default.
3. Observar si ambas filas desaparecen y comparar con escritorio/iOS.

**Solución propuesta para una tarea posterior de código:**

1. Mantener Default fuera del filtro ordinario.
2. Filtrar modelos por ID/nombre/descripción.
3. Añadir el seleccionado si no está presente, conservando su nombre original y sin duplicarlo.
4. Añadir regresiones para consulta sin coincidencias, seleccionado oculto y selección manual fuera del catálogo.
5. Ejecutar pruebas Android y repetir el recorrido visual; comprobar también recarga y cambio de cuenta.

**Alternativa sin modificar código:** borrar la consulta para volver a ver todas las opciones y seleccionar Default u otro modelo. **Cierre:** paridad demostrada por prueba y recorrido visual.

### P-03. Prueba VNC bloqueada por dependencias no instaladas

**Tipo:** preparación del entorno de revisión; no se demuestra una regresión de VNC.

`apps/desktop/node_modules` no existe en este checkout. La ejecución acotada llegó a los casos sin dependencias externas, pero `vnc-bridge.test.ts` terminó con `ERR_MODULE_NOT_FOUND: Cannot find package 'ws'`.

**Solución propuesta:**

1. En `apps/desktop`, ejecutar `npm ci` con la versión de Node admitida por el proyecto.
2. Usar el lockfile existente; no sustituirlo por una actualización oportunista de dependencias.
3. Repetir el comando de pruebas de la sección 14.
4. Ejecutar `npm run typecheck`, `npm test` y `npm run build` si se prepara aceptación del escritorio.
5. Si VNC falla con dependencias presentes, clasificar el fallo nuevo por su aserción; no atribuirlo automáticamente a este bloqueo.

**Cierre:** la prueba carga y concluye con sus aserciones ejecutadas. La sesión real sigue siendo una comprobación adicional.

### P-04. Once fallos históricos de las pruebas del actualizador

**Tipo:** deuda de pruebas preexistente según la evidencia del lote; no reproducida aquí.

El registro informa 11 fallos en fixtures del actualizador, reproducidos también en una copia aislada de la base `e60772d`. Eso justifica no atribuirlos sin más a las diez mejoras, pero **no convierte la suite completa en aprobada**.

**Solución propuesta:** instalar dependencias, guardar salida de `npm test`, identificar los casos concretos de actualización y compararlos contra una copia aislada de la base con el mismo entorno. Corregir fixtures o implementación según el resultado, en una tarea propia, y repetir la suite. No omitir casos ni relajar expectativas para obtener un verde aparente.

**Cierre:** suite completa aprobada o incidencia separada con reproducción, causa y criterio de aceptación. El reporte original no detalla aquí los once nombres, por lo que esta guía no los inventa.

### P-05. Documentación breve de pantalla aún dice non-trickle

**Tipo:** documentación incoherente confirmada.

`CLAUDE.md` conserva en su resumen de arquitectura la expresión `non-trickle SDP`, mientras `docs/reference/agent-architecture.md`, `docs/features/remote-screen.md` y el código describen trickle negociado con fallback de SDP completo.

**Solución propuesta:** en una edición documental posterior, cambiar esa descripción breve para remitir al protocolo negociado y al fallback. Respetar la regla de mantener corto CLAUDE.md y enlazar la referencia extensa. Esta revisión deja el archivo intacto y registra el desfase aquí.

**Cierre:** todas las referencias describen el mismo protocolo; no requiere cambiar el transporte.

### P-06. SSH no conecta o rechaza la identidad

**Tipo:** procedimiento de diagnóstico; no es un fallo real observado con un servidor en esta sesión.

1. Verificar OpenSSH nativo con `Get-Command ssh.exe` y comprobar la ruta que usa `sshTools()`, no solo otro SSH instalado por Git.
2. Revisar alias/hostname, usuario, puerto SSH y puerto Codync remoto por separado.
3. Confirmar que la identidad es un archivo absoluto existente y que la autenticación funciona sin prompts.
4. Si hay rechazo de clave del host, contrastar la huella con el administrador por un canal confiable antes de actualizar confianza; no desactivar `StrictHostKeyChecking`.
5. Si el puerto está ocupado, identificar el propietario y reintentar desde la aplicación. No matar procesos ajenos por nombre.
6. Si SSH conecta pero no encuentra Codync, revisar el entorno remoto y el comando `info`; no asumir compatibilidad de un destino Windows arbitrario con `sh -lc`.
7. Desconectar y confirmar cierre únicamente de los túneles de esa variante.

**Cierre:** conexión, confianza, identidad elegida, reintento, puerto ocupado, desconexión y salida de la aplicación comprobados.

### P-07. Antigravity no autentica o parece usar la cuenta equivocada

**Tipo:** procedimiento preventivo para una dependencia externa todavía sin aceptación real.

1. Verificar que el adaptador instalado anuncia `oauth-personal`; el mensaje del host indica expresamente cuándo no lo hace.
2. Abrir el navegador del ordenador host y completar el flujo dentro de los 600 segundos; un teléfono no sustituye ese consentimiento.
3. Confirmar ordenador/variante activa antes de comparar listas: los IDs de cuenta son locales.
4. Crear dos perfiles desde la aplicación y asignarlos a dos bots; verificar con una petición controlada la identidad/capacidad que corresponda a cada cuenta, sin volcar tokens en logs.
5. Cambiar cuenta, recargar modelos y confirmar que cambia la sesión del agente. Conservar chat no significa reutilizar contexto interno.
6. Si hay sospecha de identidad heredada, revisar la ruta `GEMINI_HOME` y el aislamiento del proceso con información redactada; no copiar credenciales manualmente entre carpetas.
7. Para retirar una cuenta, reasignar primero los bots que la usan y después utilizar una operación autorizada del host. No borrar directorios en uso.

**Cierre:** dos identidades independientes sobreviven a reinicio, descubrimiento de modelos y cambio de bot; el borrado en uso sigue rechazado. Los formatos reales de credenciales los controla el adaptador, no los metadatos `account.json`.

### P-08. La voz personalizada no funciona

**Tipo:** diagnóstico de requisitos y límites explícitos.

1. Comprobar que la variable está en el proceso del host que atiende la aplicación.
2. Revisar URL: HTTPS o HTTP loopback, sin usuario/contraseña, query ni fragmento.
3. Confirmar que la ruta final `/v1/models` responde con el formato esperado sin redirección y que la clave sirve en ese servidor.
4. Verificar los IDs clasificados para transcripción/síntesis; no todos los nombres arbitrarios aparecen en la clasificación actual.
5. Seleccionar **Speech** y probar audio corto y una frase breve; comprobar WAV de respuesta.
6. Si se seleccionó Realtime, cambiar a Speech. Para volver a la ruta predeterminada, retirar la variable del entorno real del servicio y revisar su clave antes de reiniciar; cambiar solo una terminal no afecta un servicio activo.
7. Si el servidor redirige, configurar la URL final correcta en lugar de habilitar seguimiento automático.

**Cierre:** transcripción y reproducción correctas, error explícito en Realtime y prueba de que las credenciales no se envían por redirección.

### P-09. El visor abre pero no muestra imagen

**Tipo:** diagnóstico del prototipo SSH/VNC; no se probó un destino real aquí.

1. Confirmar que existe un adjunto **Over SSH** activo; estar conectado por relay no basta.
2. En el ordenador remoto, comprobar que el servidor RFB está escuchando exactamente en loopback `5900` y tiene una sesión gráfica capturable.
3. Verificar compatibilidad de RFB plano sin contraseña. No desactivar seguridad de un servidor expuesto para ajustarlo al prototipo: debe quedar en loopback.
4. Comprobar que el entorno gráfico admite el servidor elegido; no aplicar la receta wayvnc a GNOME/KDE/Weston por analogía.
5. Esperar el resultado de conexión; si falla, revisar servicio remoto y reintentar. El texto de error VNC es genérico y no identifica por sí solo la causa.
6. Si se reconectó SSH, volver a abrir el visor desde el adjunto actual.
7. Probar teclado/ratón y desconectar SSH; la ventana y el puente deben cerrarse.

**Cierre:** primer fotograma, entrada, reconexión y liberación del puente comprobados, además de rechazos de origen/ruta no autorizados.

### P-10. Pantalla móvil conecta al chat pero no al vídeo

**Tipo:** aceptación de red/permisos pendiente, no diagnóstico ya confirmado.

1. Revisar auxiliar macOS/Linux activo y permisos de captura/entrada en el ordenador.
2. Comprobar la autorización Screen del dispositivo; permiso de control por sí solo no habilita estos métodos.
3. Probar primero en red local y distinguir señalización de transporte multimedia.
4. Entre redes distintas, confirmar cuenta/ordenador reclamado y configuración TURN del entorno.
5. Revisar negociación `trickle`, suscripción lista antes de la oferta y candidatos después de aplicar respuesta. Si un par es antiguo, comprobar fallback completo.
6. Revisar errores de candidatos y pérdida de canal antes de reutilizar un intento: la recuperación correcta reserva otra sesión.
7. Verificar una pareja ICE de tipo relay en el caso que exige TURN. Un chat por Cloudflare o un candidato con token TCP no demuestra que se haya usado TURN TLS.
8. Revocar permiso o apagar acceso y comprobar que cesan medios y entrada.

**Cierre:** LAN, redes separadas, candidatos tardíos, cancelación, fallback, reconexión y revocación probados con dispositivos reales o emuladores adecuados.

## 14. Pruebas realizadas y evidencias anteriores

### 14.1 Comprobaciones de esta revisión

Se inspeccionaron historial, diff del lote, manifiestos, proyecto generado, workflow Auto Tag y los recorridos de los diez añadidos en host/clientes/auxiliares. El árbol empezó sin cambios locales. No se instalaron dependencias, compilaron aplicaciones, iniciaron sesiones de agentes ni enviaron mensajes a bots.

Se ejecutó desde `apps/desktop`:

```powershell
node --test --experimental-strip-types src/shared/ssh-command.test.ts src/shared/ssh-platform.test.ts src/shared/models.test.ts src/shared/bot-template.test.ts src/main/vnc-bridge.test.ts
```

Resultado con Node `v24.15.0`: **33 casos informados, 13 aprobados, 19 omitidos y 1 fallo de carga**. Los omitidos corresponden a casos SSH de otros entornos. El fallo es la ausencia de `ws`, documentada en P-03. Entre los aprobados están rutas Windows, claves absolutas, limpieza por variante, propietario del listener, reenvío VNC, cuenta del bot y plantilla portable. No es un `npm test` completo ni una prueba de las ventanas Electron.

`adb devices` no mostró dispositivos conectados. `node`, `cargo` y `adb` están disponibles; no se encontraron `swift` ni `xcodegen` mediante `Get-Command`. `host/target/debug/codync-host.exe` y `apps/desktop/node_modules` no existen. El inicio automático del daemon de ADB durante la consulta no supone una instalación o prueba de la aplicación.

No se repitieron las suites Rust/Android completas: el trabajo es documental y se preservó el entorno sin preparar compilaciones nuevas. No se ejecutó e2e de aplicación instalada ni automatización visual de escritorio. Por ello no se certifica aceptación nativa de ninguna de las diez funciones con esta revisión.

### 14.2 Validación histórica que sí registra el lote

El documento original informa para Windows el 20 de octubre de 2026:

- Host: formato y Clippy aprobados; 420 pruebas exitosas y una ignorada. La ejecución completa se hizo con dos trabajos y un hilo de pruebas para evitar plazos de arranque por concurrencia.
- Escritorio: typecheck y compilación aprobados; 140 pruebas aprobadas, 19 omitidas y 11 fallos del actualizador reproducidos también en la base.
- Android core: 133 pruebas, sin fallos/errores y una omitida; app: 31 sin fallos/errores. Ensamblado y lint aprobados con 77 advertencias y cero errores.
- Una prueba Android de consumidor lento excedió intermitentemente ocho segundos también en la base; la ejecución final aislada y completa pasó sin ampliar el plazo.
- Binario de host de aquella validación: `2.20.0` y ayuda del subcomando send disponible.
- Revisiones de limpieza/refactor realizadas durante la integración; se corrigieron portabilidad de cuentas/plantillas y reinicio explícito de asignación incierta.

Estos resultados no contradicen que el checkout actual carezca de binarios/dependencias de ejecución: son evidencias de otro momento, no garantía de artefactos presentes hoy.

### 14.3 Pendientes históricos que esta revisión no cierra

Automatización Windows de la aplicación real, instalación/uso Android, pruebas y regeneración Xcode, auxiliares nativos macOS/Linux, OAuth de dos cuentas, instalación de Hermes, servicio compatible de voz, ordenador SSH/VNC y sesión TURN entre redes. El registro original explica que su automatización Windows no pudo conectar al pipe nativo; esa causa es histórica, no una conexión reintentada en esta revisión.

## 15. Guía de aceptación de las diez mejoras

Usar instalaciones nuevas de la variante elegida y detener antes sus procesos anteriores. Mantener intactas otras variantes y datos de otros colaboradores. En Mac usar únicamente `build/dd` para los builds de este checkout, según las reglas del repositorio.

1. **Modelos:** probar búsqueda con catálogo largo, scroll, Default, seleccionado que no coincide, manual, opciones, Fast mode, recarga, error y cambio de cuenta. Repetir en escritorio/iOS/Android; en TUI comprobar entrada/ciclo de modelo.
2. **SSH Windows:** conectar con identidad válida, confirmar confianza, probar rechazo, puerto ocupado, reintento, desconexión y cierre de app; comprobar que no quedan túneles propios ni se interrumpen ajenos.
3. **Hermes:** instalar/configurar en el host, cargar catálogo, crear bot y completar un turno ACP. Registrar versión externa y usuario del servicio.
4. **Rutinas:** generar un resultado, comprobar que cuenta una vez, leerlo, añadir seguimiento en su hilo y comprobar el no leído nuevo.
5. **CLI send:** probar aceptación sin espera, respuesta final, JSON, archivo/stdin UTF-8, hilo, bot ambiguo, límite/error, aprobación y timeout que deja al bot activo. Revisar stdout, stderr y código de salida por separado.
6. **Antigravity:** dos cuentas/dos bots, cambio, modelos, reinicio y rechazo de borrado en uso. Exportar plantilla y comprobar ausencia de cuenta local y credenciales.
7. **Voz:** transcripción/síntesis reales, formato de audio, clave inválida, URL HTTP no loopback, redirección y Realtime bloqueado. Repetir en los clientes que consumen la ruta del host.
8. **Procesos/sesiones:** cierre del padre, fallo de arranque, cancelación de asignación, fallo de modelo y reintento sin duplicar sesión; revisar hilos y políticas reanudable/no reanudable en sistemas nativos.
9. **Trickle ICE:** LAN y redes separadas, candidato tardío, orden, completo al final, pérdida de canal, recuperación, fallback con pares antiguos y revocación.
10. **SSH/VNC:** primer fotograma e interacción, destino incompatible, URL/origen no autorizado, reintento y cierre al desconectar SSH. No sustituir esta prueba por la del visor móvil.

Para cada recorrido registrar: commit del host, versión y variante del cliente, sistema operativo, proveedor/auxiliar, pasos exactos, resultado esperado/obtenido y evidencia sin secretos. Una compilación o una captura de ventana vacía no acredita el flujo completo.

## 16. Cómo localizar funciones, cambios y documentación

### 16.1 Búsquedas por funcionalidad

Desde la raíz, estos comandos son de lectura. `rg` localiza símbolos; no hay que memorizar números de línea que cambian entre versiones.

```powershell
# Catálogo, búsqueda de modelos y opciones
rg -n 'AgentModelPicker|SearchableModelPicker|SearchableChoicePicker|model_catalog|useAgentModels' host/src apps

# Cuentas y entorno aislado
rg -n 'accountAuthenticate|listAccounts|deleteAccount|GEMINI_HOME|AGY_ACP_FORCE_FILE_STORAGE' host/src apps

# Backend Hermes y sus regresiones
rg -n 'hermes|hermes_uses_native_acp|hermes_install_dirs' host/src/agent

# SSH Windows, confianza y propiedad de listeners
rg -n 'sshTools|sshListens|ownsWindowsTunnel|knownHostsFiles|tunnelArguments' apps/desktop/src

# CLI, estado final, entradas y aprobaciones
rg -n 'struct Args|resolve_bot|read_message|waited_for_approval_seconds|fn finish' host/src/cli

# Conteo de rutinas
rg -n 'routineRunId|pub fn unread|internal routine|published result' host/src host/tests/routines_e2e.rs

# Política de voz
rg -n 'CODYNC_OPENAI_BASE_URL|realtime_url|openai_http|pub async fn transcribe|pub async fn speak' host/src

# Propiedad de procesos y asignaciones
rg -n 'allocation_uncertain|pending_models|KILL_ON_JOB_CLOSE|pub\(super\) fn resume' host/src/agent

# Pantalla móvil y candidatos
rg -n 'screenCandidates|screenCandidate|MAX_CANDIDATES|ScreenSignaling|SessionCandidates' host/src apps

# Visor VNC de escritorio
rg -n 'bridgeVnc|registerScreenViewer|openComputerScreen|ViewScreenButton|VNC_HINT' apps/desktop/src
```

Para limitar ruido, añadir `-g '*.rs'`, `-g '*.kt'`, `-g '*.swift'` o `-g '*.tsx'`, o reducir la carpeta. Para abrir un fragmento desde PowerShell: `Get-Content <ruta> | Select-Object -Skip <inicio-cero> -First <cantidad>`.

### 16.2 Inventario exacto del lote

```powershell
git show --no-patch --format=fuller 774da120
git diff --stat e60772d2 774da120
git diff --name-status e60772d2 774da120
git diff e60772d2 774da120 -- host/src/cli
git diff e60772d2 774da120 -- apps/desktop/src/main
git diff e60772d2 774da120 -- apps/android apps/ios/Kit
```

La lista completa incluye código, pruebas, documentación y versiones. No interpretar cada archivo cambiado como una undécima funcionalidad. Los planes incorporados bajo `docs/nuevo thndr/` describen trabajo y análisis; su presencia no acredita que todas sus propuestas estén implementadas por estos diez PR.

Para estudiar una función: localizar su símbolo → leer la llamada del cliente → seguir el método API → revisar el módulo host/main → leer las pruebas → ejecutar un recorrido real. Un nombre de prueba demuestra intención y cobertura escrita; solo su ejecución informa resultado en el entorno concreto.

### 16.3 Referencias locales útiles

- [Documento original del lote y hashes de procedencia](upstream-pr-batch-2026-10.md).
- [Arquitectura de escritorio](../architecture/desktop-app.md) y [estructura de archivos](../architecture/file-structure.md).
- [Arquitectura de agentes](../reference/agent-architecture.md).
- [Pantalla remota y configuración TURN](../features/remote-screen.md).
- [Voz](../features/voice-call.md), [rutinas](../features/routines.md) y [marketplace](../features/marketplace.md).
- [Cuentas y SSH](../guides/accounts-and-ssh.md).
- [Desarrollo y releases](../guides/development.md), [aislamiento Dev](../guides/dev-app.md) y [compatibilidad](../reference/compatibility.md).
- [Validación Cloudflare](../guides/cloudflare-testing.md).
- [Plan de mejoras Android y escritorio](<../nuevo thndr/guia-implementacion-50-mejoras-android-escritorio.md>): material de planificación, no inventario de funciones aceptadas.

## 17. Orden recomendado para cerrar los pendientes

1. Resolver P-01 antes de afirmar que el release automático `2.20.0` se disparó correctamente o que el auxiliar macOS compila con todas las fuentes.
2. Preparar dependencias de escritorio y repetir pruebas; separar P-03 del problema histórico de fixtures P-04.
3. Corregir y aceptar la paridad del buscador Android P-02.
4. Alinear la referencia breve de arquitectura P-05.
5. Ejecutar los diez recorridos de aceptación con proveedores, servidores y dispositivos reales; documentar cada plataforma no disponible sin darla por aprobada.
6. Verificar publicación y artefactos de manera independiente cuando se autorice la entrega.

**Conclusión de esta revisión:** las diez incorporaciones tienen implementación localizable. Se confirmaron dos discrepancias de producto/configuración —buscador Android y proyecto Xcode/release— y una incoherencia documental. Hay un bloqueo actual de pruebas por dependencias, fallos históricos del actualizador y aceptación nativa pendiente. La evidencia disponible no permite afirmar que todo esté listo para publicación o probado de extremo a extremo. Las soluciones quedan documentadas para ejecutarse en una intervención posterior sin haber modificado el código en esta tarea.
