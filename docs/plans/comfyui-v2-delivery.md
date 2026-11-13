# ComfyUI: plantillas, modelos y resultados (entrega V2)

Este encargo se montó sobre lo que ya existía (runner, ComfyUI empaquetado, modelos en `userData/comfy`, cola de un trabajo, puente `comfyui` y adjuntos del chat). No se rehizo nada de esa base: se amplió. El trabajo quedó en la rama `deska/PROMPT-V2---SOBRE-LO-QUE-YA-EST----PLANTILLAS--MODELOS-Y-RE--5db46a30` (commits `7cfcb863`, `2e0b8f66`, `3942fe5b`, `4d10e9ed`, `daa5b533`, `19351b2f`, `1a847cea`, `ae8de2fd`, `c715de84`, `b20473bd`, `6837a035`, `7903f2de` y `74d6c240`), empujada a `origin`.

> **Segunda pasada.** Sobre la entrega inicial se añadieron el reproductor de vídeo y audio en el chat, el catálogo de las quince familias del desafío Higgsfield con sus repositorios y licencias, la evaluación del MCP oficial con su arnés de prueba, el servidor falso de `/ws` y el evento `comfy_template_run`.
>
> **Tercera pasada, solo pruebas de código.** Se separaron las funciones puras de los adjuntos (`media.ts`) y del importador para probarlas con `node --test`, sin recompilar la app. La compilación y las pruebas del host de la pasada anterior siguen siendo las que constan abajo.
>
> **Cuarta pasada, sin compilar.** Se importó **Paint a Plate** (Apache-2.0) desde su grafo `edit.json`, con sus tres modelos FLUX 3 **gated**: el preflight y la pestaña Modelos abren su página en vez de descargar sin credenciales. Se añadió `gated` al manifiesto y `openUrl` a las acciones, y `tools/check-comfy-manifests.mjs` valida los manifiestos sin compilar.
>
> **Quinta pasada, sin compilar.** Se importaron las tres familias que faltaban por su dificultad, no por su licencia: **H3 Motion** (AGPL-3.0), **Genobi/Openjutsu** (MIT sobre un pack GPL-3.0) y **OneShot MV** (MIT). Los manifiestos estrenan entradas `video` y `audio`, que el host sube como cualquier imagen; el preflight nombra cada nodo del pack que falte. Las licencias se anotan en el manifiesto y en la ficha, y no bloquean nada: el usuario instala el pack y acepta con ello su licencia.

> **Sexta pasada: las quince familias, sin excepción.** Se importaron las diez que faltaban (Blocksfield, ComfyArt, Henge, Splat Edit Engine, Max Editor, Creativefield, Panorama Stickers, Sonder, Reshoot 4D y H3 Audio Retake) sobre lo que ya existía: mismo runner, mismo ComfyUI empaquetado, misma cola, mismo puente, mismos adjuntos. Tres de ellas solo publican grafo en formato de editor, así que se escribió el conversor `tools/comfy-ui-to-api.mjs` con sus pruebas. Se añadió el tipo de entrada `file` para escenas de splat, packs de cámara y mallas. Se compiló y se corrieron las pruebas unitarias del host y del escritorio; un trabajo real con ComfyUI y GPU sigue sin poder hacerse en esta máquina.

> **Compilado y probado en esta máquina (Windows, sin GPU).** Se ejecutaron `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, las pruebas unitarias del host (`comfy` y `analytics`), `npm run typecheck` y `npm test` del escritorio, y el arnés del MCP oficial. No se abrió Codync Dev ni se ejecutó un trabajo real: no hay ComfyUI instalado, ni GPU, ni SDK de Android, ni macOS para iOS.

## Lo que se construyó

### T1 · Capa de plantillas

- `host/src/tools/comfy/templates.rs`: un manifiesto por carpeta en `userData/comfy/templates/<id>/template.json`, con `id`, `name`, `version`, `description`, `workflow`, `inputs`, `models`, `output`, `estimate`, `tags` y `license`.
- Validación antes de listar o ejecutar: id, versión semver, rangos y opciones, carpetas de modelos, tamaños, hashes y URLs, y que el conjunto de marcadores `{{input}}` del grafo sea exactamente el de campos del formulario (un campo que el grafo no usa se rechaza; un marcador sin campo, también). El grafo es dato, nunca código: solo se sustituyen marcadores y cada `class_type` se comprueba contra `/object_info`.
- Un `{{key}}` que ocupa todo el string conserva su tipo JSON (un número sigue siendo número); dentro de un texto se une como texto.
- Las veinte plantillas base van embebidas desde `host/assets/comfy-templates/` y se copian al arrancar. Una plantilla base que el usuario editó no se pisa: la nueva versión queda al lado como `template.json.new`.

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
- Un grafo de un solo nodo envoltorio que guarda el archivo por su cuenta (`oneshot-mv`) no reporta `outputs`: el trabajo cae entonces a los archivos ingeribles creados desde que se encoló, como mucho los cinco más nuevos (`recent_outputs` en `comfy_job.rs`).

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

### T7 · Higgsfield (las quince familias importadas)

- `host/assets/comfy-templates/curated.json` lista las quince familias con su repositorio real, licencia y tipo de salida, leídos de la página del concurso y de la API de GitHub: Panorama Stickers (MIT), Paint a Plate (Apache-2.0), H3 Motion (AGPL-3.0), OneShot MV (MIT), H3 AudioRetake (MIT), Genobi (MIT), Splat Edit Engine (AGPL-3.0), Sonder (GPL-3.0), Max Editor (Apache-2.0), Blocksfield (MIT), Creativefield (GPL-3.0), ComfyArt (MIT), Reshoot 4D (AGPL-3.0), Freefield (MIT) y Henge (MIT). Las quince llevan `imported: true` y un campo `template` con el manifiesto que las alberga; la ficha abre ese manifiesto y el detalle de la plantilla enlaza de vuelta al repositorio del autor. `tools/check-comfy-manifests.mjs` comprueba que el `template` exista y que `imported` coincida.
- **Angles (Freefield)** está importada como `freefield-angles`: su `workflow.api.json` real, cuatro campos, cinco modelos públicos con tamaño y SHA-256, 31,2 GB y ~24 GB de VRAM. `tools/import-freefield-app.mjs` convierte cualquier `app.json` + `workflow.api.json` de Freefield.
- **Paint a Plate** está importada como `paint-a-plate`: su grafo `edit.json` (30 nodos) con cinco imágenes, pie de foto, sampler y controles de máscara/relight; tres modelos FLUX 3 **gated** (11,2 GB) con tamaño exacto y sin hash público. El preflight y la pestaña Modelos abren la página de Hugging Face en vez de intentar una descarga anónima que daría 401.
- **H3 Motion** está importada como `h3-motion-mask` desde su `workflows/api/h3m_inpaint.api.json` (31 nodos): clip, clip de máscara, qué aparece dentro, fotogramas, tamaño y semilla; cinco modelos públicos (42,1 GB) con tamaño y SHA-256. **Genobi / Openjutsu** está importada como `openjutsu-swap` desde `comfy-build/workflows/openjutsu-swap.api.json` (34 nodos): clip de 5–15 s, foto de personaje, prompt, recorte y toggles de máscara/ancla; siete modelos públicos (46,3 GB). **OneShot MV** está importada como `oneshot-mv` desde `api/OneShotMV_director_api.json`: canción, una línea de mundo, duración, formato, calidad y juego de modelos; once modelos públicos (71,8 GB). Las tres necesitan el pack de nodos de su autor (ComfyUI-vlo, `h3_character_swap` GPL-3.0 con su parche, y el de OneShot MV): el preflight nombra cada nodo que falta y `installNode` lleva a instalarlo.
- Las diez restantes también están importadas, cada una desde el grafo que publica su autor y nunca reimplementada: **Blocksfield** (`blocksfield-art`, 13 nodos del núcleo), **ComfyArt** (`comfyart-edit`, 9 nodos), **Henge** (`henge-motion`, 17 nodos con su LoRA de movimiento y su selector SAM 3.1 / SAM 3D), **Splat Edit Engine** (`spede-replace`, 35 nodos de su pack), **Max Editor** (`maxeditor-retouch`, su plantilla `retocar.api.json`), **H3 Audio Retake** (`h3-audio-retake`, el preset de labios de su web), **Creativefield** (`creativefield-edit`), **Panorama Stickers** (`panorama-cutout`), **Sonder** (`sonder-render`, el render de referencia a vídeo de su editor) y **Reshoot 4D** (`reshoot-4d`, captura 4D y reshoot fotorrealista con LTX 2.3).
- Tres de esos diez grafos solo existen en formato de editor, así que `tools/comfy-ui-to-api.mjs` los convierte: aplana subgrafos, sigue `Reroute`, pliega `PrimitiveNode`, descarta notas y nodos silenciados, y sustituye un nodo puenteado por el cable del mismo tipo que lo alimentaba. Los nombres de cada campo salen de `widgets_values_named` (todos los grafos UI lo traen) y solo caen en `tools/comfy-widget-names.json` cuando el nodo guarda una lista posicional. Los diez manifiestos se regeneran desde los grafos publicados con `node tools/comfy-families/build.mjs` y `build-audioretake.mjs`, sobre el inventario resuelto de `tools/comfy-families/models.json`; los grafos y el inventario quedan en el repositorio.
- Los manifiestos estrenan los tipos de entrada `video` y `audio` (ruta local que el host sube hasta 300 MB, o nombre que el servidor ya tiene, con extensión comprobada) y ahora `file` para escenas de splat, packs de cámara y mallas (`ply`, `npz`, `spz`, `splat`, `obj`, `glb`, `gltf`, `zip`); el formulario de escritorio, la herramienta del bot y la validación del host suben los tres igual. Splat Edit Engine resuelve sus rutas contra el directorio de trabajo del proceso, no contra `input/`, así que su manifiesto las prefija.
- Paso manual que ningún manifiesto puede declarar, y que la ficha dice: Reshoot 4D necesita registro de SMPL-X y su `SMPLX_NEUTRAL.npz`, aparte de ReDirector y de las guías de LTX 2.3; Sonder necesita el dataset de ejemplo del autor o un proyecto propio; H3 Audio Retake se descarga su detector facial en el primer uso; Splat Edit Engine necesita escena, pack de cámara y objeto recortado exportados por su propio estudio.
- Licencias no comerciales anotadas en cada manifiesto: Qwen-Image 2.1 (licencia de investigación), Fibo 2.0 (CC BY-NC 4.0), el tramo de captura de Reshoot 4D (GVHMR, SMPL-X y la LoRA Turbo CC BY-NC-SA) y los términos de Gemma en LTX. Ninguna bloquea la integración.
- `tools/check-comfy-manifests.mjs` valida los veinte manifiestos (marcadores, tipos, carpetas, gated, salida) sin compilar.

## Plantillas incluidas

| Plantilla | Modelos (tamaño) | Licencia | Estimación (sin medir) |
| --- | --- | --- | --- |
| Text to image (`text-to-image`) | Stable Diffusion 1.5 fp16, 2 132 696 762 B | CreativeML OpenRAIL-M | 12 s, 4 GB VRAM; 3 s en RTX 4090 |
| Redraw a picture (`image-to-image`) | El mismo checkpoint | CreativeML OpenRAIL-M | 15 s, 4,5 GB |
| Follow the outline (`canny-to-image`) | SD 1.5 + ControlNet v1.1 canny, 1 445 234 681 B | OpenRAIL-M + Apache-2.0 | 16 s, 5 GB |
| Upscale a picture (`image-upscale`) | Real-ESRGAN x4plus, 67 040 989 B | BSD-3-Clause | 6 s, 2 GB |
| Angles (Freefield) (`freefield-angles`) | Qwen-Image-Edit 2509, su VAE y dos LoRA: 31,2 GB | Apache-2.0 | 60 s, 24 GB; 15 s en RTX 4090 |
| Paint a Plate (`paint-a-plate`) | FLUX 3 Image Klein, su text encoder y VAE: 11,2 GB, gated | FLUX Kommunity License v1.0 | 45 s, 16 GB; 12 s en RTX 4090 |
| H3 Motion (`h3-motion-mask`) | MiniMax H3, su text encoder, dos VAE y una LoRA: 42,1 GB | AGPL-3.0 + MiniMax H3 Community License | 240 s, 24 GB; 90 s en RTX 4090 |
| Swap the person (`openjutsu-swap`) | MiniMax H3, su text encoder, dos VAE, SAM 3.1 y dos LoRA: 46,3 GB | MIT + GPL-3.0 (pack) + SAM License + Apache-2.0 | 300 s, 24 GB; 120 s en RTX 4090 |
| OneShot MV (`oneshot-mv`) | Krea 2, LTX-2.3, Gemma, upscaler y seis LoRA: 71,8 GB | MIT + Krea 2 y LTX-2 Community License | 1800 s, 24 GB; 900 s en RTX 4090 |
| Blocksfield (`blocksfield-art`) | Qwen-Image 2.1 Turbo, su text encoder y VAE: 17,3 GB | MIT + Qwen-Image 2.1 (no comercial) | 12 s, 12 GB |
| ComfyArt (`comfyart-edit`) | Qwen-Image 2.1, su text encoder y VAE: 17,3 GB | MIT + Qwen-Image 2.1 (no comercial) | 18 s, 14 GB |
| Henge (`henge-motion`) | H3, su text encoder, VAE, LoRA de movimiento, SAM 3.1 y SAM 3D: 58,5 GB | MIT + MiniMax H3 Community License + SAM License | 300 s, 24 GB |
| Splat Edit Engine (`spede-replace`) | Qwen-Image 2.1 Turbo, su text encoder y VAE, SAM 3.1, TripoSplat, su VAE, DINOv3 y FLUX.2 VAE: 22,4 GB | AGPL-3.0 + AGPL-3.0 (pack) + no comercial | 240 s, 20 GB |
| Max Editor (`maxeditor-retouch`) | H3 Singularity, su text encoder, dos VAE, SAM 3.1, LoRA Pika y TAEH3: 68,3 GB | Apache-2.0 + MiniMax H3 Community License + SAM License | 300 s, 24 GB |
| H3 Audio Retake (`h3-audio-retake`) | H3 Singularity, su text encoder, dos VAE y LoRA Pika: 55,9 GB | MIT + MiniMax H3 Community License | 300 s, 24 GB |
| Creativefield (`creativefield-edit`) | Fibo 2.0, su VAE, dos text encoders: 23,0 GB | GPL-3.0 + CC BY-NC 4.0 + no comercial | 20 s, 16 GB |
| Panorama Stickers (`panorama-cutout`) | Qwen-Image 2.1, su text encoder y VAE: 17,3 GB | MIT + Qwen-Image 2.1 (no comercial) | 30 s, 16 GB |
| Sonder (`sonder-render`) | H3, su text encoder, tres VAE y dos LoRA turbo: 75,6 GB | GPL-3.0 + MiniMax H3 Community License | 420 s, 24 GB |
| Reshoot 4D (`reshoot-4d`) | LTX 2.3, Gemma, proyección, tres VAE, upscaler, tres LoRA, DA3, BiRefNet, MoGe y Qwen3-VL: 50,7 GB | AGPL-3.0 + LTX-2 Community License + Gemma + CC BY-NC-SA + Apache-2.0/MIT | 3600 s, 24 GB |
| Colour check (`smoke-test`) | Ninguno | Codync | 1 s, 0 GB |

Los tamaños, las URLs y los SHA-256 se comprobaron contra los publicadores (Hugging Face y GitHub) con una petición de rango y la API de Hugging Face. Los tiempos y la VRAM son estimaciones del manifiesto, **no medidas**: la matriz real de T10 no se pudo correr.

## Verificación ejecutada

| Comprobación | Resultado |
| --- | --- |
| `cargo check --all-targets` | Pasa, sin errores. |
| `cargo clippy --all-targets -- -D warnings` | Pasa. |
| `cargo test --bin codync-host comfy` | 44 pasan, 0 fallan, 2 ignoradas (las que piden el ComfyUI real). Incluye la plantilla inválida, el preflight, la descarga, el historial, el `/ws` falso, el clip que solo admite vídeo, el campo de escena que rechaza una imagen y la validación de los **veinte** manifiestos, uno por uno. |
| `cargo fmt --check` | Pasa. |
| `cargo test --bin codync-host analytics` | 32 pasan. |
| Suite completa `cargo test` | No terminó: los binarios de integración (`*_e2e`) se quedan esperando servicios en esta máquina. Las pruebas unitarias del host sí pasan. |
| `npm run typecheck` (escritorio) | Pasa, con el visor 3D. |
| `npm test` (escritorio) | 809 pasan, 4 fallan por el idioma del sistema (esperan inglés y el Windows está en español: `refresh`, dos de `routine`, `bot-exchange`); no tocan ComfyUI. |
| `node --test apps/desktop/src/renderer/views/thread/media.test.ts` | 4 pasan: familias de archivo, MIME, símbolo y tamaños. |
| `node --test tools/import-freefield-app.test.mjs` | 2 pasan: el importador enlaza todos los campos y no muta el grafo de origen. |
| `node --test tools/comfy-ui-to-api.test.mjs` | 8 pasan: cable y widget, widget convertido en enchufe, `Reroute`, `PrimitiveNode`, nodo silenciado y puenteado, nota, tipo sin tabla de nombres y subgrafo con promociones. |
| `node --test apps/desktop/src/renderer/views/marketplace/comfy/media.test.ts` | 2 pasan: cada campo acepta solo su tipo de archivo y el botón lo nombra. |
| `node tools/check-comfy-manifests.mjs` | Los veinte manifiestos pasan (marcadores, tipos, carpetas, gated, salida) y las quince fichas del desafío apuntan a un manifiesto instalado. |
| `node tools/comfy-families/build.mjs` y `build-audioretake.mjs` | Regeneran los diez manifiestos desde los grafos publicados dentro del repositorio, sin cambios en el resultado. |
| Verificación de grafos (`verify-graphs.mjs`) | Sin enlaces colgantes ni marcadores sin campo; queda el falso positivo conocido de `freefield-angles` (`{{move}} {{details}}` en un solo valor). |
| `tools/comfy-mcp-smoke.mjs` | 39 tools reales de `comfy-mcp 0.10.0`; 7/7 operaciones presentes; `server_info` y `search_templates` (761 plantillas) responden. Sin ComfyUI no se ejecutó ningún workflow. |
| Codync Dev abierto / trabajo real | **No**: no hay ComfyUI, GPU ni modelos en esta máquina. |
| Android / iOS | **No compilados**: no hay SDK de Android ni macOS aquí. |

## Lo que quedó fuera

- **T6 · Adopción del MCP oficial**: evaluado y probado en superficie (39 tools, 7/7), no adoptado; no se generó nada con él porque no hay ComfyUI.
- **T7 · Grafos ejecutables de Higgsfield**: las quince familias están importadas, ninguna reimplementada. Las diez de esta pasada necesitan el pack de nodos de su autor; el preflight lo dice nodo a nodo. Reshoot 4D, Sonder y Splat Edit Engine además necesitan un paso manual que no cabe en un manifiesto (SMPL-X con registro, un proyecto del dataset, escena y cámaras exportadas) y la ficha lo dice. Falta ejecutarlas: no hay ComfyUI ni GPU.
- **T10 · Matriz completa**: las pruebas unitarias del host y el `/ws` falso pasan; faltan las cinco pruebas reales con GPU y las mediciones, y la suite completa no terminó en esta máquina porque los binarios `*_e2e` esperan servicios. No hay ComfyUI ni GPU aquí.
- **Móvil**: Android muestra el porcentaje del sampler y ya cancelaba; no tiene galería ni formulario. iOS no tiene sección de ComfyUI. No hay SDK de Android ni macOS en esta máquina para compilar ninguno de los dos.
- **T9 · No bloquear**: `comfy_run_template` espera hasta 5 minutos (como `run_workflow`) para poder adjuntar dentro del turno; con `wait: false` encola y vuelve. El adjunto fuera de turno no existe en el host, así que el resultado se entrega cuando el bot vuelve a consultar.
- **Cola**: sigue habiendo un trabajo a la vez; el historial no se persiste en SQLite (vive en el proceso del host).
- **Versión**: no se pudo ejecutar `tools/bump-version.py`: el `project.pbxproj` del fork está en 2.39.0 y `project.yml` en 2.45.1, y regenerarlo necesita `xcodegen` (macOS), que no existe aquí. Editar el pbxproj a mano está prohibido por las reglas del repositorio.
- **Commits por ficha**: van once commits funcionales (`7cfcb863`, `2e0b8f66`, `3942fe5b`, `4d10e9ed`, `daa5b533`, `19351b2f`, `ae8de2fd`, `c715de84`, `b20473bd`, `6837a035`, `7903f2de`), más los de documentación y el del reporte, no uno por ficha. Los archivos se cruzan entre fichas (`comfy_job.rs`, `api/host.rs`, `lifecycle.rs`, la tarjeta) y separarlos por hunks habría dejado commits que no se sostienen.

## Cómo comprobarlo cuando se pueda compilar

1. `cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
2. `cd apps/desktop && npm run typecheck && npm test`
3. `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug`
4. Con una GPU y ComfyUI descargado: abrir Codync Dev, **Extensions → Apps → ComfyUI → Templates**, ejecutar **Colour check** (no necesita modelo), luego descargar SD 1.5 desde **Text to image**, ejecutarlo y ver el archivo en el chat. Repetir con una de vídeo (**H3 Motion**, si ya está su pack) para comprobar póster y MIME.
5. Medir las veinte plantillas y sustituir las estimaciones del manifiesto por los números medidos; empezar por las que no piden GPU propia (Colour check) y seguir por una de cada familia.
6. Ejecutar de verdad una de las familias reconvertidas (Panorama Stickers) con un grafo convertido, para confirmar que el conversor UI→API y el tipo de entrada `file` funcionan contra un servidor: primero SpEdE con una escena diminuta, luego Sonder con el dataset del autor.
7. Con el MCP oficial instalado (`pip install comfy-mcp "comfy-cli>=1.14.0"`), `node tools/comfy-mcp-smoke.mjs --bin comfy-mcp --comfy-bin <ruta a comfy> --call` y comparar su tabla con `docs/reference/comfy-official-mcp.md`.
