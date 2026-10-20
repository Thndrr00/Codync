# Plan: constelaciones interactivas de bots por grupo

Fecha: 2026-10-19.
Estado: propuesta de viabilidad e integración; sin implementación.
Ubicación: `docs/nuevo thndr`, junto a los planes existentes.

## Objetivo

Evolucionar la vista Constelación de Android para reconocer equipos y actividad
de un vistazo. Cada grupo tendrá su propia silueta celeste y cada bot será una
estrella interactiva. Los bots en reposo conservarán una presencia tenue; los
que trabajan tendrán un halo más luminoso. Pulsar una estrella permitirá
consultar lo que hace el bot y entrar en su conversación.

Ejemplo de uso: un grupo de tres trabajadores forma un triángulo y otro de
cinco forma una W. Dentro de cada figura se distinguen individualmente los
bots trabajando, esperando al usuario, en error o en reposo. Las conexiones
son una ayuda visual opcional: las posiciones deben sugerir la figura incluso
sin líneas.

Este documento solo propone cambios. No crea botones ni modifica pantallas,
modelos, versiones o comportamiento de la aplicación.

## Viabilidad y límites

La mejora es técnicamente viable con la base actual de Kotlin y Compose. No
requiere un motor 3D, generación de imágenes, inteligencia artificial para el
layout ni una conexión adicional por estrella. La dificultad principal es
mantener legibles y estables las figuras con diferentes tamaños de grupo,
pantallas, escalas de texto y estados de conexión.

La visualización inicial puede aprovechar los grupos y estados actuales sin
nuevos métodos del host. Un sistema de equipos independiente de los chats,
roles formales, asignación de tareas o actividad específica por grupo sí
necesitaría diseño y lógica en el host, con clientes que consuman ese estado.

No se garantiza que exista una constelación astronómica completa y reconocible
para cualquier cantidad de bots. La solución usará patrones estelares
verificados donde correspondan y figuras adaptadas claramente identificadas
para otras cantidades. La forma será una representación visual del grupo;
el brillo será una representación del estado del bot.

## Evidencia del código actual

### Constelación Android

- `apps/android/app/src/main/kotlin/com/codync/android/BotListView.kt` permite
  alternar Lista/Constelación y pasa el roster visible a `BotConstellation`.
- `BotConstellation.kt` dibuja bots individuales, conserva el acceso al chat
  y acciones de pulsación prolongada, adapta el espacio al tamaño de fuente
  y limita la animación de nodos fuera de pantalla.
- `ConstellationLayout.kt` distribuye celdas en filas escalonadas. Hoy no
  selecciona patrones astronómicos, no dibuja conexiones y no divide por grupo.
- `apps/android/design/src/main/kotlin/com/codync/android/design/ConstellationOrb.kt`
  ya ofrece pulsación visual durante trabajo, reposo atenuado, halo de atención
  o no leídos y reloj de animación asociado al ciclo de vida.
- El tamaño actual varía por actividad reciente y no leídos. Esto no debe
  confundirse con estar trabajando en este momento.
- Hay pruebas de layout y pruebas instrumentadas de interacción y renderizado:
  `ConstellationLayoutTest`, `ConstellationScreenTest`,
  `ConstellationRenderingTest` y sus fixtures.

### Creación de bots y grupos: ya existe

- El menú de `BotListView.kt` ofrece `New bot` y `New group chat`.
- `BotEditor.kt` y `GroupEditor.kt` proporcionan los editores correspondientes.
- `AppStoreBots.kt` guarda ambos usando `createBot` o `updateBot` del host.
- `core/Management.kt` define `GroupDraft` con nombre, descripción y miembros;
  valida que existan miembros y que no sean otros grupos.
- `core/HostClient.kt` expone en `Bot` los campos `kind`, `members`, `status`,
  `activity`, `workingChat` y `workingThread`, entre otros.

Un grupo actual es un chat compartido entre bots, no un agente adicional ni un
espacio de trabajo propio. Su interfaz indica que los miembros contestan por
turnos salvo mención. Nombrar bots como “Asistente” o “Gestor” permite reconocerlos,
pero no crea por sí solo una especialización ni reglas de coordinación.

## Decisiones propuestas para la primera entrega

1. Reutilizar chats de grupo como unidades de constelación. No crear otra
   clasificación de equipos en paralelo durante esta entrega.
2. Mostrar secciones verticales por grupo dentro de la pestaña Constelación,
   con nombre y resumen de actividad. Evitar un universo infinito que exija
   zoom o arrastre para encontrar los controles básicos.
3. Representar cada miembro visible con una estrella y conservar identidad
   mediante nombre y el avatar existente. El halo estelar será el protagonista
   del estado, sin eliminar de entrada los avatares personalizados.
4. Añadir una sección “Sin grupo” para bots visibles no incluidos en grupos
   visibles. Nunca perder bots por cambiar de presentación.
5. Mantener Lista como alternativa completa y accesible.
6. Hacer las líneas opcionales, decorativas y tenues. No presentarlas como
   dependencias, ejecución conjunta ni mensajes entre bots.

Son decisiones de propuesta, no ajustes implementados. Si “grupo” debe significar
equipo organizativo independiente del chat, resolver ese modelo en el host
antes de construir la agrupación visual.

## Catálogo de siluetas

Cada plantilla tendrá identificador, nombre, cantidad de nodos, posiciones
normalizadas, aristas opcionales y referencia de procedencia. El catálogo será
local y versionado; no hará consultas astronómicas durante el uso de la app.

- Cero bots: estado vacío con acceso a crear un bot o elegir miembros.
- Un bot: estrella central, sin afirmar que es una constelación astronómica.
- Dos bots: pareja de estrellas, sin inventar un nombre astronómico.
- Tres bots: triángulo inspirado en el Triángulo de Verano, formado por Vega,
  Deneb y Altair. Es un asterismo, no una constelación oficial completa.
- Cinco bots: W inspirada en las cinco estrellas principales de Casiopea.
  El nombre de la figura no implica que toda Casiopea tenga solo cinco estrellas.
- Otras cantidades: incorporar patrones verificados en el catálogo; mientras
  no exista uno adecuado, usar “Figura adaptada” y mantener exactamente un nodo
  por bot. No añadir estrellas ficticias que parezcan trabajadores existentes.

Referencias verificadas para los ejemplos:

- [NASA: Summer Triangle Corner — Altair](https://science.nasa.gov/solar-system/skywatching/night-sky-network/summer-triangle-corner-altair/).
- [NASA: The W in Cassiopeia](https://apod.nasa.gov/apod/ap160318.html).

No reutilizar el Cinturón de Orión como triángulo: sus tres estrellas brillantes
se disponen en una línea. Las figuras serán interpretaciones de interfaz, no
un mapa astronómico de precisión ni una simulación de brillo físico.

## Geometría y estabilidad

- Elegir la plantilla por cantidad de miembros visibles, no por cuántos trabajan.
  Un cambio de estado altera el halo, nunca la identidad de los nodos.
- Usar coordenadas normalizadas, escalar la figura conservando sus proporciones
  y reservar espacio para nombre, halo y área táctil.
- Anclar la asignación de nodos a identificadores estables dentro del contexto
  cuenta/equipo/grupo. Una actualización de actividad o del orden del roster no
  debe intercambiar estrellas.
- Ante altas y bajas que cambien la plantilla, reasignar de forma determinista
  procurando desplazamientos mínimos; transicionar con Motion.
- Guardar solo preferencias o asignaciones de presentación si es necesario.
  No persistir una copia adicional de miembros, roles o estados del host.
- Evitar solapamientos de áreas táctiles y etiquetas. Si no cabe la silueta,
  ampliar la sección con desplazamiento en lugar de reducir nodos sin límite.
- Proponer hasta ocho nodos para figuras compactas. Para grupos mayores,
  ofrecer un mapa ampliado de la misma constelación y un resumen; no ocultar
  miembros ni convertir subdivisiones visuales en grupos nuevos del host.
- Validar el umbral con datos reales y fixtures de 1, 2, 3, 4, 5, 8, 12 y 30 bots.
  No asumir que el mismo tamaño resulta legible en todas las pantallas.

## Estados y lenguaje visual

- `working`: halo luminoso con pulsación suave; texto “Trabajando”. El brillo
  identifica trabajo actual, no actividad histórica ni cantidad de mensajes.
- `idle`: luz tenue y estable, sin desaparecer y manteniendo contraste y área
  táctil. “Apagada” es una metáfora visual, no una indicación de deshabilitado.
- `needsInput`: señal de atención reconocible por icono y texto, además del tono.
- `error`: marca de error y acceso al detalle existente; no representarlo como
  una estrella en reposo.
- No leídos: indicador independiente. No encender el halo de trabajo solo por
  tener mensajes pendientes.
- Sin conexión o estado guardado: mantener una señal explícita de antigüedad
  o conexión y detener la pulsación de trabajo confirmado. Un último estado
  `working` almacenado no acredita que siga ejecutándose.
- Estado desconocido: etiqueta neutral; no transformarlo automáticamente en
  “Reposo” si el host no permite afirmarlo.

Cada grupo mostrará cuántos miembros visibles trabajan, requieren atención o
presentan error. Indicar cuando el conteo corresponde solo a miembros visibles.

La identidad de un bot puede repetirse en varios grupos porque el modelo actual
admite referencias compartidas. Todas sus estrellas deben reflejar el mismo
estado global, sin duplicar procesos. Si trabaja fuera del grupo observado,
usar `workingChat`/`workingThread` cuando sean resolubles y mostrar que la
actividad ocurre en otra conversación; no atribuirla a este grupo por defecto.

## Interacción y botones propuestos

Estos controles se documentan para una implementación futura; no se añaden ahora.

- Pulsar una estrella: abrir un detalle breve con nombre, estado y `activity`
  reportada por el host, seguido de “Abrir conversación”. Si no hay actividad
  específica, mostrar un texto honesto sin inventar qué está haciendo.
- Pulsación prolongada: reutilizar acciones existentes de editar, fijar,
  ocultar y eliminar, con las confirmaciones habituales.
- Pulsar el nombre del grupo: abrir el chat grupal existente.
- Botón “Editar grupo”: reutilizar `GroupEditor`, con permisos y disponibilidad
  equivalentes a los actuales.
- Conservar el menú existente de crear bot/grupo. Evitar duplicar el alta con
  un nuevo backend o un editor paralelo.
- Al añadir un miembro, la figura se recalcula después de la confirmación del
  host. Si falla, mantener la figura anterior y mostrar el error.
- Crear un bot y añadirlo a un grupo puede ser una segunda fase. Son dos
  operaciones del host: si el alta funciona y la asignación falla, conservar
  el bot en “Sin grupo” y ofrecer reintento sin duplicarlo.
- Opción “Mostrar conexiones”: preferencia local de visualización; desactivarla
  no cambia grupos ni actividad. Las líneas no interceptan pulsaciones.

Abrir y cerrar detalles debe animarse con los componentes compartidos. No hacer
que líneas u órbitas aparenten circulación de tareas que el host no reporta.

## Reglas para casos límite

- Miembro oculto o eliminado: no dibujarlo ni revelar su nombre en previews.
  Si el grupo queda sin miembros visibles, mostrar el estado correspondiente.
- Referencia a un miembro todavía no sincronizado: distinguir datos incompletos
  de grupo vacío; no fabricar un nodo con identidad inventada.
- Bot presente en varios grupos: permitir su representación en cada uno,
  identificarla por grupo+bot y contar bots únicos en el resumen global.
- Bots de grupos ocultos: calcular “Sin grupo” sobre grupos visibles para que
  un bot visible no desaparezca indirectamente.
- Eliminación de grupo: preservar bots y chats individuales según el flujo
  existente. Actualizar su aparición en “Sin grupo”.
- Cambio de cuenta/equipo: limpiar selección y asignaciones del contexto
  anterior; no mezclar identidades ni grupos entre equipos.
- Búsqueda futura: preferir resaltar coincidencias conservando la figura;
  si se filtran nodos, explicar que la figura es parcial.

## Integración técnica propuesta

Separar responsabilidades antes de ampliar archivos grandes:

1. `ConstellationPatterns`: catálogo de figuras y sus aristas.
2. `ConstellationLayout`: geometría pura, escalado, reservas y colisiones.
3. Modelo de presentación de grupos: transforma miembros y bots sincronizados
   en secciones visibles; no ejecuta I/O ni redefine reglas de negocio.
4. `BotConstellation`: orquesta las secciones y callbacks de interacción.
5. Componente de estrella: reutiliza o evoluciona `ConstellationOrb` para halos,
   selección, error y atención.
6. Capa decorativa Canvas: dibuja líneas detrás de controles Compose reales.
7. Detalle del bot: componente separado que consume estado existente y delega
   la navegación al store.

No introducir llamadas de red en composables. Reutilizar el mirror y los eventos
que actualizan `AppState`. Cualquier dato de trabajo que falte debe incorporarse
al host y exponerse por su API, no deducirse de animaciones o de mensajes.

## Rendimiento y accesibilidad

- Una sola referencia de tiempo para animaciones visibles, ligada al ciclo de
  vida; no un temporizador por estrella.
- Detener animación fuera de pantalla, con la app inactiva y al activar
  reducción de movimiento. Sin bots trabajando no debe existir animación continua.
- Calcular geometría solo ante cambios de miembros, dimensiones o escala de
  fuente; un evento de actividad no debe rehacer toda la disposición.
- Preferir secciones virtualizadas para muchos grupos y halos acotados, evitando
  desenfoques grandes y capas transparentes sobre toda la pantalla.
- Controles táctiles de al menos 48 dp y etiquetas accesibles con nombre,
  grupo, estado y mensajes pendientes. Orden de lectura estable por grupo.
- Las líneas serán decorativas y quedarán fuera de la navegación accesible.
- Nombre y estado deben ser legibles sin depender de brillo, color o movimiento.
- Mantener soporte de TalkBack, fuente grande y presentación Lista.
- Medir el renderizado en dispositivo; este plan no afirma un presupuesto de
  cuadros cumplido ni un coste de batería ya comprobado.

## Paridad entre clientes

El repositorio exige acompañar los cambios de UI con equivalentes en otros
clientes. La implementación debe planificarse como una entrega coordinada,
aunque la motivación inicial sea Android.

- Android: constelaciones por grupo, halos y detalle táctil según esta propuesta.
- iOS: revisar `apps/ios/Views/BotListView.swift` y el editor de grupos;
  incorporar una presentación equivalente, respetando navegación nativa y
  controles compartidos. No asumir que la constelación Android ya existe allí.
- Escritorio: integrar la opción correspondiente en el roster común de macOS,
  Linux y Windows, con foco por teclado y detalle de actividad. Referencias:
  `apps/desktop/src/renderer/views/BotRoster.tsx`, `BotRow.tsx` y editores de grupo.
- Terminal: conservar equivalencia funcional con grupos, estados textuales,
  selección por teclado y acceso al detalle; la silueta y el halo se adaptan
  a las posibilidades del terminal, sin requerir efectos gráficos idénticos.

Antes de implementar, completar la inspección de cada cliente y documentar las
excepciones concretas. No justificar omitir iOS o escritorio solo porque la
constelación inicial se haya construido en Android. Este documento no modifica
ninguno de los clientes.

## Fases de trabajo y validación

### 1. Cerrar semántica y alcance

Confirmar si los grupos de chat actuales representan los equipos deseados.
La propuesta usa esos grupos. Separar como trabajo posterior los roles formales,
equipos independientes, edición manual de posiciones y asignación automática.

### 2. Prototipo con datos ficticios

Probar dos grupos simultáneos de tres y cinco bots, líneas activadas/desactivadas,
reposo total y estados mezclados. Validar figuras con texto grande, nombres
largos y un miembro compartido. No conectar este prototipo a cuentas personales.

### 3. Layout e integración

Implementar catálogo y geometría pura, agrupación visible y componente de estrella.
Integrar navegación y editores existentes, después errores y desconexión.
Aplicar los equivalentes funcionales en los demás clientes en la misma entrega.

### 4. Pruebas y revisión

- Unitarias de correspondencia exacta bot/nodo, geometría y estabilidad de
  posiciones cuando cambian estado, actividad o no leídos.
- Grupos vacíos, compartidos, ocultos, referencias incompletas, eliminación,
  cambios de cuenta y fallos de creación/asignación.
- Instrumentación de pulsación, menú contextual, detalle, cambio de miembro,
  TalkBack y reducción de movimiento.
- Regresiones de la presentación Lista y de creación/edición de bots y grupos.
- Capturas de tres y cinco nodos, modo claro/oscuro y reposo/trabajo/atención/error.
- Medición de frames con muchos bots y grupos, incluyendo scroll y suspensión.

Ejecutar checks Android establecidos: `:core:test`, `:app:testDebugUnitTest`,
`:app:assembleDebug` y `:app:lintDebug`, además de instrumentación seleccionada
en una instalación desechable. Ejecutar los checks correspondientes de cada
cliente afectado.

Tras pasar pruebas, ejecutar en paralelo los agentes de limpieza y revisión
de refactorización de solo lectura exigidos por `AGENTS.md`, aplicar mejoras
pertinentes y repetir las pruebas antes del recorrido e2e.

### 5. Aceptación real y entrega

Detener las versiones anteriores antes de instalar builds y recorrer el flujo
en las aplicaciones reales. Probar Android en emulador y, si está disponible,
dispositivo físico; validar el resto según las reglas e2e del repositorio.
Reportar plataformas o comprobaciones no disponibles y el motivo.

Criterios de aceptación:

1. Tres miembros visibles forman una silueta triangular y cinco una W,
   sin solapamientos y con exactamente una estrella por miembro en cada grupo.
2. Dos grupos generan dos figuras independientes con sus nombres y resúmenes.
3. Un bot trabajando se distingue al instante sin cambiar de posición.
4. En reposo todas las estrellas quedan tenues, legibles y pulsables.
5. Atención, error, no leídos y desconexión no se confunden con trabajo activo.
6. Pulsar permite ver actividad reportada y abrir la conversación correcta.
7. Altas, bajas y miembros compartidos se reflejan sin duplicar agentes ni
   perder bots visibles.
8. Sin líneas se reconoce la silueta; con líneas no se sugieren dependencias reales.
9. El flujo funciona con reducción de movimiento y navegación accesible.
10. Lista y los editores existentes mantienen su comportamiento.

Actualizar la documentación Android y las guías de UI que queden desfasadas.
Aplicar el versionado coordinado exigido por `AGENTS.md` al implementar código
distribuido; si cambia iOS, incluir sus notas de versión y validaciones.
Crear este documento por sí solo no requiere incremento de versión.

## Evaluación final y pendientes

Viabilidad alta para siluetas, halos, agrupación de chats existentes y navegación.
Complejidad media de presentación; aumenta con grupos grandes y la paridad
entre clientes. Roles formales y coordinación de trabajadores constituyen un
alcance adicional que no debe ocultarse dentro de una mejora visual.

Quedan por validar con prototipo la legibilidad, el límite de nodos compactos,
el coste de renderizado y la utilidad del detalle previo al chat. No se han
ejecutado builds, pruebas visuales ni pruebas de dispositivos para esta propuesta.
