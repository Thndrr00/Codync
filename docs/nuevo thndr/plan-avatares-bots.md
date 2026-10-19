# Plan de viabilidad: avatares de bots

- **Estado:** borrador de trabajo
- **Fecha:** 2026-10-19
- **Ámbito:** iPhone, Android, desktop (macOS/Linux/Windows), terminal, widgets, Live Activity, notificaciones y notch island.
- **Objetivo:** decidir si conviene mejorar el avatar actual o cambiar el concepto, con coste, riesgos y fases concretas.

---

## 1. Resumen ejecutivo

Hoy cada bot tiene un **personaje de puntos (halftone)** con una de 8 siluetas y uno de 11 colores. Es una identidad visual, no un indicador de estado: el estado lo comunican el badge y el `ThinkingOrb`.

Cambiar el concepto es viable, pero **no es cambiar un componente**: hay cuatro renderers nativos, seis superficies derivadas y unas 47 referencias en el código. La opción más rentable es evolucionar el lenguaje de puntos (por ejemplo, una **constelación determinista** por bot) en lugar de sustituirlo por fotos, que añade almacenamiento, caché y límites en widgets y notificaciones.

Recomendación:

1. **Fase 0:** definir el problema real y prototipar 2–3 conceptos (1–2 días).
2. **Fase 1:** pulir el concepto actual (3–6 días).
3. **Fase 2:** si hace falta diferenciación, evolucionar a **constelación** o **criatura 2.0** con un campo aditivo y fallback (3–5 semanas).
4. **Fase 3:** añadir **emoji** como personalización barata (1–2 semanas).
5. **Fase 4 (opcional):** fotos subidas, solo si es un requisito duro (4–8 semanas).

---

## 2. Qué es hoy

### 2.1 Concepto

- **Silueta:** `blob`, `pebble`, `squircle`, `tablet`, `wedge`, `hex`, `cloud`, `teardrop`.
- **Color:** 11 ids (`black`, `brown`, `red`, `orange`, `yellow`, `green`, `cyan`, `blue`, `violet`, `magenta`, `gray`).
- **Dibujo:** rejilla de puntos (7×7, 9×9 o 13×13 según tamaño), sombreado como bola iluminada, ojos huecos y color solo en las zonas iluminadas.
- **Movimiento:** la luz gira y los ojos miran durante `working`; parpadeo ocasional; ripple en `needsInput`; `Reduce Motion` lo congela.
- **Grupos:** clúster de 2–4 miembros; a partir de 5, la última celda cuenta el resto.
- **Proveedores:** `ProviderMascot` reutiliza las mismas siluetas para los iconos de Claude/Codex.

### 2.2 Dónde vive el dato

- `BotConfig.avatar_color` y `BotConfig.avatar_shape` en `host/src/store/model.rs`.
- Valores por defecto en `host/src/store/bots.rs` (`blue` / `blob`).
- Viaja por SSE, snapshot y push; el push manda solo `avatarColor` y `avatarShape` en `faces` (`host/src/remote/push.rs`).
- Cada cliente lo redibuja; no existe una imagen compartida.

### 2.3 Superficies que lo dibujan

| Superficie | Implementación | Notas |
|---|---|---|
| iPhone app | `apps/ios/Kit/Sources/CodyncKit/Design/CharacterAvatar.swift` | Canvas animado + ZStack estático |
| Widgets iOS | misma vista con `still: true` | WidgetKit renderiza una sola frame |
| Live Activity / Dynamic Island | fuente de dígitos generada por `tools/timer-fonts.py` | 10 frames por silueta; requiere regenerar fuentes |
| Notificación iOS | `apps/ios/NotificationService/NotificationService.swift` | `ImageRenderer` a 88 pt |
| Android app | `apps/android/design/src/main/kotlin/com/codync/android/design/CharacterAvatar.kt` | Compose Canvas |
| Widgets y notificaciones Android | `characterAvatarBitmap` | Bitmap estático |
| Desktop | `apps/desktop/src/renderer/components/Avatar.tsx` | Canvas 2D a 30 fps |
| Bandeja / notch island | `tray-summary.ts`, `views/island/island-parts.tsx` | Icono y personaje grande con glow |
| Terminal | `host/src/tui/view/style.rs` | Solo color y bloque `••`; no dibuja la silueta |
| Editores | `BotEditorView` (iOS/desktop), `BotEditor.kt`, `forms.rs` (TUI) | Selector de forma y color |
| Otros usos | roster, chat, grupos, llamada, splash, welcome | Identidad decorativa |

### 2.4 Tamaño del cambio

- **4 renderers independientes** (SwiftUI, Canvas 2D, Compose/bitmap, terminal).
- **≈47 archivos** referencian `CharacterAvatar`, `characterAvatarBitmap`, `drawCharacter` o `workingLoop`.
- **8 fuentes TTF + 8 entradas en `Info.plist`** solo para las siluetas actuales; añadir una silueta implica 2 fuentes nuevas y regenerar las existentes.
- Los tests de `SurfaceRenderTests.swift` exportan los frames y generan snapshots; cualquier cambio de geometría los actualiza.

---

## 3. Restricciones que definen la viabilidad

1. **Regla de diseño documentada.** `docs/design/ui-conventions.md:95` dice: *"Preserve the original character avatar design: dotted shading, highlight-only color, silhouettes and animation."* Un cambio de concepto exige una decisión explícita y actualizar esa regla y `docs/design/mobile-widgets.md` en el mismo cambio.
2. **Paridad entre clientes.** La regla del repo exige implementar el cambio en iOS, Android, desktop y TUI a la vez, o documentar por qué un cliente no se ve afectado.
3. **Protocolo aditivo.** Añadir campos no sube `minApp`; renombrar o quitar `avatarShape`/`avatarColor` sí exige subir el floor, actualizar el snapshot del wire y retener hosts hasta que la App Store tenga la app (`docs/reference/compatibility.md`).
4. **Widgets y Live Activity son estáticos.** WidgetKit renderiza una frame; el Dynamic Island solo anima mediante la fuente de dígitos. Cualquier animación nueva necesita poses fijas y, si va al island, 10 frames pregenerados.
5. **La extensión de notificaciones no puede descargar con fiabilidad.** Un avatar de foto debe tener fallback local (personaje, monograma o emoji).
6. **La terminal no dibuja imágenes.** Necesita un fallback explícito: color + inicial, emoji o un glifo.
7. **Cada cambio de código es un release.** Aplica bump de versión; los cambios solo de documentación no.
8. **Privacidad.** Si se suben fotos, deben quedarse en el host (o en el almacenamiento cifrado del usuario), nunca en el relay.

---

## 4. Diagnóstico: qué problema resolvemos

| Síntoma | Causa probable | Opción que lo resuelve |
|---|---|---|
| "Todos los bots se parecen" | Poca variación percibida entre 8 siluetas y 11 colores | B1 Constelación o B2 Criatura 2.0 |
| "No se leen en el roster" | Contraste insuficiente a 15–18 pt | A Pulido |
| "No representan a mi bot" | Identidad genérica, sin relación con nombre/rol | C1 Emoji o C2 Foto |
| "No sé cuál está trabajando" | Se confunde identidad con estado | No cambiar el avatar: reforzar orb/badge |
| "No me gustan estéticamente" | Dirección visual | A o B1, con prototipo previo |
| "Quiero la misma cara en todas partes" | Superficies estáticas | A + contrato de poses estáticas |

**Decisión clave:** el avatar debe seguir siendo *quién es el bot*; el estado debe seguir siendo *qué está haciendo*. Mezclar ambos hace que el roster sea ilegible.

---

## 5. Matriz de opciones

| Opción | Qué cambia | Diferenciación | Coste | Riesgo | Protocolo | TUI | Widgets/LA |
|---|---|---|---|---|---|---|---|
| **A. Pulido** | Más contraste, más siluetas/colores, mejor picker, grupos, accesibilidad | Media | 3–6 días | Bajo | No | Se alinea | Sin cambios |
| **B1. Constelación** | Estrellas deterministas por bot dentro de la silueta; luz viajera | Alta | 3–5 semanas | Medio-alto | Aditivo | Glifo + color | Pose fija + fuente |
| **B2. Criatura 2.0** | Accesorios, estilos de ojos, combinaciones | Alta | 2–3 semanas | Medio | Aditivo | Color + inicial | Pose fija |
| **B3. Monograma vivo** | Inicial + color + halo de puntos | Baja-media | 3–5 días | Bajo | No | Ideal | Trivial |
| **C1. Emoji** | Un emoji por bot, dentro de la forma/color | Media-alta | 1–2 semanas | Bajo-medio | Aditivo | Emoji si cabe | Texto/imagen |
| **C2. Foto** | Imagen subida, recorte y caché | Muy alta | 4–8 semanas | Alto | Aditivo | Fallback | Caché + fallback |
| **D. Por agente** | Logo del proveedor + color | Baja (identifica runtime, no bot) | 1 semana | Medio | No | Logo no cabe | Limitado |

> Las estimaciones son de ingeniería, sin contar exploración de diseño ni revisión. Incluyen tests y validación por plataforma.

---

## 6. Conceptos expandidos

### 6.1 A. Pulir el personaje actual

**Idea:** mantener el concepto y mejorar lo que hoy falla.

- Ampliar de 8 a 12 siluetas (nuevas: `capsule`, `star`, `diamond`, `arch`).
- Ampliar la paleta con 3–4 tonos (por ejemplo, `mint`, `coral`, `sand`, `indigo`).
- Selector con vista previa grande, nombres legibles ("Blob", no "blob shape"), aleatorio y "copiar avatar de otro bot".
- Mejorar el contraste de los puntos en tamaños pequeños y en modo claro.
- Grupos de 5+ con una disposición más clara que el contador `+N`.
- Alinear la TUI: usar la inicial del bot con su color y un glifo de estado, no un bloque genérico.
- Accesibilidad: etiquetas humanas en el picker y nombre/estado fuera del avatar.

**Coste real:** cada silueta nueva son 2 fuentes TTF nuevas y 2 entradas en `Info.plist`; las 8 existentes se regeneran. Hay que tocar los 4 renderers y los tests de frames.

**Cuándo elegirla:** si el problema es legibilidad, consistencia o falta de variedad, no concepto.

### 6.2 B1. Constelación (concepto nuevo principal)

**Idea:** el bot es una pequeña constelación: 4–12 estrellas dentro de su silueta, unidas por líneas finísimas, generadas de forma determinista a partir de su identidad.

- **Semilla:** `FNV-1a` de `bot.id` (estable aunque cambie el nombre) o una semilla explícita si el usuario "relanza" el avatar.
- **Distribución:** muestreo por rechazo sobre una rejilla entera de 1000×1000 dentro del path de la silueta, con distancia mínima entre estrellas. Sin `sin/cos` para la posición, para que Swift, Kotlin, JS y Rust den el mismo resultado.
- **Aristas:** cada estrella se une a su vecina más cercana; desempate por índice. Máximo 5 aristas para no ensuciar.
- **Estrella ancla:** la más cercana al centroide es más grande y define el "corazón" del bot.
- **Estados:**
  - `idle`: respiración lenta; una estrella parpadea cada pocos segundos.
  - `working`: un cometa recorre las aristas y las estrellas se encienden en secuencia.
  - `needsInput`: pulso ámbar desde el ancla.
- **Tamaños:** 7×7 → 4 estrellas sin líneas; 9×9 → 6 estrellas con 2–3 líneas; 13×13 → 9–12 estrellas con 5 líneas.
- **Grupos:** cada miembro conserva su mini-constelación; el clúster no cambia.
- **Widgets:** una pose fija por mood.
- **Dynamic Island:** 10 frames por silueta, como hoy; hay que regenerar las fuentes.
- **TUI:** `✦` con el color del bot y la inicial; o una mini-rejilla ASCII de 3×3.
- **Ventajas:** diferenciación real sin almacenamiento, encaja con el lenguaje de puntos, el dato que viaja sigue siendo pequeño (`style` + `seed`).
- **Riesgos:** ruido visual a tamaño pequeño, deriva entre plataformas y complejidad de la animación.
- **Criterio de éxito:** con 10 bots en el roster, una persona identifica cada uno a 30 pt sin leer el nombre.

> Esta idea ya apareció en un turno anterior de este worktree (`BotConstellation.kt`, `ConstellationFixtures.kt`, `ConstellationRenderingTest.kt`). Aquí se desarrolla como concepto, no como código existente.

### 6.3 B2. Criatura 2.0 (accesorios)

**Idea:** mantener la silueta y añadir tres ejes de expresión, todos por id:

- **Accesorio:** ninguno, antena, cresta, halo, orejas, visor.
- **Ojos:** redondos, rendija, grandes, dormilones.
- **Patrón:** ninguno, motas, anillo, media luna.

El selector se convierte en "aleatorio + bloquear lo que te gusta". El dibujo sigue siendo de puntos, así que widgets, notificaciones y fonts se reutilizan con menos trabajo que B1.

**Ventajas:** mucha variedad percibida con poco dato nuevo; no requiere almacenamiento.
**Riesgos:** QA combinatorio (6 × 4 × 4 = 96 combinaciones), riesgo de kitsch y de que el accesorio desaparezca a 15 pt. Limitar a 3 accesorios y 3 ojos legibles.

### 6.4 B3. Monograma vivo

**Idea:** inicial (o primera letra CJK) sobre el color del bot, con un halo de puntos y el color solo en el borde.

- Inmediatamente legible, barato y perfecto como **fallback** de TUI, notificaciones y fotos que fallan.
- Poco distintivo: dos bots que empiezan por la misma letra se confunden.
- Se puede combinar: constelación por defecto, monograma en tamaños < 20 pt.

### 6.5 C1. Emoji

**Idea:** un emoji por bot, dibujado dentro de la silueta o sobre el color.

- Sin almacenamiento de archivos: solo una cadena de ≤32 bytes.
- Funciona en widgets, notificaciones y TUI (si el terminal tiene ancho 2).
- Requiere validar que sea **un solo grafema** (no secuencias combinadas), y decidir si el color de fondo sigue siendo el del bot.
- Es la personalización más barata: buen primer paso antes de fotos.
- Riesgo: rompe la estética monocroma; se puede limitar a un emoji "de identidad" y mantener el color.

### 6.6 C2. Foto subida

**Idea:** el usuario sube una imagen, la recorta y la usa como avatar.

- Almacenamiento en el host: original + miniaturas 128/256/512 (WebP/PNG).
- Endpoint autenticado para descargar; caché local por bot y por tamaño.
- iPhone/desktop/Android: subida y recorte; TUI: fallback.
- Widgets: miniatura en el App Group (iOS) o caché local (Android).
- Notificaciones: fallback obligatorio; la NSE no debe depender de la red.
- Privacidad: almacenamiento local/cifrado; nunca en el relay; borrar al eliminar el bot.
- Coste real: infraestructura + caché + recorte + fallbacks + QA visual.

**Cuándo elegirla:** solo si "foto real" o "avatar de marca" es un requisito duro. No es una mejora incremental del concepto actual; es otro producto de identidad.

### 6.7 D. Identidad por agente

**Idea:** logo de Claude/Codex + color del bot.

- Útil para saber *qué runtime* ejecuta, no *quién* es el bot.
- Choca con la identidad única y con `ProviderMascot`.
- No recomendada como avatar principal; sí como icono secundario en detalles.

### 6.8 Híbrido recomendado

Un solo campo `avatarStyle` con:

- `character` (por defecto, el actual),
- `constellation` (B1),
- `emoji` (C1),
- `photo` (C2),
- `monogram` (B3, también fallback).

Así se puede lanzar B1 sin romper nada, añadir emoji después y dejar la foto para más adelante, con el mismo contrato y los mismos fallbacks.

---

## 7. Diseño técnico propuesto

### 7.1 Modelo de datos (aditivo)

```json
{
  "avatarStyle": "character",
  "avatarSeed": null,
  "avatarEmoji": null,
  "avatarImage": null,
  "avatarShape": "blob",
  "avatarColor": "blue"
}
```

- `avatarStyle`: `character | constellation | emoji | photo | monogram`; por defecto `character`.
- `avatarSeed`: 16 hex; si es `null`, se deriva de `bot.id`.
- `avatarEmoji`: un grafema, ≤32 bytes.
- `avatarImage`: id local del host (`[a-f0-9]{32}`), nunca una URL pública.
- `avatarShape` y `avatarColor` se conservan como fallback y para clientes viejos.

**Validación en el host:** enum cerrado, emoji de un solo grafema, seed hexadecimal, id de imagen válido y perteneciente al bot.

**Compatibilidad:**

| Cliente | Host | Resultado |
|---|---|---|
| Viejo | Nuevo | Ignora los campos nuevos; dibuja `shape` + `color` |
| Nuevo | Viejo | No hay `style`; dibuja `character` |
| Nuevo | Nuevo | Dibuja el estilo elegido |
| NSE | Cualquiera | `character`, `monogram` o `emoji`; nunca foto |
| TUI | Cualquiera | Color + inicial (o emoji si cabe) |

**Protocolo:** cambio aditivo; re-grabar `host/tests/fixtures/wire-shape.json` con `CODYNC_UPDATE_WIRE_SHAPE=1 cargo test --test e2e wire_shape`. No subir `MIN_APP` salvo que se renombre o elimine un campo existente.

### 7.2 Determinismo entre plataformas

Para B1:

1. Especificación única en `docs/` con el algoritmo exacto.
2. PRNG `SplitMix64` sembrado con `FNV-1a` del id.
3. Coordenadas enteras 0–1000; sin coma flotante en el layout.
4. Vectores dorados en JSON, compartidos por tests de Swift, Kotlin, TypeScript y Rust.
5. Poses estáticas fijas por mood (`idle` fase 0, `working` fase 0.25, `needsInput` fase 0.6) para widgets y notificaciones.
6. Animación: solo la fase depende del reloj; la geometría no.

Si no se puede garantizar el determinismo, B1 no es viable en paridad y habría que elegir B2 (más simple) o aceptar diferencias visuales entre plataformas.

### 7.3 Superficies y fallbacks

| Superficie | B1 Constelación | C1 Emoji | C2 Foto |
|---|---|---|---|
| iPhone app | Completo | Completo | Completo |
| Widgets iOS | Pose fija | Texto | Miniatura cacheada |
| Live Activity | 10 frames | Estático | Estático |
| Notificación iOS | Personaje/constelación | Emoji | Fallback a personaje |
| Android app | Completo | Completo | Completo |
| Widgets Android | Bitmap estático | Texto | Bitmap cacheado |
| Notificación Android | Bitmap estático | Emoji | Fallback |
| Desktop | Completo | Completo | Completo |
| Bandeja | Versión simplificada | Emoji | Miniatura |
| TUI | `✦` + inicial | Emoji si ancho 2 | Inicial |
| Editores | Picker + "relanzar" | Picker de emoji | Subida + recorte |

**Decisión de diseño:** `ProviderMascot` y el splash deben quedar **desacoplados** del estilo de avatar del bot. Comparten el primitivo de dibujo, pero no la configuración.

### 7.4 Rendimiento y accesibilidad

- Animar solo si la vista está visible; pausar fuera de pantalla y con `Reduce Motion`.
- 30 fps en desktop; en iOS usar `TimelineView`/Canvas como hoy.
- Widgets: una sola frame, sin animación.
- Contraste mínimo: probar 15/20/26/30/46 pt en claro y oscuro.
- El avatar sigue siendo decorativo (`accessibilityHidden`); el nombre y el estado van fuera.
- Picker con etiquetas humanas y estado seleccionado anunciado.

---

## 8. Plan por fases

### Fase 0 — Decisión y prototipo (1–2 días)

**Entregables:**

- Tablero estático con A, B1 y C1 a 15/30/46 pt, claro y oscuro.
- Prueba de identificación con 10 bots ficticios.
- Decisión registrada en este documento.

**Criterio de salida:** una opción elegida y el problema formulado en una frase.

### Fase 1 — Pulido del concepto actual (3–6 días)

**Alcance:**

- 3–4 siluetas nuevas y 3–4 colores nuevos.
- Selector con vista previa grande, nombres legibles y aleatorio.
- Contraste en tamaños pequeños.
- Grupos de 5+ más claros.
- TUI alineada con inicial + color.
- Tests de frames y snapshots actualizados.

**Aceptación:** matriz visual completa, tests verdes, e2e en macOS y simulador.

### Fase 2 — Constelación o Criatura 2.0 (3–5 semanas)

**Alcance:**

- Campo `avatarStyle` + `avatarSeed` (aditivo).
- Especificación y vectores dorados.
- Renderers en SwiftUI, Canvas 2D, Compose y TUI.
- Poses estáticas para widgets y notificaciones.
- 10 frames por silueta y regeneración de fuentes del Dynamic Island.
- Picker con "relanzar constelación".
- Fallback completo para clientes y hosts viejos.

**Aceptación:**

- 10 bots identificables a 30 pt sin leer el nombre.
- 60 fps en desktop y sin caídas en listas largas.
- Widgets y Live Activity verificados en dispositivo.
- Host viejo + cliente nuevo y host nuevo + cliente viejo funcionan.

### Fase 3 — Emoji (1–2 semanas)

**Alcance:**

- `avatarEmoji` validado en el host.
- Picker de emoji en iPhone, desktop y Android.
- Render en todas las superficies con fallback.
- Sin almacenamiento de archivos.

**Aceptación:** emoji visible en roster, chat, widget, notificación y TUI; un solo grafema; sin regresiones en el picker actual.

### Fase 4 — Foto (4–8 semanas, opcional)

**Alcance:**

- Almacenamiento de original + miniaturas en el host.
- Subida y recorte en iPhone, desktop y Android.
- Endpoint autenticado, caché y borrado al eliminar el bot.
- Fallback en NSE, widgets, notificaciones y TUI.
- Revisión de privacidad.

**Aceptación:** funciona offline con caché, no filtra la imagen al relay, y el borrado elimina todos los derivados.

---

## 9. Validación end-to-end

### 9.1 Checks automáticos

```bash
# Host
cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test

# Desktop
cd apps/desktop && npm run typecheck && npm test

# Android
cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug

# iPhone (paquete)
cd apps/ios/Kit && xcodebuild test -scheme CodyncKit-Package -destination 'platform=iOS Simulator,name=<iPhone>'
```

Además:

- `CODYNC_UPDATE_WIRE_SHAPE=1 cargo test --test e2e wire_shape` si cambia el contrato.
- Tests de vectores dorados para B1.
- `SurfaceRenderTests` para snapshots y frames del Dynamic Island.

### 9.2 Matriz visual

- Tamaños: 15, 18, 20, 26, 30, 46, 60, 72, 84 pt.
- Rejillas: 7×7, 9×9, 13×13.
- Apariencia: claro y oscuro.
- Estados: `idle`, `working`, `needsInput`.
- Superficies: roster, chat, grupo, detalles, editor, widget, Live Activity, notificación, island, tray, TUI.

### 9.3 E2E por plataforma

- **Mínimo:** macOS con Computer Use.
- **Completo (cambios grandes):** macOS, Linux (Lima `codync-linux`) y Windows (UTM).
- **iPhone:** simulador con Device Hub; widgets y Live Activity en dispositivo real.
- **Android:** emulador/dispositivo; widgets y notificaciones.
- **TUI:** terminal real, con y sin color, con y sin emoji.
- **Accesibilidad:** VoiceOver/TalkBack y `Reduce Motion`.

---

## 10. Riesgos

| Riesgo | Probabilidad | Impacto | Mitigación |
|---|---|---|---|
| Deriva visual entre Swift, Kotlin, JS y Rust | Alta | Alto | Especificación única + vectores dorados + snapshots |
| Regeneración de fuentes del Dynamic Island | Media | Alto | Script reproducible + prueba en dispositivo |
| Bajo contraste a 15 pt | Media | Medio | Matriz visual + opacidad mínima + fallback a monograma |
| Ruido visual de la constelación | Media | Medio | Limitar estrellas y aristas; probar con 10 bots |
| QA combinatorio de accesorios | Media | Medio | Limitar a 3 accesorios y 3 ojos |
| Coste de fotos | Alta | Alto | Fase 4 separada; solo con requisito duro |
| Privacidad de fotos | Media | Alto | Almacenamiento local/cifrado; nunca en relay |
| Deuda de documentación | Media | Bajo | Actualizar `ui-conventions.md` y `mobile-widgets.md` en el mismo cambio |
| Compatibilidad con clientes viejos | Baja | Alto | Campos aditivos + fallback + matriz viejo/nuevo |

---

## 11. Decisiones pendientes

1. ¿El problema es diferenciación, legibilidad, representación o estética?
2. ¿Se acepta cambiar la regla "preserve the original character avatar design"?
3. ¿La TUI debe mantener paridad completa o basta inicial + color?
4. ¿La foto es un requisito duro o un "estaría bien"?
5. ¿Se elige A, B1, B2, C1 o el híbrido?
6. ¿Hay plazo o presupuesto que descarte fases?
7. ¿Quién valida el diseño visual y con qué tablero?
8. ¿Se lanza B1 detrás de un ajuste o directamente como predeterminado?

---

## 12. Anexo: mapa de archivos

### Host

- `host/src/store/model.rs`: `avatar_color`, `avatar_shape`.
- `host/src/store/bots.rs`: valores por defecto.
- `host/src/remote/push.rs`: `faces` del push.
- `host/src/tui/view/style.rs`: avatar de terminal.
- `host/src/tui/app/forms.rs`: selector TUI.
- `host/tests/fixtures/wire-shape.json`: snapshot del contrato.

### iPhone

- `apps/ios/Kit/Sources/CodyncKit/Design/CharacterAvatar.swift`: renderer, formas y grupos.
- `apps/ios/Kit/Sources/CodyncKit/Design/Theme.swift`: `AvatarPalette`.
- `apps/ios/Kit/Sources/CodyncKit/Design/WidgetCards.swift`: widgets.
- `apps/ios/Kit/Sources/CodyncKit/Design/ActivityCards.swift`: Live Activity.
- `apps/ios/Kit/Sources/CodyncKit/Design/ProviderMascot.swift`: mascotas de proveedor.
- `apps/ios/Kit/Sources/CodyncUI/Bots/BotEditorView.swift`: picker.
- `apps/ios/Kit/Sources/CodyncUI/Bots/BotDetailsView.swift`: detalles.
- `apps/ios/Kit/Sources/CodyncUI/Bots/GroupEditorView.swift`: grupos.
- `apps/ios/Kit/Sources/CodyncUI/Thread/*`: roster, chat, composer, llamada.
- `apps/ios/Views/BotListView.swift`: lista.
- `apps/ios/Widgets/CodyncWidgets.swift`: widget y Dynamic Island.
- `apps/ios/Widgets/Resources/Fonts/*.ttf` + `Info.plist`: fuentes.
- `apps/ios/NotificationService/NotificationService.swift`: notificación.
- `apps/ios/Kit/Tests/CodyncKitTests/SurfaceRenderTests.swift`: snapshots y frames.

### Android

- `apps/android/design/src/main/kotlin/com/codync/android/design/CharacterAvatar.kt`: renderer y bitmap.
- `apps/android/app/src/main/kotlin/com/codync/android/BotAvatar.kt`: avatar con estado.
- `apps/android/app/src/main/kotlin/com/codync/android/BotEditor.kt`: editor.
- `apps/android/app/src/main/kotlin/com/codync/android/EditorControls.kt`: picker.
- `apps/android/app/src/main/kotlin/com/codync/android/BotListView.kt`: roster.
- `apps/android/app/src/main/kotlin/com/codync/android/ChatRows.kt`: chat.
- `apps/android/app/src/main/kotlin/com/codync/android/CodyncWidgets.kt`: widgets.
- `apps/android/app/src/main/kotlin/com/codync/android/AlertNotification.kt`: notificaciones.
- `apps/android/app/src/main/kotlin/com/codync/android/GroupEditor.kt`: grupos.

### Desktop

- `apps/desktop/src/renderer/components/Avatar.tsx`: renderer.
- `apps/desktop/src/renderer/components/avatar.css`: clúster y badge.
- `apps/desktop/src/renderer/views/bots/BotEditorView.tsx`: editor.
- `apps/desktop/src/renderer/views/BotRow.tsx`: roster.
- `apps/desktop/src/renderer/views/island/*`: notch island.
- `apps/desktop/src/renderer/views/tray-summary.ts`: bandeja.
- `apps/desktop/src/renderer/lib/theme.ts`: colores.

### Documentación y herramientas

- `docs/design/ui-conventions.md`: regla de preservación.
- `docs/design/mobile-widgets.md`: halftone y widgets.
- `docs/reference/compatibility.md`: protocolo y floors.
- `tools/timer-fonts.py`: fuentes del Dynamic Island.

---

## 13. Anexo: paleta y siluetas actuales

**Siluetas:** `blob`, `pebble`, `squircle`, `tablet`, `wedge`, `hex`, `cloud`, `teardrop`.

**Colores:** `black` (`#2B2B2B`), `brown` (`#936439`), `red` (`#FF263C`), `orange` (`#FF6700`), `yellow` (`#FF9800`), `green` (`#00C972`), `cyan` (`#00BCA6`), `blue` (`#1084FE`), `violet` (`#9159FE`), `magenta` (`#FF309B`), `gray` (`#777777`).

**Rejillas:** 7×7 (<18 pt), 9×9 (18–27 pt), 13×13 (≥28 pt).

---

## 14. Próximo paso

Prototipar A, B1 y C1 en un tablero estático y hacer la prueba de identificación con 10 bots. Con el resultado, cerrar la decisión de la sección 11 y convertir la fase elegida en un plan de implementación por archivo.
