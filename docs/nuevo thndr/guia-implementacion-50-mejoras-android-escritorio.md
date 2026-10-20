# Guía de implementación y viabilidad: 50 mejoras Android y escritorio

Fecha: 2026-10-19. Estado: planificación; sin cambios de código.
Documento de origen: [catálogo de 50 propuestas](plan%20de%20mejora%20andorid+escritorio.md).

## Cómo utilizar esta guía

El catálogo original contiene 50 ideas, no cinco. Esta guía mantiene sus números
y convierte cada idea en una ficha con viabilidad, integración, pasos y prueba
de aceptación. No representa un compromiso de construir las 50 ni autoriza una
reescritura global. Elegir un bloque, confirmar sus decisiones pendientes y
completar su validación antes de comenzar el siguiente.

“Alta” indica una solución conocida sobre la base existente; no significa
terminada. “Condicionada” identifica decisiones de producto, compatibilidad o
contratos todavía pendientes. La complejidad es relativa y no una estimación
de días. No se han ejecutado compilaciones ni pruebas en dispositivos para
redactar esta revisión.

Las rutas de archivos nuevos y los nombres de métodos nuevos son propuestas.
Antes de escribir código, verificar el dispatcher y los contratos actuales,
reutilizar capacidades equivalentes y documentar los cambios definitivos.
Una búsqueda sin coincidencias no acredita que una capacidad no exista.

## Hallazgos que corrigen el catálogo

- `MainTabs.kt` mantiene dos destinos (`Bots`, `State`) con `GlassTabBar`.
  Abrir un bot desde enlaces o notificaciones fuerza el destino Bots; esto
  debe mantenerse al reorganizar la navegación.
- `BotListView.kt` ya ofrece Lista/Constelación y crear bot/chat de grupo.
  Buscadores, filtros y acciones nuevas deben servir también a Constelación.
- `MemoryCard.kt` ya distingue memoria compartida, del bot, Dreams y memoria
  del harness. `MemoryModel.kt` y `core/Memory.kt` incluyen notas, versiones,
  archivado y mutaciones. La idea 35 requiere auditoría de paridad y completar
  huecos concretos, no un port completo desde cero.
- `core/Routine.kt` ya tiene un borrador estructurado de horario y una vista
  previa del host. El editor móvil lo convierte a estilo `custom`; la idea 36
  puede ampliar su UI sin calcular cron ni próximas ejecuciones en Android.
- El archivo Android de Markdown es `MarkdownText.kt`, apoyado en
  `core/MarkdownBlocks.kt`; el bloque Code ya conserva su lenguaje. No usar
  la ruta histórica `Markdown.kt` como si fuera un archivo comprobado.
- `VoiceControls.kt` ya tiene Mute/Unmute. `LocalVoiceAudio.kt` recibe resultados
  parciales del reconocedor; debe verificarse cómo llegan al estado de UI.
- `CodyncSheet` es un componente propio con arrastre y `BackHandler`. Cambiar
  sus detents o predictive back exige adaptar ese componente, no solo instalar
  otra hoja ni añadir un atributo al manifest.
- `host/src/hub.rs` ya calcula keep-awake por bots ocupados o rutinas y
  `host/src/service/mod.rs` tiene `KeepAwake`. La idea 47 debe extender esa
  política y comprobar sus plataformas, sin crear un segundo inhibidor rival.
- `core/Screen.kt` ya contempla mensajes de portapapeles dentro de pantalla
  remota. La idea 49 trata de independizar la transferencia, no inventarla
  ignorando el canal existente.

## Reglas comunes de implementación

1. Trabajar exclusivamente con el fork del usuario. No publicar ni abrir PR
   al upstream. Revisar cambios ajenos antes de tocar archivos; coordinar toda
   operación de sincronización, generación o versionado que afecte el árbol.
2. El host posee búsqueda de historial compartida, permisos, horarios, memoria,
   archivos remotos, presencia y datos sincronizados. Android renderiza estado
   y envía intenciones; no duplica reglas ni ejecuta comandos del equipo remoto.
3. Conservar controles Codync. “Estilo Android” significa adaptar ergonomía,
   disposición y navegación usando componentes compartidos. No autoriza sustituir
   todo por Material, añadir bordes a superficies rellenas ni romper tokens.
4. Separar presentación, estado e I/O. Archivos ya grandes deben dividirse antes
   de sumar comportamiento; las rutas citadas son puntos de entrada, no permiso
   para concentrar toda la implementación allí.
5. Cada mejora funcional visible necesita equivalentes en iOS, escritorio y
   terminal donde sea aplicable. Las integraciones exclusivas del SO requieren
   documentar el equivalente o la razón concreta de no aplicar. No conservar
   sin más la etiqueta “Solo Móvil” del catálogo.
6. Datos y preferencias deben tener alcance explícito: dispositivo para aspecto,
   cuenta/equipo para selección y borradores, host para contenido sincronizado.
   Cancelar solicitudes al cambiar de contexto y rechazar respuestas tardías.
7. Toda operación remota debe representar carga, error, falta de permisos,
   desconexión y capacidades no disponibles. No convertir un fallo en lista vacía
   ni mostrar una acción como completada antes de confirmación.
8. API nuevas: describir petición/respuesta, autorización, límites, errores,
   concurrencia, cancelación, reintentos y compatibilidad. Preferir adiciones y
   detección de capacidad; no elevar mínimos por cambios puramente visuales.

## Bloque A: navegación principal — ideas 1 a 8

### 1. Cuatro destinos principales

- **Viabilidad:** alta; complejidad media. Requiere decidir qué contiene Actividad
  y conservar acceso a State, widgets y uso; no eliminarlos por cambiar etiquetas.
- **Implementación:** definir destinos tipados y un controlador de navegación;
  mapear Chats, Actividad, Extensiones y Ajustes/PC a pantallas existentes.
  Separar el contenido hoy presentado en hojas para reutilizarlo como destino.
  Actualizar `MainTabs.kt`, composición de `CodyncApp.kt` y `Chrome.kt` sin
  duplicar stores. Guardar selección y posición de cada destino.
- **Aceptación:** volver conserva el historial y borradores; notificación,
  widget o enlace abren el bot correcto aunque otro destino esté activo.

### 2. Badges en navegación

- **Viabilidad:** alta; complejidad baja después de 1.
- **Implementación:** crear un resumen de presentación desde bots visibles y
  conexión confirmada; no sumar miembros y chats grupales como si fueran bots
  únicos. Mostrar no leídos, atención y trabajo con significados separados.
  Incorporar textos accesibles y máximo visual como `99+`, sin truncar el valor
  anunciado. Desactivar pulsación al reducir movimiento o perder conexión.
- **Aceptación:** un bot compartido por dos grupos no duplica el contador;
  datos guardados no anuncian trabajo en tiempo real.

### 3. Acción flotante contextual

- **Viabilidad:** alta; complejidad baja/media después de 1.
- **Implementación:** extraer las acciones de `NewMenuButton` para reutilizarlas
  en un control Codync inferior; mantener `newBot`, `newGroup` y sus condiciones
  actuales de disponibilidad. Reservar insets para teclado, navbar y llamadas.
  El menú expandido se cierra al navegar o pulsar atrás.
- **Aceptación:** ambas acciones abren sus editores existentes una sola vez;
  el control no tapa la última fila ni las estrellas de Constelación.

### 4. Gestos entre destinos y volver al inicio

- **Viabilidad:** condicionada; complejidad media. No habilitar deslizamiento
  global hasta resolver conflictos con arrastre de bots, código y pantalla remota.
- **Implementación:** usar pager solo en las raíces compatibles, con una única
  fuente de selección. Enviar un evento de reselección a la raíz activa para
  desplazar al inicio. Definir explícitamente re-tap frente a doble toque;
  se recomienda re-tap por accesibilidad. No consumir gestos dentro del chat.
- **Aceptación:** navegar por botones y gestos da el mismo estado; TalkBack y
  teclado no dependen del gesto; una selección no abre dos destinos.

### 5. Ocultación de barra al desplazar

- **Viabilidad:** alta técnicamente; condicionada a prueba de usabilidad.
- **Implementación:** un estado de visibilidad compartido recibe deltas del
  contenedor desplazable activo mediante nested scroll. Aplicar umbral e
  histéresis para evitar parpadeos. No ocultar con foco en la barra, lector
  de pantalla activo o acciones críticas pendientes. Mantener accesible la
  navegación y no alterar insets bruscamente al animar.
- **Aceptación:** scroll lento y cambios de dirección no provocan saltos;
  aparecer/desaparecer no mueve el contenido ni tapa controles.

### 6. Minibarra de llamada y pantalla remota

- **Viabilidad:** alta para representar una sesión existente; media de integración.
- **Implementación:** exponer estado resumido del controlador de voz/pantalla
  en el shell de navegación. Reutilizar mute, terminar y volver a sesión;
  abandonar el chat no debe destruir la sesión ni crear otra. Distinguir
  audio de visor remoto y definir cómo conviven. No capturar audio desde UI.
- **Aceptación:** cambiar de pestaña conserva audio y sesión; terminar libera
  recursos y elimina la barra; desconexión se ve como tal.

### 7. Layout adaptable y vista dividida

- **Viabilidad:** alta; complejidad alta por navegación y ciclo de vida.
- **Implementación:** medir ventana utilizable y postura, no solo orientación
  o ancho físico. Introducir modo compacto y lista/detalle con el mismo store;
  en compacto volver al roster, en expandido conservar selección junto a lista.
  Adaptar el rail con componentes Codync y revisar la orientación del manifest.
  Respetar bisagra, teclado, insets y ancho mínimo de cada panel.
- **Aceptación:** plegar, rotar o redimensionar no duplica llamadas ni pierde
  borradores. Validar teléfono, ventana partida y tablet. Ver fuente S1.

### 8. Selector de estilo de navegación

- **Viabilidad:** alta; complejidad media y coste permanente de mantenimiento.
- **Implementación:** después de estabilizar 1, definir un único modelo de
  destinos y varios renderizadores. Guardar una preferencia de dispositivo.
  Todos deben ofrecer los mismos destinos y badges; el modo clásico no puede
  dejar funciones inaccesibles. Empezar con un estilo y añadir variantes solo
  si tienen una necesidad demostrada.
- **Aceptación:** cambiar estilo mantiene destino, foco y borrador, también
  tras reiniciar. Validar cada variante con fuente grande.

## Bloque B: apariencia y controles — ideas 9 a 16

### 9. Color dinámico, acento y negro OLED

- **Viabilidad:** alta; complejidad media. Color dinámico depende de soporte del SO.
- **Implementación:** centralizar resolución en `Palette.kt` y `CodyncTheme.kt`:
  tema del sistema/manual, superficies, acento y colores semánticos separados.
  En API compatible obtener colores dinámicos y traducirlos a tokens Codync;
  mantener paleta de respaldo y advertencia/error estables. Compartir este
  cambio con 16 para no crear dos motores de tema.
- **Aceptación:** contraste legible en claro, oscuro, negro y fondos dinámicos;
  las preferencias sobreviven y el fallback funciona. Ver S2.

### 10. Acento por bot

- **Viabilidad:** alta; complejidad baja después de 9.
- **Implementación:** derivar un acento contextual de `avatarColor` y aplicarlo
  solo a elementos seleccionados del chat mediante tokens locales. Mantener
  colores de error, permisos y éxito. No modificar la preferencia global ni
  añadir contornos a superficies rellenas del compositor.
- **Aceptación:** cambiar rápidamente de bot no deja el color anterior;
  enlaces y botón de envío conservan contraste en todos los temas.

### 11. Cabecera Android

- **Viabilidad:** alta; complejidad media por densidad y navegación.
- **Implementación:** componer atrás, identidad y acciones con controles Codync;
  delegar atrás al stack existente. Abrir inspector al tocar identidad.
  Colapsar acciones secundarias en menú según ancho y tamaño de fuente.
  Preservar título, cuenta/equipo y estado necesarios para distinguir bots.
- **Aceptación:** nombres largos no desplazan volver ni acciones; back del
  sistema y botón visual producen el mismo resultado.

### 12. Superficies ligeras

- **Viabilidad:** alta para alternativa tonal; mejora a 120 Hz no demostrada.
- **Implementación:** medir primero la superficie actual. Añadir una ruta sin
  blur/lens a los componentes `Glass.kt` y tokens de superficies, sin bordes
  añadidos a fondos rellenos. Conservar jerarquía y estados de interacción.
  Elegir por preferencia de dispositivo; no inferir rendimiento solo por API.
- **Aceptación:** comparar scroll y frames en el mismo equipo/escenario con
  datos equivalentes. Publicar mediciones, sin prometer 120 Hz por diseño.

### 13. Fuente y densidad

- **Viabilidad:** alta; complejidad media.
- **Implementación:** preferencia de escala de chat y densidad con límites,
  aplicada sobre la escala accesible del sistema, sin anularla. Resolverla en
  `Type.kt`/tema y componentes de filas. Mantener áreas táctiles aunque se
  reduzca padding. Compartir semántica con el ajuste de escritorio existente.
- **Aceptación:** fuente grande del sistema más escala local no recorta texto
  o botones; cambiar densidad no pierde posición ni borradores.

### 14. Háptica

- **Viabilidad:** alta; complejidad baja.
- **Implementación:** servicio pequeño de feedback de dispositivo invocado por
  acciones ya confirmadas. Respetar ajustes del sistema y evitar vibración
  por recomposición. Drag emite al comenzar/cambiar posición, no cada pixel.
  Para notificaciones usar su canal, evitando doble vibración desde UI.
- **Aceptación:** una acción genera a lo sumo un feedback pertinente; desactivar
  háptica no cambia funcionalidad ni genera vibraciones residuales.

### 15. Hojas con alturas y predictive back

- **Viabilidad:** alta; complejidad alta al ser una infraestructura compartida.
- **Implementación:** representar estados de hoja con enum y detents permitidos;
  calcular alturas desde contenido, ventana e IME. Unificar arrastre, scroll
  interno y back en el mismo controlador de transición. Para predictive back
  gestionar progreso, cancelación y confirmación con APIs compatibles; conservar
  fallback del sistema. No sustituir todas las hojas en una sola tanda.
- **Aceptación:** cancelar el gesto restaura foco/posición; confirmar cierra
  una vez; formularios largos y teclado no quedan inaccesibles. Ver S3.

### 16. Tema manual

- **Viabilidad:** alta; complejidad baja si se entrega con 9.
- **Implementación:** enum Sistema/Claro/Oscuro/Negro OLED en ajustes de
  dispositivo. Resolver el modo en la raíz; actualizar barras del sistema,
  diálogos, hojas y previews. Migrar preferencias existentes con un default
  que conserve la apariencia actual.
- **Aceptación:** Sistema responde a cambios del SO; los demás modos se
  mantienen tras recreación y reinicio sin destellos de un tema incompatible.

## Bloque C: roster y acciones — ideas 17 a 24

### 17. Búsqueda de bots e historial

- **Viabilidad:** alta para filtrar metadatos; condicionada y más costosa para historial.
- **Implementación:** separar filtros locales de nombre/cwd/agente de búsqueda
  de mensajes. Localizar primero cualquier API/indexador del host equivalente;
  ampliar allí búsqueda paginada por cuenta autorizada/equipo/bot/hilo. Android
  envía consulta con debounce y cancelación. Un modo offline sobre mirror debe
  rotular resultados parciales, sin convertirse en otro buscador de producto.
- **Aceptación:** consultas tardías no reemplazan la actual; saltar abre mensaje
  e hilo correctos; “sin coincidencias” se distingue de búsqueda fallida.

### 18. Filtros rápidos

- **Viabilidad:** alta; complejidad baja/media después de 17.
- **Implementación:** enum de filtro, búsqueda y agrupación como ejes separados
  de presentación. Aplicar a los mismos bots visibles en Lista y Constelación.
  No confundir `needsInput` con cualquier aprobación ni borrar orden del host.
  En Constelación decidir resaltar coincidencias o indicar figura parcial,
  según su plan específico.
- **Aceptación:** cada filtro da el mismo conjunto en ambas vistas; limpiar
  filtros restaura el roster y distingue vacío real de vacío por filtro.

### 19. Gestos de acción en filas

- **Viabilidad:** alta; complejidad media por conflictos de gestos.
- **Implementación:** reutilizar acciones de `BotRosterActions.kt` y sus comandos;
  no duplicar mutations. Deslizamiento revela acciones explícitas; detener
  requiere pulsarlas, evitando que un gesto accidental cancele trabajo.
  Resolver precedencia con reordenamiento y pager. Ofrecer menú/teclado como
  alternativa; rollback visual si el host rechaza.
- **Aceptación:** drag de orden no fija/oculta; un stop afecta al bot elegido
  y un fallo conserva la fila y muestra causa.

### 20. Panel de atención y aprobaciones

- **Viabilidad:** alta para resumen; condicionada para aprobar desde él.
- **Implementación:** listar bots pendientes y resolver la solicitud real con
  sus `entryId`, opciones y contexto, usando el flujo de permisos existente
  (`respondPermission`, a través del store). No aprobar por el mero estado
  `needsInput`. Si hay varias solicitudes o información incompleta, abrir detalle.
  Revalidar que siga pendiente al pulsar y reflejar resolución de otro cliente.
- **Aceptación:** dos clientes no resuelven dos veces una petición; el usuario
  ve qué autoriza y un permiso caducado no habilita otra operación.

### 21. Metadatos en filas

- **Viabilidad:** alta; complejidad baja.
- **Implementación:** modelo compacto de agente, proyecto y Fast mode desde
  campos ya sincronizados. Respetar la diferencia entre Fast solicitado,
  disponible y rechazado. Mostrar solo datos útiles según ancho y densidad;
  grupo sin workspace no debe recibir el cwd de un miembro arbitrario.
- **Aceptación:** campos ausentes y nombres largos no producen badges falsos
  ni eliminan estado, atención o mensajes pendientes.

### 22. Agrupar por proyecto

- **Viabilidad:** alta para presentación; media si se requiere identidad canónica.
- **Implementación:** conservar orden del host dentro de cada sección. Clave de
  proyecto incluye equipo e identidad de workspace, nunca solo basename.
  Si el host solo expone cwd, no intentar canonicalizar rutas remotas en Android;
  mostrar agrupación por ruta declarada o añadir identidad al host. Guardar
  plegado por contexto y separar “Por proyecto” de chats grupales.
- **Aceptación:** dos carpetas llamadas igual en equipos distintos no se mezclan;
  bots sin carpeta quedan accesibles y crear/ocultar actualiza su sección.

### 23. Duración y detener

- **Viabilidad:** alta si el host entrega inicio de ejecución y destino de stop.
- **Implementación:** verificar significado de `startedAt` antes de usarlo como
  reloj de tarea; si falta, mostrar duración no disponible. Un reloj compartido
  de UI actualiza solo filas visibles. Reutilizar cancelación existente, con
  estado “deteniendo” hasta evento confirmado y sin simular reposo local.
- **Aceptación:** reconexión no reinicia duración; otra ejecución no hereda
  temporizador anterior; desconexión no simula stop exitoso.

### 24. Previsualización al mantener pulsado

- **Viabilidad:** alta; complejidad media.
- **Implementación:** reutilizar preview del menú contextual y cargar hasta
  tres mensajes mediante store/mirror, con consulta acotada y cancelable.
  Distinguir conversación principal e hilo; no marcar como leído al previsualizar
  salvo decisión explícita compartida. Preservar acciones de accesibilidad.
- **Aceptación:** cerrar cancela carga; cambiar cuenta no expone mensajes previos;
  sin caché ofrece abrir el chat y no muestra un historial vacío inventado.

## Bloque D: chat y contenido — ideas 25 a 34

### 25. Bloques de código

- **Viabilidad:** alta; complejidad media; el parser ya guarda lenguaje.
- **Implementación:** extraer `CodeBlock` de `MarkdownText.kt`; encabezado con
  lenguaje, copia literal y wrap por bloque. Resaltado local con límites de
  longitud y fallback de texto plano, sin consultas externas. Para bloques
  enormes, diferir tokenización; pantalla ampliada reutiliza el componente.
- **Aceptación:** copiar conserva espacios y saltos; lenguajes desconocidos
  funcionan; respuestas en streaming no congelan el chat.

### 26. Renderizado de diff

- **Viabilidad:** alta; complejidad media después de 25.
- **Implementación:** reconocer bloques diff, conservar prefijos y cabeceras,
  construir filas de hunks con numeración antigua/nueva solo cuando los rangos
  permitan calcularla. Tokens de añadido/eliminado y marcas textuales; diff
  incompleto vuelve a texto seguro. Visualizar nunca implica aplicar el parche.
- **Aceptación:** renombres, binarios y hunks parciales no inventan líneas;
  copia devuelve el diff original y el color no es el único indicador.

### 27. Atajos del compositor

- **Viabilidad:** alta; complejidad media por edición y teclado.
- **Implementación:** operaciones puras sobre texto, selección y cursor:
  envolver selección en código, insertar bloque o abrir sugerencias existentes
  de mención/comando. Evitar reemplazar borradores completos. Pegar solo ante
  acción explícita; registrar en undo y conservar selección tras IME.
- **Aceptación:** selección inversa, emojis, texto multilínea y dictado no
  corrompen el borrador; un atajo nunca envía el mensaje.

### 28. Deslizar para abrir hilo

- **Viabilidad:** alta; complejidad media.
- **Implementación:** llamar a la navegación a replies existente desde un gesto
  con umbral y dirección definida para RTL. Excluir selección de texto,
  desplazamiento horizontal de código y gestos de archivos. Añadir la misma
  acción al menú contextual; no crear replies automáticamente al deslizar.
- **Aceptación:** un único hilo correcto se abre; el scroll vertical y copiar
  texto siguen funcionando y volver conserva la posición del chat.

### 29. Cámara y anotación

- **Viabilidad:** alta; complejidad media/alta por archivos y recuperación.
- **Implementación:** auditar captura existente antes de añadir otra ruta.
  Capturar a URI privada temporal con FileProvider/contrato compatible;
  permisos solo cuando correspondan. Editor de anotación separado con undo
  exporta copia, corrige orientación y limita dimensiones/memoria. Reutilizar
  preparación, límite de adjuntos y cola existentes; borrar temporales cancelados.
- **Aceptación:** denegar cámara permite continuar con archivos; rotar o perder
  proceso conserva o recupera borrador sin enviar ni duplicar. Ver S4.

### 30. Buscar en conversación

- **Viabilidad:** alta; complejidad media/alta; comparte backend con 17.
- **Implementación:** consulta por bot y ámbito de hilo con cursor y límite.
  Resultados identifican entrada, coincidencia y contexto; store carga el tramo
  necesario antes del salto. Resaltar sin modificar el Markdown original.
  Una búsqueda en caché debe informar que el historial puede estar incompleto.
- **Aceptación:** búsqueda encuentra mensajes fuera de la página visible,
  cambiar consulta cancela la anterior y cerrar restaura navegación normal.

### 31. Nuevos mensajes sobre JumpToLatest

- **Viabilidad:** alta; complejidad media.
- **Implementación:** seguir el ancla visible y los identificadores recibidos
  desde que se dejó el final, sin incrementar por editar un mensaje en streaming
  ni por cargar historia anterior. Mantener estado por bot/hilo y resetear al
  alcanzar el final o confirmar lectura conforme al flujo actual.
- **Aceptación:** cien fragmentos de una respuesta no cuentan como cien mensajes;
  llegar al final limpia contador y no salta durante lectura histórica.

### 32. Timeline de Full conversation

- **Viabilidad:** alta; complejidad media.
- **Implementación:** crear filtros tipados sobre eventos reales de `TraceView`;
  comandos, herramientas, archivos y errores derivan del tipo de evento, no de
  buscar palabras en mensajes. Preservar orden y anclas al plegar o filtrar.
  Solo mostrar pensamiento que el proveedor y host ya expongan al usuario.
- **Aceptación:** eventos desconocidos siguen consultables; filtrar no elimina
  datos y respuestas largas mantienen scroll y rendimiento.

### 33. Inspector unificado

- **Viabilidad:** alta; complejidad media/alta por composición.
- **Implementación:** extraer contenido reutilizable de detalles, memoria,
  rutinas y ajustes del bot; cada sección mantiene su modelo de estado/I/O.
  Un contenedor controla selección, navegación y borradores sin cargar todos
  los datos en cada apertura. Mostrar capacidades del backend y permisos reales.
- **Aceptación:** cambiar pestaña no guarda cambios accidentalmente; salir con
  edición pendiente conserva el flujo actual; memoria no carga desde la vista.

### 34. Selección parcial y cita

- **Viabilidad:** alta para selección existente; condicionada por integración
  del menú de selección de Compose usado por el proyecto.
- **Implementación:** verificar API de toolbar/selección de la versión fijada;
  añadir Citar sobre el fragmento seleccionado, escaparlo como cita multilínea
  e insertarlo en la posición del borrador. Fallback: acción de citar mensaje
  completo cuando no se pueda obtener selección parcial de forma fiable.
- **Aceptación:** no se incluye texto fuera de la selección ni se pierde el
  borrador; fuentes, enlaces y código mantienen copiar/seleccionar.

## Bloque E: productividad e integración del sistema — ideas 35 a 40

### 35. Completar memoria y notas

- **Viabilidad:** alta para completar diferencias; base sustancial ya existente.
- **Implementación:** elaborar checklist contra host/iOS/escritorio: capas,
  archivado, versiones, restauración, memoria del harness y avisos handoff.
  Probar cada función de `MemoryCard`, `MemoryNotes`, `MemoryModel` y `MemoryCalls`.
  Cambiar solo los huecos encontrados. Conservar control de concurrencia por
  hash/base al editar notas y el diagnóstico para hosts antiguos.
- **Aceptación:** edición concurrente produce conflicto recuperable; restaurar
  una versión no pisa cambios silenciosamente; cambio de cuenta cancela cargas.

### 36. Horarios visuales de rutinas

- **Viabilidad:** alta; complejidad media usando contrato estructurado existente.
- **Implementación:** añadir editor visual para campos de `RoutineScheduleDraft`
  y dejar conversión, validación y próximas fechas al host. Revisar
  `forMobileEditor()` para no forzar siempre estilo custom. Preservar triggers
  originales desconocidos y horarios avanzados. Mostrar zona horaria explícita.
- **Aceptación:** días, intervalos, cambio de zona y horario de verano coinciden
  con preview del host; guardar requiere una validación de la revisión actual.

### 37. Recibir desde Compartir

- **Viabilidad:** alta; complejidad media/alta.
- **Implementación:** receptor explícito de SEND/SEND_MULTIPLE con tipos MIME
  admitidos; crear borrador de importación antes de elegir bot, nunca enviar
  automáticamente. Copiar adjuntos a almacenamiento privado mientras la URI
  temporal sea accesible; persistir permisos solo si el proveedor los concede.
  Validar tamaño y contenido por el flujo de adjuntos existente.
- **Aceptación:** app cerrada, proveedor sin permiso persistente, cancelación y
  cambio de cuenta no pierden datos ni envían al destinatario anterior. Ver S4.

### 38. Accesos del launcher

- **Viabilidad:** alta; complejidad media por ciclo de cuentas y privacidad.
- **Implementación:** publicar pocos atajos de bots fijados con identificadores
  estables y enlaces internos; el límite lo decide el launcher. Actualizar al
  cambiar nombre/fijado y retirar atajos dinámicos al cerrar sesión. Un acceso
  fijado puede sobrevivir: revalidar cuenta, equipo y bot al abrirlo.
- **Aceptación:** un atajo antiguo muestra destino no disponible sin abrir otro
  bot; pin se solicita mediante el sistema y no se da por confirmado. Ver S5.

### 39. Responder y aprobar desde notificaciones

- **Viabilidad:** condicionada; complejidad alta. Separar respuesta de permisos.
- **Implementación:** reutilizar cola de envío con nonce estable para RemoteInput,
  receptor explícito y validación de contexto. Para aprobar, resolver solicitud
  pendiente y opciones reales en el host; exigir desbloqueo donde corresponda
  y abrir detalle si falta contexto. Diseñar PendingIntents con mutabilidad
  mínima compatible con RemoteInput; no publicar receptores genéricos.
- **Aceptación:** doble toque no duplica mensajes; permiso vencido no ejecuta;
  offline y acción fallida muestran estado real, sin éxito anticipado. Ver S6.

### 40. Biometría para operaciones sensibles

- **Viabilidad:** alta como protección local; complejidad media/alta.
- **Implementación:** definir operaciones protegidas y duración de autorización;
  lanzar BiometricPrompt desde la capa Android, con fallback permitido por el
  producto y hardware. Ante cambio de cuenta, background o cancelación invalidar
  el estado. Credenciales nunca pasan a logs ni se almacenan solo para pintarlas.
  La autorización real y revocación siguen perteneciendo al host.
- **Aceptación:** cancelar no ejecuta la operación; otro cliente no hereda el
  desbloqueo local. La biometría no se presenta como política remota. Ver S7.

## Bloque F: pantalla remota y voz — ideas 41 a 44

### 41. Macros y flechas

- **Viabilidad:** alta; complejidad media.
- **Implementación:** ampliar el modelo de combinaciones de `ScreenKeys.kt`,
  adaptando Cmd/Ctrl al equipo remoto. Reutilizar el canal de entrada actual.
  Para repetición, emitir down/up equilibrados y liberar todas las teclas al
  cancelar gesto, cambiar ventana o perder conexión. Pegar usa transferencia
  de texto existente cuando un simple Ctrl+V no lleva el portapapeles local.
- **Aceptación:** no quedan modificadores pulsados tras desconexión; cada macro
  produce la combinación correcta en macOS, Linux y Windows.

### 42. Picture-in-Picture

- **Viabilidad:** condicionada a soporte del dispositivo y arquitectura de sesión;
  complejidad alta, no resuelta solo declarando `supportsPictureInPicture`.
- **Implementación:** prototipo limitado al visor de vídeo; ocultar controles
  de entrada y conservar sesión fuera del composable visible. Manejar entrada,
  salida y cierre PiP, relación de aspecto y lifecycle; revisar audio/servicios
  de fondo solo para lo que realmente continúe. No ofrecer control remoto
  completo en una ventana demasiado pequeña.
- **Aceptación:** cerrar PiP libera la sesión según política; volver restaura
  visor sin segunda conexión; desconexión no congela una imagen como viva. Ver S8.

### 43. Lupa en entrada directa

- **Viabilidad:** alta; complejidad media.
- **Implementación:** transformar dedo a coordenadas del framebuffer usando
  escala, letterbox, zoom y rotación vigentes. Lupa decorativa desplazada del
  dedo, recortada desde el vídeo disponible, sin segunda captura. Separar
  posición de preview del evento de click y preservar modo trackpad.
- **Aceptación:** lupa y click coinciden en bordes, zoom y pantallas de distinta
  proporción; soltar/cancelar elimina overlay sin clicks fantasmas.

### 44. Voz: transcripción y salida de audio

- **Viabilidad:** alta para ampliar controles; condicionada a rutas/proveedor
  disponible; complejidad media/alta. Mute ya existe.
- **Implementación:** auditar estado de voz y resultados parciales; exponer
  texto provisional/final distinto del mensaje enviado. Centralizar rutas en
  la capa de audio Android y mostrar solo salidas disponibles. Coordinar foco
  de audio, Bluetooth, interrupciones y minibarra de 6; no reiniciar la llamada
  al cambiar de vista. No registrar transcripciones por defecto.
- **Aceptación:** desconectar auriculares actualiza ruta; mute detiene captura
  según contrato; subtítulos no se envían dos veces. Ver S9.

## Bloque G: nuevas capacidades compartidas — ideas 45 a 50

### 45. Archivos del proyecto y cambios Git

- **Viabilidad:** alta; complejidad alta. Auditar lectura de archivos existente
  antes de añadir endpoints; los nombres del catálogo son tentativos.
- **Implementación:** resolver workspace autorizado en el host, listar con
  paginación y exclusiones explícitas, y consultar Git en proceso acotado con
  argumentos estructurados. Canonicalizar rutas, verificar symlinks/junctions,
  limitar profundidad/bytes y cancelar consultas. Diferenciar staged, unstaged,
  untracked y binarios; archivos sin Git conservan exploración válida.
- **Contrato propuesto:** contexto de bot/workspace y cursor; respuesta con
  ruta relativa, revisión/snapshot, estado y truncamiento. Diff recibe la
  revisión consultada o advierte si cambió. No aceptar comandos shell arbitrarios.
- **Clientes:** árbol y panel de cambios consumen el mismo contrato; autocompletar
  archivos usa búsqueda limitada del host, no descarga todo el árbol.
- **Aceptación:** repositorio grande, rutas externas, archivo cambiado durante
  lectura y salida truncada se manejan sin bloqueo ni escape de workspace.

### 46. Continuar entre dispositivos

- **Viabilidad:** alta; complejidad media/alta. El Handoff de memoria existente
  no acredita presencia de navegación entre clientes.
- **Implementación:** estado efímero del host por cliente autorizado con bot,
  hilo y ancla opcional, TTL y secuencia. Publicar al navegar en primer plano,
  retirar/expirar al desconectar y limitar a la cuenta/equipo correctos.
  Consumirlo como sugerencia; no robar foco ni seguir automáticamente al otro
  cliente. No enviar texto de borradores en la primera versión.
- **Contrato propuesto:** publicar foco y suscribirse a sugerencias vigentes,
  con origen/fecha de observación y referencia estable del destino.
- **Aceptación:** destino borrado, cliente dormido y eventos fuera de orden no
  abren chats incorrectos; dos clientes no generan un bucle de navegación.

### 47. Salud del host y keep-awake

- **Viabilidad:** alta para métricas soportadas; condicionada para control de
  energía equivalente en todos los SO; complejidad alta.
- **Implementación:** recoger métricas en servicio del host con frecuencia
  acotada y muestreo bajo demanda/suscripción. Distinguir memoria del proceso
  de RAM del sistema; no inventar batería en equipos sin sensor. Extender
  `Hub.update_keep_awake()` con intención manual temporal y automática separadas;
  retirar un override no debe anular la protección de una tarea activa.
- **Contrato propuesto:** métricas con unidad, timestamp y campo no disponible;
  política manual con vencimiento y estado efectivo, motivo y soporte por SO.
- **Aceptación:** reinicio/cierre y caducidad liberan inhibidores; tarea activa
  conserva su protección; sin soporte el cliente informa la limitación.

### 48. Previsualización de servidor local

- **Viabilidad:** condicionada; complejidad muy alta. Un RPC que devuelve HTML
  no basta para una web moderna con assets, cookies, redirecciones y WebSockets.
- **Implementación:** primera fase HTTP de solo navegación con puerto elegido
  y autorizado en el host; limitar loopback, puertos y redirects. Definir sesión
  con revocación y cuotas. Diseñar transporte binario/streaming y después
  WebSocket para HMR si se requiere. No interpolar URLs libres en una petición
  del host ni exponer red privada arbitraria mediante redirecciones.
- **Clientes:** origen aislado por sesión, sin bridge nativo, navegación externa
  controlada y cookies separadas de login/Codync. Resolver URL absoluta y assets
  antes de prometer compatibilidad con frameworks específicos.
- **Aceptación:** assets y rutas funcionan; WebSocket solo se anuncia tras prueba;
  cerrar sesión o cambiar cuenta revoca el acceso. Revisión de aislamiento
  y amenazas del proxy obligatoria antes de habilitarlo por defecto.

### 49. Portapapeles y drop de archivos

- **Viabilidad:** alta para transferencia explícita; sincronización automática
  del clipboard condicionada por restricciones Android y privacidad.
- **Implementación:** empezar con “Enviar texto/archivo” y “Recibir” iniciados
  por el usuario, reutilizando canal cifrado, transferencia por chunks y mensajes
  clipboard existentes donde encajen. Separar recepción de escritura efectiva
  en el portapapeles/descargas. En Android no prometer lectura continua en fondo.
- **Contrato propuesto:** transferencia identificada, destinatario autorizado,
  tipo, tamaño, integridad, progreso, caducidad y cancelación. Host gestiona
  destino permitido y colisiones de nombre; dispositivos hacen su I/O local.
- **Aceptación:** archivo grande, reconexión y reintento no duplican resultados;
  cancelar borra temporales; contenido no se pega sin acción explícita. Ver S10.

### 50. Biblioteca de prompts

- **Viabilidad:** alta; complejidad media/alta; requiere contenido sincronizado
  en host. Distinguir prompts reutilizables de skills y del prompt del sistema.
- **Implementación:** modelo SQLite en host con id, título, cuerpo, etiquetas,
  revisión y timestamps; CRUD autorizado y eventos de cambio. Updates usan
  revisión esperada y devuelven conflicto. Clientes editan/listan mediante store;
  selector inserta texto en el borrador y nunca ejecuta ni envía por seleccionar.
- **Contrato propuesto:** listado paginado/búsqueda, crear/actualizar/eliminar y
  resolución explícita de conflicto; definir alcance host/cuenta y política de
  borrado antes de migrar datos. Variables y sincronización cloud son otra fase.
- **Aceptación:** edición simultánea no pisa contenido silenciosamente; cambiar
  cuenta no filtra prompts y seleccionar conserva el resto del borrador.

## Orden recomendado y dependencias

Esta secuencia reemplaza la sugerencia del catálogo de agrupar cambios muy
heterogéneos en una sola primera fase. Cada entrega debe tener un resultado
visible acotado, checks y e2e propios.

1. **Auditoría de base:** cerrar diferencias reales de 35 y 44, verificar APIs
   reutilizables y registrar versiones fijadas. Sin rediseño de navegación aún.
2. **Tokens y accesibilidad:** 9+16 juntos; después 13 y 14. Medir antes de 12.
3. **Shell de navegación:** 1+2, luego 3 y 11; 6 comparte el controlador de sesión.
   4, 5 y 8 se mantienen opcionales hasta validar ergonomía. 7 es una entrega
   propia y no un efecto lateral de cambiar la barra.
4. **Roster:** 18 y 21; luego 22, 23 y 24. 19 depende de resolver conflictos
   de gestos. Coordinar todo con el plan de constelaciones de esta carpeta.
5. **Chat:** 25→26; 27 y 31; después 28, 33 y 34. Búsqueda 17+30 comparte
   contrato y no se construye dos veces.
6. **Operaciones y sistema:** 36, 37 y 38 como entregas separadas. 20 necesita
   identificar permisos reales; 39 se entrega después del flujo de envío y
   permisos probado. 40 requiere una política local de bloqueo definida.
7. **Voz/pantalla:** 41 y 43, después 44 con 6; prototipo específico para 42.
8. **Host compartido:** 45 y 50 primero; 46 y 47 con contratos acotados; 49
   explícito antes de cualquier automatismo; 48 queda como proyecto separado.

No hay una estimación cerrada de calendario. Las áreas más inciertas son
proxy web, PiP, captura/selección según APIs fijadas, control de energía por SO
y cobertura multiplataforma. Medir esas incertidumbres con prototipos pequeños
antes de comprometer una fecha.

## Instrucción estándar para ejecutar una ficha

Para cualquier número de este documento, la futura tarea debe seguir este orden:

1. Leer la ficha, `AGENTS.md`, arquitectura y componentes existentes. Comprobar
   el estado actual de esa función; esta guía es una revisión fechada.
2. Acordar resultado observable y decisiones pendientes; identificar clientes
   afectados. Registrar si una API ya cubre la necesidad y su compatibilidad.
3. Si hace falta host: implementar primero contrato, autorización, lógica,
   persistencia y pruebas. Añadir fixtures para clientes y documentación del API.
4. Implementar store/modelo y después componentes visuales por cliente. Evitar
   I/O en vistas y duplicar pantallas o métodos que ya existen.
5. Probar éxito, vacío, error, offline, permisos, cuenta cambiada, cancelación
   y concurrencia relevantes. Reproducir al menos la regresión que se corrige.
6. Ejecutar checks adecuados. Tras cambios de código, agentes de limpieza y
   revisión de refactorización en paralelo según las reglas del repositorio;
   aplicar mejoras pertinentes y repetir pruebas afectadas.
7. Detener versiones anteriores, instalar el build nuevo y caminar el flujo
   real. En cambios de host, comprobar que el cliente usa el binario nuevo.
8. Actualizar documentación, capturas y notas visibles; aplicar un único bump
   coordinado antes de `main` cuando corresponda a código distribuido. Nunca
   cambiar `CURRENT_PROJECT_VERSION`. No hacer push al upstream.

## Validación y criterio de cierre

Comandos base desde el directorio correspondiente; ejecutar solo los relevantes
a los archivos realmente implementados:

```text
apps/android:
  .\gradlew.bat :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug
  .\gradlew.bat :app:assembleDebugAndroidTest
apps/desktop:
  npm run typecheck
  npm test
host:
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
```

La instrumentación Android debe usar una instalación desechable, nunca suites
de reset sobre la cuenta personal. iOS necesita Mac/Xcode y pruebas de paquete
y recorrido de Simulator. Desktop se recorre con Computer Use en el sistema
local; cambios grandes, de main o plataforma requieren cobertura de SO según
las reglas e2e y reportar VMs ausentes. Las pruebas unitarias no reemplazan
verificación real de notificaciones, biometría, rutas de audio, cámara o PiP.

Cada entrega debe adjuntar: números implementados; comportamiento observado;
clientes actualizados y excepciones justificadas; comandos y resultados;
capturas de UI con datos ficticios; e2e ejecutado; comprobaciones omitidas y
motivo; riesgos o capacidades aún no soportadas. No marcar una ficha terminada
solo porque compila o porque existe un archivo con el nombre propuesto.

## Referencias oficiales para decisiones dependientes de Android

Las dependencias del proyecto están fijadas; verificar que las APIs elegidas
existan en esas versiones antes de copiarlas de documentación más reciente.
Estas fuentes sustentan las restricciones y enfoques de sus fichas, no afirman
que ya estén implementadas en Codync.

- **S1, adaptación:** [Get started with adaptive apps](https://developer.android.com/develop/adaptive-apps/guides/get-started-with-adaptive-apps).
- **S2, tema:** [Dynamic color](https://developer.android.com/develop/ui/compose/designsystems/material3#dynamic_color_schemes).
- **S3, back:** [Set up predictive back](https://developer.android.com/develop/ui/compose/system/predictive-back-setup).
- **S4, archivos e intents:** [Receive simple data](https://developer.android.com/training/sharing/receive)
  y [Secure file sharing](https://developer.android.com/training/secure-file-sharing).
- **S5, launcher:** [Create shortcuts](https://developer.android.com/develop/ui/views/launch/shortcuts/creating-shortcuts).
- **S6, notificaciones:** [Build a notification](https://developer.android.com/develop/ui/views/notifications/build-notification)
  y [RemoteInput](https://developer.android.com/reference/android/app/RemoteInput).
- **S7, autenticación:** [Show a biometric authentication dialog](https://developer.android.com/identity/sign-in/biometric-auth).
- **S8, PiP:** [Support picture-in-picture](https://developer.android.com/develop/ui/views/picture-in-picture).
- **S9, audio:** [Manage audio focus](https://developer.android.com/media/optimize/audio-focus).
- **S10, clipboard:** [Copy and paste](https://developer.android.com/develop/ui/views/touch-and-input/copy-paste).

## Planes relacionados

- [Constelaciones interactivas por grupo](plan-constelaciones-interactivas-por-grupo.md):
  geometría, agrupación, brillo por actividad y navegación; coordinar ideas 2,
  3, 17–24 y cambios del roster con este plan.
- [Widget de pantalla de bloqueo](plan-widget-bloqueo-android.md): integración
  dependiente del dispositivo y privacidad; no queda incluida automáticamente
  en el rediseño de navegación ni en los accesos del launcher.
- [Avatares de bots](plan-avatares-bots.md): revisar identidad visual compartida
  antes de reemplazar personajes por nuevos indicadores.
