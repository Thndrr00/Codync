# ComfyUI: plantillas, modelos y resultados (entrega V2)

Este encargo se montó sobre lo que ya existía (runner, ComfyUI empaquetado, modelos en `userData/comfy`, cola de un trabajo, puente `comfyui` y adjuntos del chat). No se rehizo nada de esa base: se amplió. El trabajo quedó en la rama `deska/PROMPT-V2---SOBRE-LO-QUE-YA-EST----PLANTILLAS--MODELOS-Y-RE--5db46a30` (commit `7cfcb863`), empujada a `origin`.

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

### T5 · Resultados tipados (parcial)

- `host/src/tools/comfy/results.rs` clasifica la salida por extensión: imagen, vídeo, audio o malla, con su MIME. Se lee todo el registro `outputs`, no solo `images`, así que un nodo de vídeo o audio no se pierde.
- Un vídeo recibe un póster JPEG con `ffmpeg` si está en `PATH`; sin él se pierde el póster, no el archivo.
- El adjunto del chat sigue siendo el mismo `send_file`: no se tocó el canal, y los clientes ya muestran imágenes y vídeo.

### T8 · Galería y formulario (escritorio)

- La tarjeta de ComfyUI pasa a cuatro pestañas: **Templates**, **Models**, **Jobs** y **Settings**.
- La galería lista nombre, descripción, tipo de salida, tiempo estimado, VRAM y número de modelos. Al elegir una plantilla se genera el formulario desde su manifiesto (texto, número con límites, selección, imagen, semilla aleatoria, booleano), con **Run**, **Check again** y las acciones del preflight.
- Una imagen elegida con el diálogo del sistema se sube al host (`comfyUploadImage`, base64) y se convierte en el nombre que conoce ComfyUI, sin archivos temporales.
- **Jobs** muestra nodo, paso, barra, póster y cancelación, y la lista de trabajos recientes.

### T9 · Herramientas del bot

Añadidas junto a las existentes (los nombres viejos se conservan): `comfy_list_templates`, `comfy_describe_template`, `comfy_run_template`, `comfy_job_status`, `comfy_cancel_job`, `comfy_upload_input` y `comfy_result`. Si falta un modelo, la respuesta trae la acción de descarga y no encola.

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

- **T6 · MCP/CLI/Agent oficiales**: no se evaluaron. Requiere el servidor fijado, red y ejecución real; sin compilar no hay prueba de humo ni tabla de cobertura. El puente propio se queda con sus nombres, y lo que cubra el oficial se podrá retirar cuando exista esa comparación.
- **T7 · Paquete Higgsfield** (Panorama Stickers, Paint a Plate, H3 Motion, OneShot MV, H3 AudioRetake, Genobi, Splat Edit Engine, Sonder, Max Editor, Blocksfield, Creativefield, ComfyArt, Reshoot 4D, Freefield, Henge): no se incluyó. Son grafos de sus autores, con licencias y nodos propios; inventarlos habría sido peor que no tenerlos. El formato de manifiesto y el importador están listos para copiarlos con sus marcadores, modelos y licencia.
- **T10 · Matriz completa**: hay pruebas unitarias (manifiesto inválido, campo sin marcador, rangos, sustitución, preflight, clasificación de salidas, nombres seguros, historial) y un servidor falso HTTP para `/object_info`, `/prompt`, `/history`, `/queue` e `/interrupt`. Falta el servidor falso con `/ws` en la suite y las cinco pruebas reales con GPU; no se ejecutó ninguna.
- **T5 · Visores**: audio y 3D llegan con su MIME correcto, pero no hay reproductor de audio propio ni visor WebGL de malla en el escritorio; el chat ofrece la tarjeta de archivo. Las imágenes y el vídeo sí se muestran.
- **Móvil**: Android muestra el porcentaje del sampler y ya cancelaba; no tiene galería ni formulario. iOS no tiene sección de ComfyUI.
- **T9 · No bloquear**: `comfy_run_template` espera hasta 5 minutos (como `run_workflow`) para poder adjuntar dentro del turno; con `wait: false` encola y vuelve. El adjunto fuera de turno no existe en el host, así que el resultado se entrega cuando el bot vuelve a consultar.
- **Cola**: sigue habiendo un trabajo a la vez; el historial no se persiste en SQLite (vive en el proceso del host).
- **Analytics**: no se añadieron eventos PostHog; el esquema tipado de `analytics/reported.rs` no se tocó para no arriesgar el arranque sin compilar.
- **Versión**: no se hizo `tools/bump-version.py`; la rama no va a `main` todavía.
- **Commits por ficha**: se hizo uno solo. Los archivos se cruzan entre fichas (`comfy_job.rs`, `api/host.rs`, `lifecycle.rs`, la tarjeta), y separarlos por hunks habría dejado commits que no se sostienen; el cuerpo del commit enumera T1–T5, T8 y T9.

## Cómo comprobarlo cuando se pueda compilar

1. `cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
2. `cd apps/desktop && npm run typecheck && npm test`
3. `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug`
4. Con una GPU y ComfyUI descargado: abrir Codync Dev, **Extensions → Apps → ComfyUI → Templates**, ejecutar **Colour check** (no necesita modelo), luego descargar SD 1.5 desde **Text to image**, ejecutarlo y ver el archivo en el chat. Repetir con un vídeo real (fuera de las plantillas base) para comprobar póster y MIME.
5. Medir las cinco plantillas y sustituir las estimaciones del manifiesto por los números medidos.
