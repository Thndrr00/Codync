# ComfyUI: plantillas, modelos y resultados (entrega V2)

Este encargo se montó sobre lo que ya existía (runner, ComfyUI empaquetado, modelos en `userData/comfy`, cola de un trabajo, puente `comfyui` y adjuntos del chat). No se rehizo nada de esa base: se amplió. El trabajo quedó en la rama `deska/PROMPT-V2---SOBRE-LO-QUE-YA-EST----PLANTILLAS--MODELOS-Y-RE--5db46a30` (commits `7cfcb863`, `2e0b8f66`, `3942fe5b` y el de este reporte), empujada a `origin`.

> **Segunda pasada.** Sobre la entrega inicial se añadieron el reproductor de vídeo y audio en el chat, el catálogo de las quince familias del desafío Higgsfield con sus repositorios y licencias, la evaluación del MCP oficial con su arnés de prueba, el servidor falso de `/ws` y el evento `comfy_template_run`.
>
> **Tercera pasada, solo pruebas de código.** Se separaron las funciones puras de los adjuntos (`media.ts`) y del importador para probarlas con `node --test`, sin recompilar la app. La compilación y las pruebas del host de la pasada anterior siguen siendo las que constan abajo.

> **Compilado y probado en esta máquina (Windows, sin GPU).** Se ejecutaron `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, las pruebas unitarias del host (`comfy` y `analytics`), `npm run typecheck` y `npm test` del escritorio, y el arnés del MCP oficial. No se abrió Codync Dev ni se ejecutó un trabajo real: no hay ComfyUI instalado, ni GPU, ni SDK de Android, ni macOS para iOS.

## Lo que se construyó

### T1 · Capa de plantillas

- `host/src/tools/comfy/templates.rs`: un manifiesto por carpeta en `userData/comfy/templates/<id>/template.json`, con `id`, `name`, `version`, `description`, `workflow`, `inputs`, `models`, `output`, `estimate`, `tags` y `license`.
- Validación antes de listar o ejecutar: id, versión semver, rangos y opciones, carpetas de modelos, tamaños, hashes y URLs, y que el conjunto de marcadores `{{input}}` del grafo sea exactamente el de campos del formulario (un campo que el grafo no usa se rechaza; un marcador sin campo, también). El grafo es dato, nunca código: solo se sustituyen marcadores y cada `class_type` se comprueba contra `/object_info`.
- Un `{{key}}` que ocupa todo el string conserva su tipo JSON (un número sigue siendo número); dentro de un texto se une como texto.
- Las cinco plantillas base van embebidas desde `host/assets/comfy-templates/` y se copian al arrancar. Una plantilla base que el usuario editó no se pisa: la nueva versión queda al lado como `template.json.new`.

### T2 · Preflight

- `host/src/tools/comfy/preflight.rs` comprueba, antes de encolar: servidor arrancado, nodos instalados, modelos presentes y del tamaño declarado, VRAM libre frente a la estimación, y disco para la salida.
- Cada problema trae una acción: `start`, `installNode`, `download` (carpeta, archivo, URL, tamaño y licencia), `lowerResolution` o `freeSpace`. Sin resolverlos no se encola nada. En la galería, el botón **Run** se convierte en **Download model** cuando falta un modelo.
- Sin NVIDIA hay aviso (`useCpu`), no bloqueo.

### T3 · Progreso real y cola con historial

- `host/src/tools/comfy/progress.rs` escucha `/ws`: nodo actual, clase y paso (`12/20`), con porcentaje. El estado del trabajo vive en el host y se publica en `toolStatus` (`progress`, `template`, `createdAt`).
- Un historial de veinte trabajos terminados (`jobs`) queda en el host; el móvil y la tarjeta leen lo mismo aunque se cierre el escritorio.
- Cancelar sigue interrumpiendo (`/interrupt`) o desencolando; parar el turno del bot que espera cancela su trabajo.

### T4 · Gestor de modelos

- `host/src/tools/comfy/models.rs`: lista por carpeta, tamaño, uso por plantilla, licencia, espacio por carpeta y libre en disco.
- Descarga por rangos a `<archivo>.part`, con pausa y reanudación; comprueba tamaño y SHA-256 del manifiesto antes de renombrar. Solo acepta `(carpeta, archivo)` que declare una plantilla, y solo desde la URL de ese manifiesto.
- Borrar pide confirmación si una plantilla lo necesita.

### T5 · Resultados tipados (completo en escritorio)

- `host/src/tools/comfy/results.rs` clasifica la salida por extensión: imagen, vídeo, audio o malla, con su MIME. Se lee todo el registro `outputs`, no solo `images`, así que un nodo de vídeo o audio no se pierde.
- Un vídeo recibe un póster JPEG con `ffmpeg` si está en `PATH`; sin él se pierde el póster, no el archivo.
- El chat reproduce el vídeo y el audio en la tarjeta del archivo (`Attachments.tsx`, `FileDownloadCard.tsx`, `FileView.tsx`), con un lector de blobs compartido, sin descargarlos primero.
- Un `.glb` se dibuja en un visor WebGL propio (`MeshViewer.tsx`, three.js): se encuadra, se ilumina y gira despacio; las demás mallas conservan su tarjeta de descarga.
- El adjunto del chat sigue siendo el mismo `send_file`: no se tocó el canal.

### T8 · Galería y formulario (escritorio)

- La tarjeta de ComfyUI pasa a cuatro pestañas: **Templates**, **Models**, **Jobs** y **Settings**.
- La galería lista nombre, descripción, tipo de salida, tiempo estimado, VRAM y número de modelos. Al elegir una plantilla se genera el formulario desde su manifiesto (texto, número con límites, selección, imagen, semilla aleatoria, booleano), con **Run**, **Check again** y las acciones del preflight.
- Una imagen elegida con el diálogo del sistema se sube al host (`comfyUploadImage`, base64) y se convierte en el nombre que conoce ComfyUI, sin archivos temporales.
- **Jobs** muestra nodo, paso, barra, póster y cancelación, y la lista de trabajos recientes.

### T9 · Herramientas del bot

Añadidas junto a las existentes (los nombres viejos se conservan): `comfy_list_templates`, `comfy_describe_template`, `comfy_run_template`, `comfy_job_status`, `comfy_cancel_job`, `comfy_upload_input` y `comfy_result`. Si falta un modelo, la respuesta trae la acción de descarga y no encola.

### T6 · MCP oficial (evaluado, no adoptado)

- `docs/reference/comfy-official-mcp.md` compara el MCP oficial (`Comfy-Org/comfy-mcp`, `comfy-cli` ≥ 1.14, beta, 39 tools en 0.10.0) con el puente de Codync. Cubre las siete operaciones genéricas (listar, describir, ejecutar, estado, cancelar, subir, recuperar), pero su licencia **AGPL-3.0-o-comercial**, su runtime Python + `comfy-cli` y su estado beta mantienen el puente propio, que es el único que conoce manifiestos, preflight y el adjunto al chat.
- `tools/comfy-mcp-smoke.mjs` se ejecutó contra una venv con `comfy-mcp 0.10.0` y `comfy-cli 1.26.0`: handshake, 39 tools, **7/7** operaciones presentes, `server_info` y `search_templates` (761 plantillas). No había ComfyUI, así que no se ejecutó ningún workflow.
- Si se adopta, el orden de retirada del puente está escrito: primero `list_models`/`list_nodes`, luego `run_workflow`/`job_status`/`cancel_job`, después `upload_image`; los `comfy_*` de plantillas se quedan.

### T7 · Higgsfield (una familia importada, el resto catalogado)

- `host/assets/comfy-templates/curated.json` lista las quince familias con su repositorio real, licencia y tipo de salida, leídos de la página del concurso y de la API de GitHub: Panorama Stickers (MIT), Paint a Plate (Apache-2.0), H3 Motion (AGPL-3.0), OneShot MV (MIT), H3 AudioRetake (MIT), Genobi (MIT), Splat Edit Engine (AGPL-3.0), Sonder (GPL-3.0), Max Editor (Apache-2.0), Blocksfield (MIT), Creativefield (GPL-3.0), ComfyArt (MIT), Reshoot 4D (AGPL-3.0), Freefield (MIT) y Henge (MIT).
- **Angles (Freefield)** está importada como `freefield-angles`: su `workflow.api.json` real, cuatro campos (foto, movimiento, detalle, semilla), cinco modelos con tamaño exacto y SHA-256 de Hugging Face, 31,2 GB y ~24 GB de VRAM. `tools/import-freefield-app.mjs` hace la conversión desde un `app.json` + `workflow.api.json` de Freefield y se puede repetir con sus otras apps.
- La galería muestra el resto bajo las plantillas instaladas; elegir una abre el repositorio del autor. No se presentan como ejecutables porque el grafo, los nodos y los modelos son del autor.

## Plantillas incluidas

| Plantilla | Modelos (tamaño) | Licencia | Estimación (sin medir) |
| --- | --- | --- | --- |
| Text to image (`text-to-image`) | Stable Diffusion 1.5 fp16, 2 132 696 762 B | CreativeML OpenRAIL-M | 12 s, 4 GB VRAM; 3 s en RTX 4090 |
| Redraw a picture (`image-to-image`) | El mismo checkpoint | CreativeML OpenRAIL-M | 15 s, 4,5 GB |
| Follow the outline (`canny-to-image`) | SD 1.5 + ControlNet v1.1 canny, 1 445 234 681 B | OpenRAIL-M + Apache-2.0 | 16 s, 5 GB |
| Upscale a picture (`image-upscale`) | Real-ESRGAN x4plus, 67 040 989 B | BSD-3-Clause | 6 s, 2 GB |
| Angles (Freefield) (`freefield-angles`) | Qwen-Image-Edit 2509, su VAE y dos LoRA: 31,2 GB | Apache-2.0 | 60 s, 24 GB; 15 s en RTX 4090 |
| Colour check (`smoke-test`) | Ninguno | Codync | 1 s, 0 GB |

Los tamaños, las URLs y los SHA-256 se comprobaron contra los publicadores (Hugging Face y GitHub) con una petición de rango y la API de Hugging Face. Los tiempos y la VRAM son estimaciones del manifiesto, **no medidas**: la matriz real de T10 no se pudo correr.

## Verificación ejecutada

| Comprobación | Resultado |
| --- | --- |
| `cargo check --all-targets` | Pasa, sin errores. |
| `cargo clippy --all-targets -- -D warnings` | Pasa. |
| `cargo test --bin codync-host comfy` | 40 pasan, 0 fallan, 2 ignoradas (las que piden el ComfyUI real). Incluye la plantilla inválida, el preflight, la descarga, el historial, el `/ws` falso y el manifiesto de Freefield. |
| `cargo test --bin codync-host analytics` | 32 pasan. |
| Suite completa `cargo test` | No terminó: los binarios de integración (`*_e2e`) se quedan esperando servicios en esta máquina. Las pruebas unitarias del host sí pasan. |
| `npm run typecheck` (escritorio) | Pasa, con el visor 3D. |
| `npm test` (escritorio) | 805 pasan, 4 fallan por el idioma del sistema (esperan inglés y el Windows está en español); no tocan ComfyUI. |
| `node --test apps/desktop/src/renderer/views/thread/media.test.ts` | 4 pasan: familias de archivo, MIME, símbolo y tamaños. |
| `node --test tools/import-freefield-app.test.mjs` | 2 pasan: el importador enlaza todos los campos y no muta el grafo de origen. |
| `tools/comfy-mcp-smoke.mjs` | 39 tools reales de `comfy-mcp 0.10.0`; 7/7 operaciones presentes; `server_info` y `search_templates` (761 plantillas) responden. Sin ComfyUI no se ejecutó ningún workflow. |
| Codync Dev abierto / trabajo real | **No**: no hay ComfyUI, GPU ni modelos en esta máquina. |
| Android / iOS | **No compilados**: no hay SDK de Android ni macOS aquí. |

## Lo que quedó fuera

- **T6 · Adopción del MCP oficial**: evaluado y probado en superficie (39 tools, 7/7), no adoptado; no se generó nada con él porque no hay ComfyUI.
- **T7 · Grafos ejecutables de Higgsfield**: una familia importada (Angles, de Freefield) y las otras catorce catalogadas con su repositorio y licencia. Importar cada una exige su grafo, marcadores, modelos y prueba; no se inventó ninguno. Panorama Stickers no publica un workflow JSON (es una app de nodos), así que no se puede importar como manifiesto sin integrarla.
- **T10 · Matriz completa**: las pruebas unitarias del host y el `/ws` falso pasan; faltan las cinco pruebas reales con GPU y las mediciones, y la suite completa no terminó en esta máquina porque los binarios `*_e2e` esperan servicios. No hay ComfyUI ni GPU aquí.
- **Móvil**: Android muestra el porcentaje del sampler y ya cancelaba; no tiene galería ni formulario. iOS no tiene sección de ComfyUI. No hay SDK de Android ni macOS en esta máquina para compilar ninguno de los dos.
- **T9 · No bloquear**: `comfy_run_template` espera hasta 5 minutos (como `run_workflow`) para poder adjuntar dentro del turno; con `wait: false` encola y vuelve. El adjunto fuera de turno no existe en el host, así que el resultado se entrega cuando el bot vuelve a consultar.
- **Cola**: sigue habiendo un trabajo a la vez; el historial no se persiste en SQLite (vive en el proceso del host).
- **Versión**: no se pudo ejecutar `tools/bump-version.py`: el `project.pbxproj` del fork está en 2.39.0 y `project.yml` en 2.45.1, y regenerarlo necesita `xcodegen` (macOS), que no existe aquí. Editar el pbxproj a mano está prohibido por las reglas del repositorio.
- **Commits por ficha**: van tres commits funcionales (`7cfcb863`, `2e0b8f66`, `3942fe5b`) más el del reporte, no uno por ficha. Los archivos se cruzan entre fichas (`comfy_job.rs`, `api/host.rs`, `lifecycle.rs`, la tarjeta) y separarlos por hunks habría dejado commits que no se sostienen.

## Cómo comprobarlo cuando se pueda compilar

1. `cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
2. `cd apps/desktop && npm run typecheck && npm test`
3. `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug`
4. Con una GPU y ComfyUI descargado: abrir Codync Dev, **Extensions → Apps → ComfyUI → Templates**, ejecutar **Colour check** (no necesita modelo), luego descargar SD 1.5 desde **Text to image**, ejecutarlo y ver el archivo en el chat. Repetir con un vídeo real (fuera de las plantillas base) para comprobar póster y MIME.
5. Medir las cinco plantillas y sustituir las estimaciones del manifiesto por los números medidos.
6. Con el MCP oficial instalado (`pip install comfy-mcp "comfy-cli>=1.14.0"`), `node tools/comfy-mcp-smoke.mjs --bin comfy-mcp --comfy-bin <ruta a comfy> --call` y comparar su tabla con `docs/reference/comfy-official-mcp.md`.
7. Importar una familia de Higgsfield desde su repositorio y validar su manifiesto con `Template::validate` antes de añadirla a la galería. Sus modelos y su VRAM se miden como las demás.
