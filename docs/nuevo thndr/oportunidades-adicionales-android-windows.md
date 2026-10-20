# Oportunidades adicionales: Android y escritorio Windows

Fecha: 2026-10-19. Estado: propuestas de implementación, sin código ni aceptación.
Parte de la [revisión integral](revision-integral-android-windows.md).

## Lectura de las fichas

Veinte oportunidades distintas de trabajo. “Nueva propuesta” significa que no
se encontró una especificación equivalente en la documentación revisada; no
prueba ausencia absoluta de código. “Ampliación” aprovecha una función existente
o una de las cincuenta ideas. “Pendiente documentado” incorpora un hueco conocido.

Viabilidad alta indica solución técnica conocida; media/condicionada requiere
contrato o prototipo. Complejidad se refiere a desarrollo y aceptación conjunta,
no a días. Los nombres de API nuevos son tentativos. En todas las fichas se
debe verificar compatibilidad, autorización y APIs actuales antes de implementar.

No todas deben construirse: la primera prioridad sigue siendo el
[sistema visual Android](especificacion-identidad-visual-android.md). Añadir más
funciones antes de ordenar el shell puede empeorar la experiencia.

## N01. Previsualizar, aplicar y restablecer apariencia

- **Origen:** ampliación de ideas 9/16, que proponen tema pero no un flujo de
  prueba y recuperación. Viabilidad alta; complejidad baja/media.
- **Experiencia:** ver una miniatura representativa de chat, formulario y estados
  al probar modo/acento; Aplicar guarda y Cancelar restaura la apariencia anterior.
- **Cómo:** separar preferencia persistida de selección temporal, pasando ambas
  por el mismo resolvedor del tema. Restablecer afecta solo apariencia local.
  Windows ofrece el mismo propósito con vista previa desktop, no con otra copia
  de la pantalla móvil. No hace falta API de host.
- **Aceptación:** cambiar el tema del SO durante el preview no guarda cambios
  accidentales; volver o recrear actividad conserva una política definida.

## N02. Tema de contraste y foco claramente visible

- **Origen:** ampliación de tema/accesibilidad, no otra paleta decorativa.
  Viabilidad alta; complejidad media.
- **Experiencia:** controles, selección y errores legibles en Android y al usar
  contraste del sistema en Windows, sin perder foco de teclado en overlays.
- **Cómo:** tokens semánticos de contraste, revisión de focus ring y estados,
  detección de preferencias mediante la capa SO de cada cliente; en Electron,
  comprobar `nativeTheme` y `forced-colors`. No desactivar correcciones del SO
  para conservar un color de marca. Terminal respeta capacidades de su paleta.
- **Aceptación:** operar roster, chat y permiso completo con teclado/Narrator o
  TalkBack, claro/oscuro y contraste, sin depender de halo o color.

## N03. Idioma de interfaz y vocabulario coherente

- **Origen:** nueva propuesta de producto transversal; se observaron cadenas
  en inglés y etiquetas en español en la constelación. No es traducir mensajes.
  Viabilidad alta; complejidad media/alta por cobertura.
- **Experiencia:** selector Sistema/Español/Inglés, según recursos realmente
  disponibles, con Bots, Actividad, Equipos y Grupos usados con significado estable.
- **Cómo:** extraer cadenas de UI, usar plurales y formatos de fecha/número;
  configurar idiomas por app en Android y un catálogo equivalente en escritorio.
  Mantener códigos de error del host separados del texto localizado; no traducir
  nombres de archivo, comandos o respuestas del agente.
- **Aceptación:** no hay frases mezcladas en el flujo piloto; cambiar idioma
  conserva borrador/contexto y soporta cadenas largas y navegación accesible.

## N04. Paleta contextual de acciones

- **Origen:** nueva ampliación funcional; distinta de buscar historial (17/30)
  y de atajos de formato (27). Reutilizar `SearchPalette` donde corresponda.
  Viabilidad alta; complejidad media.
- **Experiencia:** encontrar “Crear bot”, “Ver errores”, “Abrir archivos”,
  “Conectar equipo” o “Editar grupo” sin memorizar dónde está cada menú.
- **Cómo:** registro tipado de comandos con contexto, disponibilidad y callback;
  acceso táctil en Android y atajo configurable/no conflictivo en escritorio.
  No ejecutar texto escrito como shell ni iniciar acciones destructivas al buscar.
- **Aceptación:** resultados solo muestran acciones aplicables; una acción
  confirma destino/permisos y no se ejecuta dos veces al pulsar Enter.

## N05. Teclado, IME y foco como flujo completo

- **Origen:** ampliación de layout adaptable y aceptación física pendiente;
  no repetir la función de adjuntar teclado ya documentada. Viabilidad alta;
  complejidad media.
- **Experiencia:** escribir, buscar, crear bot y cambiar entre paneles sin
  perder foco al cerrar una hoja o conectar teclado al teléfono/tablet.
- **Cómo:** contrato común de Tab, Escape/atrás, enviar/salto de línea, foco
  inicial y retorno. Validar composición CJK, selección y teclado virtual en
  Android; equivalentes Ctrl/Alt/Enter en Windows sin capturar atajos del SO.
- **Aceptación:** recorrido sin ratón y texto compuesto sin envío prematuro;
  abrir detalle de una estrella retorna foco al mismo bot al cerrar.

## N06. Centro de envíos pendientes y recuperación

- **Origen:** ampliación de la cola actual; nueva superficie, no nuevo sistema
  de envío. Viabilidad alta; complejidad media.
- **Experiencia:** ver Esperando equipo, Enviando, Entrega en curso, Error y
  Enviado, con acceso a conversación y reintento/cancelación válidos.
- **Cómo:** presentar `PendingSend` y estados existentes por cuenta/equipo;
  mantener nonce al reintentar. Cancelar una entrega exige consultar el resultado
  real; no ofrecer Editar cuando pueda estar procesándose. Escritorio aplica
  estados equivalentes con su store; no sincronizar borradores sin contrato nuevo.
- **Aceptación:** reinicio y reconexión no duplican mensajes; contenido con
  adjuntos no se promete entregable por el mailbox de texto.

## N07. Diagnóstico guiado y recuperación de conexión

- **Origen:** ampliación de guías de conexión y del plan de reporte de errores;
  no se cuenta como descubrimiento de una función nunca documentada.
  Viabilidad alta; complejidad media/alta.
- **Experiencia:** explicar si falla cuenta, autorización, host, canal, versión,
  proveedor o media y mostrar la próxima acción posible en teléfono o PC.
- **Cómo:** diagnósticos tipados y no secretos desde host/transporte, con fecha
  de observación y capacidad; UI presenta pasos y reintentos acotados. Un reporte
  exportable se previsualiza y requiere envío explícito según el plan existente.
  No volcar tokens, logs completos o contenido de conversaciones automáticamente.
- **Aceptación:** se distingue host apagado de falta de acceso; el teléfono no
  afirma que puede resolver un permiso del SO que exige interacción en la PC.

## N08. Modo de datos limitados y descarga consciente

- **Origen:** nueva política de dispositivo, distinta de mostrar salud de PC
  (47) y de cambiar el transporte cifrado. Viabilidad alta para descargas;
  condicionada para calidad de vídeo. Complejidad media.
- **Experiencia:** en red medida, no precargar archivos grandes y ofrecer
  continuar; mostrar qué contenido está guardado para lectura offline.
- **Cómo:** política local de prefetch/caché con cuota y limpieza separada de
  borradores. Conservar límites del host; para calidad de pantalla, negociar
  perfiles solo si helper/contrato lo soportan. No mantener sockets en fondo
  saltándose las restricciones Android.
- **Aceptación:** cambiar Wi-Fi/datos no corrompe transferencias; limpieza de
  caché nunca borra un archivo pendiente de enviar ni una copia guardada por el usuario.

## N09. Biblioteca de entregables y centro de transferencias

- **Origen:** ampliación de snapshots de `send_file`, ideas 45/49 y progreso
  existente. Viabilidad alta; complejidad media/alta.
- **Experiencia:** encontrar archivos generados por bots desde Android y PC,
  saber de qué mensaje proceden y distinguir remoto, descargando y guardado.
- **Cómo:** reutilizar referencias de archivos inmutables, tamaño y SHA-256;
  si falta listado global, añadir consulta paginada al host por bot/proyecto.
  La ubicación local y cola de descarga pertenecen a cada dispositivo. Descarga
  reanudable exige contrato propio; no anunciarla por existir chunks.
- **Aceptación:** integridad se verifica antes de exportar; cambiar cuenta cancela
  transferencias y no presenta archivos privados anteriores.

## N10. Preferencias de interrupción por conversación y tipo

- **Origen:** ampliación de notificaciones existentes; distinta de RemoteInput
  y aprobar desde una notificación (39). Viabilidad alta; complejidad media/alta.
- **Experiencia:** separar respuestas, tareas terminadas, errores y solicitudes
  de atención; silenciar una conversación sin ocultarla del panel de pendientes.
- **Cómo:** auditar canales y preferencias `notify` actuales. Separar decisión
  de publicar push (host) de presentación y permisos del SO (dispositivo).
  Respetar elección de canales y modo No molestar; preferencias compartidas
  necesitan contrato explícito por alcance. No prometer sonido si el SO lo bloquea.
- **Aceptación:** un bot silenciado sigue accesible y conserva pendientes;
  cambiar ajustes en Android no rompe los de Windows ni burla canales del sistema.

## N11. Privacidad visual configurable

- **Origen:** nueva propuesta de dispositivo; complementa biometría (40) y
  privacidad del widget, sin sustituir autorización del host. Viabilidad alta
  con límites del SO; complejidad media.
- **Experiencia:** ocultar contenido en recientes, elegir previews de notificación
  y activar una pantalla discreta al perder foco si el usuario lo desea.
- **Cómo:** aplicar mecanismos Android apropiados por superficie, revisar
  capturas y compartir para no bloquearlos accidentalmente; en Windows,
  diferenciar contenido de la ventana y vista previa del SO. Preferencias locales,
  fallback explícito y sin afirmar protección universal contra capturas.
- **Aceptación:** notificación, widget, recientes y cambio de cuenta se prueban
  por separado; ocultar preview no elimina mensajes ni invalida el chat.

## N12. Bandeja compartida de decisiones pendientes

- **Origen:** ampliación de 20/39 y del estado de aprobaciones de desktop/island.
  Viabilidad alta; complejidad media/alta.
- **Experiencia:** abrir Android y encontrar lo que requiere decisión en la PC:
  permiso, credencial, confirmación o intervención, con contexto y destino.
- **Cómo:** empezar por permisos estructurados ya disponibles; representar cada
  petición por ID, opciones válidas y revisión. Nuevas clases de decisión deben
  tener contrato host propio. Resolver una en Windows actualiza el teléfono;
  rechazar una petición nunca se convierte en detener todos los bots.
- **Aceptación:** vencida/resuelta por otro cliente se retira de forma consistente;
  sin contexto suficiente abre detalle en lugar de ofrecer aprobación ciega.

## N13. Mapa real de colaboración entre bots

- **Origen:** nueva visualización sobre colaboración existente; distinta de
  unir estrellas para formar una silueta de grupo. Viabilidad alta con datos
  estructurados; complejidad media/alta.
- **Experiencia:** ver quién pidió trabajo a quién, qué intercambio espera y
  cuáles terminaron/fallaron, desde Android y escritorio.
- **Cómo:** consumir `botMessage`, IDs y estados de intercambios del host;
  añadir listado agregado acotado si el actual endpoint por pareja no basta.
  Capa “Colaboración” diferenciada de líneas decorativas de Constelación;
  flechas representan mensajes reales, no relaciones jerárquicas inventadas.
- **Aceptación:** no se dibuja actividad inexistente; pulsar arista abre historia
  de esa pareja, filtrada/autorizada, y reinicio conserva el estado interrumpido real.

## N14. Marcadores y notas personales sobre mensajes

- **Origen:** nueva propuesta de contenido sincronizado; distinta de biblioteca
  de prompts (50), memoria de bot y reacciones emoji. Viabilidad alta;
  complejidad media/alta.
- **Experiencia:** guardar una respuesta importante en la PC y recuperarla en
  Android con una nota privada y enlace al hilo original.
- **Cómo:** modelo del host con identidad de entrada, propietario, nota y revisión;
  CRUD autorizado, listado paginado y eventos. No copiar la conversación completa
  a otra base de clientes. Definir política para bot eliminado o entrada inaccesible.
- **Aceptación:** edición concurrente muestra conflicto; contenido de otra cuenta
  no aparece; marcador huérfano no abre un bot con ID parecido.

## N15. Galería de perfiles y creación guiada de trabajadores

- **Origen:** ampliación de Create template de escritorio y hueco Android
  documentado. No afirmar que el sistema carece de plantillas.
  Viabilidad alta; complejidad media/alta.
- **Experiencia:** crear Asistente, Gestor o Revisor mostrando instrucciones,
  agente, recursos requeridos y capacidades antes de confirmar.
- **Cómo:** reutilizar serialización de plantillas existente y validación host;
  distinguir perfil de instrucciones de rol organizativo formal. No exportar
  credenciales ni cwd personal literal a otro equipo; resolver dependencias
  e instanciar workspace propio. Android añade consumo/edición equivalente.
- **Aceptación:** una plantilla incompleta explica qué falta; no instala ni
  conecta servicios o concede permisos por elegir una tarjeta de perfil.

## N16. Aviso de trabajo simultáneo sobre el mismo proyecto

- **Origen:** nueva propuesta sobre límite explícito de `bot-workspaces.md`:
  workspaces separados no impiden editar otro mismo proyecto. Viabilidad media;
  complejidad alta. Es aviso, no aislamiento garantizado.
- **Experiencia:** advertir en PC y Android que varios bots declaran trabajo
  sobre un workspace/proyecto compartido y facilitar ver participantes.
- **Cómo:** identidad canónica de workspace en host y sesiones activas declaradas;
  no deducir archivos exactos desde conversación. Para bloqueo/worktrees reales,
  diseñar otro contrato, merge y recuperación; no construirlo dentro del cliente.
- **Aceptación:** rutas equivalentes no producen proyectos duplicados; actividad
  desconocida se etiqueta como tal y la advertencia no promete eliminar conflictos.

## N17. Ver y controlar Windows desde Android

- **Origen:** pendiente de plataforma documentado; nuevo helper requerido.
  Viabilidad condicionada a prototipo; complejidad muy alta.
- **Experiencia:** visor Android reutilizable para el host Windows, con selección
  de monitor, solo lectura/tomar control y estado de sesión claro.
- **Cómo:** helper Windows de captura, codificación, media/input y lifecycle,
  compatible con señalización y autorización existentes. Evaluar API de captura
  Windows y política de consentimiento con el paquete real; mantener entradas
  en el contexto permitido. No confundir con driver computer-use de bots.
- **Límites:** escritorio seguro/UAC, ventanas elevadas, contenido protegido,
  bloqueo, monitores/DPI y sesión de usuario necesitan comportamiento explícito.
  No resolverlos elevando silenciosamente el host.
- **Aceptación:** vídeo y coordenadas correctos en dos escalas DPI; revocación,
  cierre, bloqueo y red perdida cortan entrada/media; probar TURN y reconexión.
  El cambio incluye packaging y pruebas Windows reales. Ver referencia W3.

## N18. Backend SSH Windows con diagnóstico claro

- **Origen:** pendiente de arquitectura, no una necesidad de SSH dentro de Android.
  Viabilidad alta en principio; complejidad alta por proceso/credenciales.
- **Experiencia:** conectar un host remoto desde la app Windows sin instrucciones
  que dependan de rutas Unix; el teléfono conoce sus propios equipos autorizados.
- **Cómo:** abstraer invocación de OpenSSH/ssh-agent y ciclo del túnel en desktop
  main, verificar binario instalado, host keys, reconexión y procesos ocultos.
  Mantener secretos fuera de renderer y logs. Android no hereda túneles de desktop
  ni convierte perfil SSH en grant remoto automáticamente.
- **Aceptación:** clave desconocida/modificada exige decisión correcta; desconectar
  mata solo su túnel, y reinicio/update no deja procesos huérfanos ni consolas visibles.

## N19. Varios equipos en Android

- **Origen:** pendiente conocido; arquitectura prioriza un equipo. Requiere
  decisión de producto adicional, no activarlo por descubrir stores agregados.
  Viabilidad alta arquitectónicamente; complejidad alta.
- **Experiencia:** alternar PC Windows y otro host con identidad y estado claros,
  después considerar una vista agregada con permisos por origen.
- **Cómo:** comenzar por selección de un equipo activo y contextos separados;
  referencia cuenta+equipo+bot en navegación, borradores, archivos, push y widgets.
  Reutilizar arquitectura de cuenta/iOS donde proceda. No usar nombres o botId
  solos como identidad global ni abrir todas las conexiones por defecto.
- **Aceptación:** dos hosts con nombres/IDs parecidos no mezclan mensajes;
  cambiar uno offline no bloquea el otro y cada acceso se autoriza por host.

## N20. Resultados de trabajo estructurados y verificables

- **Origen:** nueva propuesta más allá de trace/diff (26/32/45), con datos
  compartidos. Viabilidad condicionada; complejidad alta.
- **Experiencia:** una tarjeta resume qué entregó un bot, archivos publicados,
  checks observados y pendientes; se consulta en Android y escritorio.
- **Cómo:** empezar enlazando mensaje final, archivos snapshot y eventos de
  herramientas existentes. Para resultados formales, añadir esquema versionado
  al host con origen de evidencia y estado desconocido. Diferenciar lo que
  afirma el bot de un resultado realmente observado por el host o una integración.
  No analizar texto libre y convertir “tests passed” en verificación certificada.
- **Aceptación:** cada resultado tiene referencia consultable; check sin evidencia
  se muestra no verificado; repetir una consulta no vuelve a ejecutar comandos.

## Propuesta de priorización

- **Primera entrega visual:** resolvedor y componentes Android del documento
  de identidad, con N01 y N02 en la pantalla piloto.
- **Experiencia diaria:** N03 y N05, después N04 si la nueva navegación aún
  necesita acceso rápido. Una paleta no sustituye una buena jerarquía.
- **Confianza móvil + Windows:** N06/N07, luego N09/N12. El host sigue siendo
  autoridad; los clientes muestran estados coherentes.
- **Productividad compartida:** N13/N14/N15 según necesidad real. N13 conecta
  particularmente bien con la idea de trabajadores y constelaciones.
- **Opciones de dispositivo:** N08/N10/N11 sin forzar la misma preferencia en
  teléfono y PC.
- **Investigación/proyecto independiente:** N16–N20. N17 y N18 requieren
  trabajo de plataforma y no deben retrasar el rediseño del tema.

## Ejemplo de función complementaria recomendada

Caso: un bot de Windows trabaja, pide revisión a otro y publica un archivo.

1. Windows muestra progreso, colaboración y archivo en el chat existente.
2. Android presenta la constelación del grupo; el halo identifica actividad
   reportada y la capa de colaboración identifica la petición real (N13).
3. Si llega un permiso, aparece en la bandeja (N12). El usuario ve contexto
   antes de resolverlo; la resolución se refleja también en Windows.
4. El archivo generado aparece como entregable (N09) con origen e integridad.
5. Una respuesta útil puede guardarse como marcador (N14) para ambos clientes.

Este flujo no exige transmitir pantalla Windows. N17 sería una ampliación
independiente para intervenir visualmente cuando sea necesario.

## Contratos y validación comunes

Para cada ficha: inspeccionar APIs reales; definir qué es local y qué es host;
implementar lógica en host si falta; añadir contratos aditivos y compatibilidad;
integrar stores y UI; comprobar equivalentes funcionales iOS/terminal.
No crear un renderer de escritorio separado para Windows.

Pruebas mínimas: estado inicial/vacío/error/offline, permiso insuficiente,
cambio de cuenta/equipo, cancelación y eventos tardíos. Añadir concurrencia para
mutaciones sincronizadas y muerte de proceso para estado local persistido.
La cola, autorización y archivos deben cubrir reintentos reales, no solo mocks.

Después de cambiar código, checks por cliente, agentes de limpieza/refactor
requeridos y e2e en builds recién instalados con versiones anteriores detenidas.
Cambios Windows de main, helper o packaging requieren pruebas de plataforma;
reportar otras plataformas/VMs no disponibles, sin declarar aceptación completa.
Fuente y límites: [guía de ejecución anterior](guia-implementacion-50-mejoras-android-escritorio.md#instrucción-estándar-para-ejecutar-una-ficha).

Cada entrega describe qué números aborda y qué no, captura la UI con datos
ficticios, actualiza documentación vigente y aplica versionado cuando corresponda
a código distribuido. Crear este documento no requiere release ni implica
activación de ninguna función.

## Referencias y fuentes

Locales, respecto a `docs/`:

- [Arquitectura y ownership](../architecture/overview.md).
- [Escritorio y límites Windows](../architecture/desktop-app.md#platform-notes).
- [Colaboración estructurada](../features/bot-collaboration.md).
- [Grupos y threads](../features/groups-and-threads.md).
- [Workspaces y límites de aislamiento](../features/bot-workspaces.md).
- [Archivos y snapshots](../features/file-attachments.md).
- [Computer use](../features/computer-use.md) y [Remote screen](../features/remote-screen.md).
- [Notch island y atención](../features/notch-island.md).
- [Reporte de problemas ya propuesto](../plans/error-tracking-detail.md).
- [Paridad Android y aceptación pendiente](../plans/android-app.md).

Oficiales consultadas para restricciones del SO:

- **A1:** [Idiomas por aplicación Android](https://developer.android.com/guide/topics/resources/app-languages).
  Configurar selección no traduce recursos automáticamente; revisar versiones fijadas.
- **A2:** [Canales de notificaciones](https://developer.android.com/develop/ui/compose/notifications/channels).
  La presentación depende también de los ajustes elegidos por el usuario en el sistema.
- **W1:** [Electron nativeTheme](https://www.electronjs.org/docs/latest/api/native-theme).
- **W2:** [Windows contrast themes](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/high-contrast-themes).
- **W3:** [Windows screen capture](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture).
  Referencia de tecnología candidata; no demuestra que el helper de Codync esté construido.

Esta guía propone soluciones a partir de documentación y código inspeccionados;
no certifica soporte por API o dispositivo antes de ejecutar los prototipos.
