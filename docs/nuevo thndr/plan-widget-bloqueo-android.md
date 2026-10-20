# Plan de integración: widget de pantalla de bloqueo en Android

Fecha: 2026-10-19.
Estado: propuesta; implementación y pruebas pendientes.
Alcance de este documento: planificación. No autoriza ni implica cambios en el código.

## Objetivo y viabilidad

Permitir consultar un resumen de la actividad de Codync desde la pantalla de
bloqueo, con información mínima y acceso a la aplicación después de desbloquear.

La integración es viable en dispositivos cuyo sistema ofrezca un alojamiento
de widgets de terceros en el bloqueo. No se puede garantizar por número de API,
fabricante o versión principal de Android sin comprobar el dispositivo concreto.
El sistema controla la ubicación, la activación y el flujo para añadir widgets.
No se promete una disposición idéntica a la de iOS ni presencia permanente en
Always-on Display.

Google describe la reutilización de widgets convencionales para esta experiencia
y su disponibilidad por defecto salvo exclusión explícita. Por ello, la primera
prueba debe usar los widgets existentes: no asumir que cambiar `home_screen` a
`home_screen|keyguard` habilita por sí solo la función en teléfonos modernos.

Fuente: [Widgets on lock screen: FAQ, Android Developers](https://android-developers.googleblog.com/2025/03/widgets-on-lock-screen-faq.html).
El anuncio no sustituye una verificación actual en el modelo objetivo.

## Base existente inspeccionada

- `apps/android/app/src/main/kotlin/com/codync/android/CodyncWidgets.kt`: cuatro
  widgets Glance — Bots, Team, Provider usage y Usage limits — con tamaños
  adaptables y navegación hacia la aplicación.
- `apps/android/core/src/main/kotlin/com/codync/android/core/WidgetFeed.kt`:
  resumen de bots, uso y marcas de tiempo. No incluye conversaciones ni
  credenciales; sí nombres e identificadores de bots y del equipo.
- `apps/android/app/src/main/kotlin/com/codync/android/WidgetStore.kt`:
  almacenamiento y publicación vinculados a la cuenta activa.
- `apps/android/app/src/main/kotlin/com/codync/android/WidgetRefreshWorker.kt`:
  consultas acotadas al host, restricciones de red y trabajo periódico de
  30 minutos. La ejecución puede retrasarse por decisión del sistema.
- `apps/android/app/src/main/kotlin/com/codync/android/WidgetConfigurationActivity.kt`:
  configuración actual de proveedor y equipo.
- `apps/android/app/src/main/res/xml/widget_*.xml`: metadatos de widgets.
- `apps/ios/Widgets/CodyncWidgets.swift`: referencia funcional; Bots y Usage
  limits ya declaran familias accesorias para el bloqueo de iOS.

Documentación relacionada: [widgets móviles](../design/mobile-widgets.md)
y [Android](../../apps/android/README.md).

## Alcance inicial recomendado

Crear o adaptar una presentación de resumen que muestre:

- Cantidad de bots trabajando.
- Cantidad de bots que requieren atención.
- Antigüedad del último dato válido.
- Entrada a Codync mediante una pulsación, con desbloqueo previo.

Por defecto, omitir nombres, mensajes, identificadores y detalles de equipos.
Los contadores también revelan actividad: explicarlo en la configuración.
Los detalles opcionales y un resumen de consumo quedan para una ampliación
posterior a la validación del primer alcance.

No incluir envío de mensajes, aprobaciones, ejecución de herramientas ni otras
acciones que alteren el host desde el bloqueo. No añadir superposiciones,
servicios permanentes ni mecanismos que sustituyan la pantalla de bloqueo.

## Fase 1: prueba de compatibilidad

1. Registrar modelo, fabricante, versión completa de Android y compilación del
   dispositivo de prueba. Utilizar una instalación de desarrollo y datos ficticios.
2. Comprobar que existe la opción del sistema para widgets de terceros en el
   bloqueo y registrar cómo se activa y si exige carga, base u otra condición.
3. Intentar añadir los widgets actuales sin modificar su código.
4. Revisar dimensiones reales, configuración, pulsaciones y comportamiento
   al bloquear, desbloquear y reiniciar.
5. Documentar los resultados y una lista de combinaciones verificadas.

Salida: decidir entre adaptar un widget existente o añadir un proveedor de
resumen independiente. Si el dispositivo no tiene la capacidad, registrar esa
limitación y conservar los widgets de inicio; no implementar un reemplazo del
bloqueo. Una notificación sería una alternativa de producto con alcance propio.

## Fase 2: presentación y privacidad

La opción preferida para el primer alcance es un proveedor de resumen con
contenido mínimo en cualquier superficie. Esto evita depender de que cada
fabricante comunique de forma fiable que el widget está en el bloqueo.

1. Reutilizar los componentes y datos existentes, separando la presentación
   nueva en un archivo de responsabilidad única.
2. Soportar redimensionado, modo claro y oscuro, escala de fuente y TalkBack.
   Mostrar estados con texto además de color.
3. Definir los textos de configuración y una vista previa con datos ficticios.
4. Si se ofrecen detalles, guardarlos como preferencia por instancia y exigir
   configuración dentro de una actividad protegida por el desbloqueo.
5. Auditar los widgets actuales: podrían ser visibles en el bloqueo y exponer
   nombres. Evaluar una política explícita para ellos. Si procede excluirlos,
   comprobar el uso de `not_keyguard` en recursos específicos de API 36,
   conservando la compatibilidad anterior. Esta decisión debe quedar documentada
   y comprobarse en el dispositivo; no asumir que el nuevo widget protege a los otros.
6. No usar `showWhenLocked` para mostrar conversaciones o configuración privada.

## Fase 3: datos, actualización y navegación

Reutilizar `WidgetFeed`, `WidgetStore` y `WidgetRefreshWorker`. El host conserva
la lógica de bots y consumo; este alcance no necesita nuevos métodos ni cambios
de protocolo. Los componentes visuales no deben abrir conexiones ni hacer I/O.

- Actualizar con los mecanismos ya disponibles y el trabajo periódico existente.
  Inspeccionar los puntos de publicación y actualización antes de añadir nuevos.
- Mostrar la antigüedad usando `botsAt`; si se incorpora uso, tratar `usageAt`
  por separado para no atribuir frescura de bots a los datos de consumo.
- Proponer 60 minutos como umbral inicial de datos antiguos, sujeto a validación
  de producto. No presentar datos guardados como conexión en vivo.
- Revisar cómo avanza el texto de antigüedad cuando no llegan actualizaciones:
  usar una presentación compatible con RemoteViews/Glance o una hora absoluta
  para no dejar congelado un texto relativo engañoso.
- Diferenciar: sin equipo vinculado, sin datos, sin bots, datos guardados,
  actualización fallida y actividad válida. No inferir desconexión solo por edad.
- Si falla una actualización, conservar el último resumen válido con su fecha.
- Al salir de la cuenta o eliminar el vínculo, invalidar y actualizar las
  instancias para que no permanezcan datos de la identidad anterior.
- Revalidar la cuenta y el destino al abrir la app desde un widget guardado.
- Mantener almacenamiento protegido por credenciales. Tras reiniciar y antes
  del primer desbloqueo, no mover datos a almacenamiento de acceso directo
  para hacerlos visibles; comprobar el comportamiento real del sistema.
- No prometer actualización inmediata ni introducir sondeo continuo.

## Fase 4: integración Android y paridad

Archivos candidatos, sujetos al resultado de la prueba inicial:

- Nuevo archivo de presentación y receptor, si se elige proveedor independiente.
- `AndroidManifest.xml` y nuevos metadatos XML del proveedor.
- `CodyncWidgets.kt`: registro, recuento y actualización de instancias.
- `WidgetStore.kt`: únicamente si se añaden preferencias por instancia.
- `WidgetConfigurationActivity.kt` o actividad específica, según responsabilidad.
- Galería y tutorial de widgets Android, recursos de texto y pruebas existentes.

Inspeccionar los equivalentes de iOS, escritorio y terminal antes de cerrar la
implementación. Mantener iguales los significados de actividad, atención y
antigüedad. Android adapta una superficie del sistema; escritorio y terminal
no requieren un widget de bloqueo porque no disponen de esa integración móvil.
iOS ya tiene widgets accesorios y debe revisarse para determinar si la nueva
semántica o política de privacidad exige cambios equivalentes. Si se introduce
un comportamiento compartido nuevo, implementarlo en todos los clientes donde
sea aplicable y justificar expresamente cualquier plataforma sin cambios.

## Fase 5: validación y criterios de aceptación

Pruebas automatizadas propuestas:

- Cálculo del resumen con bots trabajando, pendientes, vacíos, ocultos o eliminados.
- Política de contenido mínimo y detalles opcionales, si se incluyen.
- Datos antiguos, ausencia de marcas de tiempo y fallo de actualización.
- Cambio de cuenta, eliminación del vínculo y respuestas tardías del worker.
- Configuración independiente de varias instancias y eliminación de preferencias.
- Navegación con destino que ya no existe o pertenece a otra cuenta.

Reutilizar `WidgetFeedTest`, `WidgetStoreTest` y `WidgetSystemTest` cuando
corresponda; evitar pruebas que solo repitan la estructura del código.

Validación Android desde `apps/android` con JDK 21 y el SDK requerido:

```powershell
.\gradlew.bat :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug
.\gradlew.bat :app:assembleDebugAndroidTest
```

Ejecutar instrumentación específica en un emulador desechable compatible.
No ejecutar suites que reinicien cuentas o credenciales sobre un teléfono personal.

Para el recorrido real, detener la versión anterior antes de instalar el nuevo
build y verificar:

1. El usuario añade, redimensiona, configura y elimina el widget desde el sistema.
2. El resumen funciona con la app cerrada y distingue información guardada.
3. Pulsar requiere desbloquear antes de acceder a información privada.
4. Cambio de cuenta y cierre de sesión no dejan datos anteriores visibles.
5. Reinicio, pérdida de red y ahorro de batería no producen estados engañosos.
6. Texto grande, TalkBack y modo oscuro siguen siendo utilizables.
7. Los widgets existentes conservan su funcionamiento en la pantalla de inicio.
8. Un dispositivo sin la capacidad conserva una experiencia válida y no recibe
   instrucciones que prometan una opción inexistente.

Guardar capturas con datos ficticios y resultados por dispositivo. Un build
correcto o una vista previa no acreditan el funcionamiento en el bloqueo.
Reportar las comprobaciones que no puedan ejecutarse y su motivo.

Después de modificar código y pasar pruebas, ejecutar en paralelo un agente de
limpieza y otro de revisión de refactorización de solo lectura, según las reglas
del repositorio. Aplicar mejoras pertinentes, repetir pruebas y completar e2e.

## Entrega y dependencias

Orden de trabajo: compatibilidad → decisión de proveedor → privacidad y diseño
→ integración de datos → pruebas y revisión → e2e → documentación de entrega.

Antes de implementar falta identificar el teléfono objetivo y disponer de una
combinación compatible para la prueba real. No hay estimación cerrada hasta
resolver esa dependencia. El riesgo principal es la disponibilidad del sistema;
los siguientes son exposición de información y retrasos de actualización.

Actualizar la documentación Android y de widgets con las combinaciones
verificadas, privacidad y límites de frescura. Para cambios de código distribuido,
aplicar las reglas de versionado de `AGENTS.md` antes de llegar a `main`, sin
duplicar un incremento ya pendiente. Si se modifica iOS, actualizar sus notas
de versión y ejecutar sus validaciones correspondientes.

Este documento no incorpora implementación, cambios de versión ni resultados
de pruebas de dispositivos. Su creación solo requiere revisar el contenido y
confirmar que el diff contiene el nuevo archivo de documentación.
