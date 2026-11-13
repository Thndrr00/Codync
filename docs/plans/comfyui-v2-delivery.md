# ComfyUI: plantillas, modelos y resultados (entrega V2)

Este encargo se montó sobre lo que ya existía (runner, ComfyUI empaquetado, modelos en `userData/comfy`, cola de un trabajo, puente `comfyui` y adjuntos del chat). No se rehizo nada de esa base: se amplió. El trabajo quedó en la rama `deska/PROMPT-V2---SOBRE-LO-QUE-YA-EST----PLANTILLAS--MODELOS-Y-RE--5db46a30` (commits `7cfcb863` y `2e0b8f66`), empujada a `origin`.

> **Segunda pasada.** Sobre la entrega inicial se añadieron el reproductor de vídeo y audio en el chat, el catálogo de las quince familias del desafío Higgsfield con sus repositorios y licencias, la evaluación del MCP oficial con su arnés de prueba, el servidor falso de `/ws` y el evento `comfy_template_run`. Sigue sin compilarse ni ejecutarse nada.

> **Sin compilar ni ejecutar.** El encargo terminaba con «NO COMPILES», así que no se ejecutó `cargo build`, `cargo test`, `npm run typecheck`, `npm test`, ni se abrió Codync Dev. Lo único que se pasó fue `rustfmt --check` (parseo y formato) y un validador de los JSON. Por eso ninguna de las pruebas de aceptación que piden ejecutar la app está verificada, y hay que tratar este código como una entrega sin compilar.

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

### T5 · Resultados tipados (casi completo)

- `host/src/tools/comfy/results.rs` clasifica la salida por extensión: imagen, vídeo, audio o malla, con su MIME. Se lee todo el registro `outputs`, no solo `images`, así que un nodo de vídeo o audio no se pierde.
- Un vídeo recibe un póster JPEG con `ffmpeg` si está en `PATH`; sin él se pierde el póster, no el archivo.
- El chat reproduce el vídeo y el audio en la tarjeta del archivo (`Attachments.tsx`, `FileDownloadCard.tsx`, `FileView.tsx`), con un lector de blobs compartido, sin descargarlos primero. Las mallas siguen ofreciendo solo la descarga.
- El adjunto del chat sigue siendo el mismo `send_file`: no se tocó el canal.

### T8 · Galería y formulario (escritorio)

- La tarjeta de ComfyUI pasa a cuatro pestañas: **Templates**, **Models**, **Jobs** y **Settings**.
- La galería lista nombre, descripción, tipo de salida, tiempo estimado, VRAM y número de modelos. Al elegir una plantilla se genera el formulario desde su manifiesto (texto, número con límites, selección, imagen, semilla aleatoria, booleano), con **Run**, **Check again** y las acciones del preflight.
- Una imagen elegida con el diálogo del sistema se sube al host (`comfyUploadImage`, base64) y se convierte en el nombre que conoce ComfyUI, sin archivos temporales.
- **Jobs** muestra nodo, paso, barra, póster y cancelación, y la lista de trabajos recientes.

### T9 · Herramientas del bot

Añadidas junto a las existentes (los nombres viejos se conservan): `comfy_list_templates`, `comfy_describe_template`, `comfy_run_template`, `comfy_job_status`, `comfy_cancel_job`, `comfy_upload_input` y `comfy_result`. Si falta un modelo, la respuesta trae la acción de descarga y no encola.

### T6 · MCP oficial (evaluado, no adoptado)

- `docs/reference/comfy-official-mcp.md` compara el MCP oficial (`Comfy-Org/comfy-mcp`, `comfy-cli` ≥ 1.14, beta, 41 tools) con el puente de Codync. Cubre las siete operaciones genéricas (listar, describir, ejecutar, estado, cancelar, subir, recuperar), pero su licencia **AGPL-3.0-o-comercial**, su runtime Python + `comfy-cli` y su estado beta mantienen el puente propio, que es el único que conoce manifiestos, preflight y el adjunto al chat.
- `tools/comfy-mcp-smoke.mjs` arranca el servidor oficial por stdio, hace el handshake, lista sus tools y comprueba esas siete operaciones; con `--call` consulta `server_info` y `search_templates`. No se ejecutó: el paquete no está instalado aquí.
- Si se adopta, el orden de retirada del puente está escrito: primero `list_models`/`list_nodes`, luego `run_workflow`/`job_status`/`cancel_job`, después `upload_image`; los `comfy_*` de plantillas se quedan.

### T7 · Higgsfield (catalogado, no ejecutable todavía)

- `host/assets/comfy-templates/curated.json` lista las quince familias con su repositorio real, licencia y tipo de salida, leídos de la página del concurso y de la API de GitHub: Panorama Stickers (MIT), Paint a Plate (Apache-2.0), H3 Motion (AGPL-3.0), OneShot MV (MIT), H3 AudioRetake (MIT), Genobi (MIT), Splat Edit Engine (AGPL-3.0), Sonder (GPL-3.0), Max Editor (Apache-2.0), Blocksfield (MIT), Creativefield (GPL-3.0), ComfyArt (MIT), Reshoot 4D (AGPL-3.0), Freefield (MIT) y Henge (MIT).
- La galería las muestra bajo las plantillas instaladas; elegir una abre el repositorio del autor. No se presentan como ejecutables porque el grafo, los nodos y los modelos son del autor.

## Plantillas incluidas

| Plantilla | Modelos (tamaño) | Licencia | Estimación (sin medir) |
| --- | --- | --- | --- |
| Text to image (`text-to-image`) | Stable Diffusion 1.5 fp16, 2 132 696 762 B | CreativeML OpenRAIL-M | 12 s, 4 GB VRAM; 3 s en RTX 4090 |
| Redraw a picture (`image-to-image`) | El mismo checkpoint | CreativeML OpenRAIL-M | 15 s, 4,5 GB |
| Follow the outline (`canny-to-image`) | SD 1.5 + ControlNet v1.1 canny, 1 445 234 681 B | OpenRAIL-M + Apache-2.0 | 16 s, 5 GB |
| Upscale a picture (`image-upscale`) | Real-ESRGAN x4plus, 67 040 989 B | BSD-3-Clause | 6 s, 2 GB |
| Colour check (`smoke-test`) | Ninguno | Codync | 1 s, 0 GB |

Los tamaños y las URLs se comprobaron contra los publicadores (Hugging Face y GitHub) con una petición de rango. Los tiempos y la VRAM son las estimaciones del manifiesto, **no medidas**: la matriz real de T10 no se pudo correr.

## Lo que quedó fuera

- **T6 · Adopción del MCP oficial**: evaluado, no adoptado; el arnés existe pero no se ejecutó. Sin compilar no hay prueba de humo real ni tabla de resultados de llamadas.
- **T7 · Grafos ejecutables de Higgsfield**: las quince están catalogadas con su repositorio y licencia, pero no importadas. Importar una exige copiar su workflow, añadir marcadores, declarar modelos y probarla; no se inventó ninguno.
- **T10 · Matriz completa**: hay pruebas unitarias (manifiesto inválido, campo sin marcador, rangos, sustitución, preflight, clasificación de salidas, nombres seguros, historial, evento de analytics) más un servidor falso HTTP y otro de `/ws`. Faltan las cinco pruebas reales con GPU y las mediciones; no se ejecutó ninguna.
- **T5 · Visores**: vídeo y audio ya se reproducen en el chat; una malla o un splat llegan con su MIME y su descarga, sin visor WebGL.
- **Móvil**: Android muestra el porcentaje del sampler y ya cancelaba; no tiene galería ni formulario. iOS no tiene sección de ComfyUI.
- **T9 · No bloquear**: `comfy_run_template` espera hasta 5 minutos (como `run_workflow`) para poder adjuntar dentro del turno; con `wait: false` encola y vuelve. El adjunto fuera de turno no existe en el host, así que el resultado se entrega cuando el bot vuelve a consultar.
- **Cola**: sigue habiendo un trabajo a la vez; el historial no se persiste en SQLite (vive en el proceso del host).
- **Versión**: no se hizo `tools/bump-version.py`; la rama no va a `main` todavía.
- **Commits por ficha**: van dos commits funcionales (`7cfcb863` y `2e0b8f66`) más el del reporte, no uno por ficha. Los archivos se cruzan entre fichas (`comfy_job.rs`, `api/host.rs`, `lifecycle.rs`, la tarjeta) y separarlos por hunks habría dejado commits que no se sostienen; los cuerpos enumeran las fichas.

## Cómo comprobarlo cuando se pueda compilar

1. `cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
2. `cd apps/desktop && npm run typecheck && npm test`
3. `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug`
4. Con una GPU y ComfyUI descargado: abrir Codync Dev, **Extensions → Apps → ComfyUI → Templates**, ejecutar **Colour check** (no necesita modelo), luego descargar SD 1.5 desde **Text to image**, ejecutarlo y ver el archivo en el chat. Repetir con un vídeo real (fuera de las plantillas base) para comprobar póster y MIME.
5. Medir las cinco plantillas y sustituir las estimaciones del manifiesto por los números medidos.
6. Con el MCP oficial instalado (`pip install comfy-mcp "comfy-cli>=1.14.0"`), `node tools/comfy-mcp-smoke.mjs --bin comfy-mcp --comfy-bin <ruta a comfy> --call` y comparar su tabla con `docs/reference/comfy-official-mcp.md`.
7. Importar una familia de Higgsfield desde su repositorio y validar su manifiesto con `Template::validate` antes de añadirla a la galería. Sus modelos y su VRAM se miden como las demás.
