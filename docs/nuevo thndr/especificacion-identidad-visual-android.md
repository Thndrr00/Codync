# Especificación propuesta: identidad visual propia para Android

Fecha: 2026-10-19. Estado: dirección técnica y de UX propuesta; sin código ni
maquetas verificadas. Forma parte de la [revisión integral](revision-integral-android-windows.md).
Consolida ideas 1–16 del catálogo, no las vuelve a contar como nuevas funciones.

## Resultado deseado

Una aplicación que se sienta Android: navegación clara, jerarquía tipográfica,
superficies tonales estables, controles alcanzables y respuesta coherente al
botón atrás, teclado y tamaño de ventana. Mantener identidad de Codync mediante
avatares, estados y vocabulario, adaptando la forma de presentarlos.

El cambio principal propuesto es dejar el cristal como material dominante y
usar superficies sólidas o tonales en navegación, formularios y compositor.
Evitar cápsulas gigantes alrededor de cada título y hojas a pantalla completa
para cualquier ajuste. No convertir todas las pantallas en un panel de tarjetas.

## Restricciones de la propuesta

La petición del usuario fija Android como dirección de producto. Se implementa
evolucionando los componentes del módulo `design`, conforme a `AGENTS.md`.
No implica importar todos los controles Material ni copiar interfaces de otra app.
Conservar reglas actuales: tokens comunes, Motion, controles compartidos y
superficies rellenas sin un borde extra. Flujos de autenticación y permisos del
SO siguen usando sus integraciones nativas.

La paridad de funciones no obliga a dibujar Android, iOS y Windows igual.
iOS conserva sus affordances y excepciones de navegación nativa; Windows usa
puntero, teclado y ventanas. Documentar cualquier excepción de capacidad.

## 1. Un único resolvedor de apariencia

Crear un modelo tipado, de responsabilidad única, con estas dimensiones:

- Modo: Sistema, Claro, Oscuro o Negro OLED.
- Acento: Codync por defecto, dinámico del dispositivo donde esté soportado o
  uno de pocos acentos revisados. Evitar un selector RGB libre en la primera fase.
- Material: tonal como dirección propuesta; no ofrecer tres variantes antes
  de validar una. La ruta anterior puede servir temporalmente para rollback.
- Movimiento: obedecer reducción del sistema; la app puede reducir más, nunca
  forzar animación continua contra esa preferencia.

Orden de resolución: necesidades de accesibilidad del sistema → preferencia
de modo → paleta/acento disponible → ajustes contextuales del bot. Error,
advertencia, atención y éxito no se convierten en variaciones arbitrarias del acento.

Guardar preferencias de apariencia por instalación. No sincronizar modo oscuro
entre teléfono y Windows por defecto: cada pantalla tiene ambiente y SO distintos.
Si más adelante se comparten presets, eso será una acción explícita del usuario.

## 2. Migración de tokens

1. Inventariar usos de `Palette`, `MaterialTheme.colorScheme`, colores literales
   y `isSystemInDarkTheme()`. Incluir `botMessageBackground`, barras del sistema,
   hojas, menús, visor de código y previews.
2. Definir un conjunto resuelto de colores semánticos: fondo, superficie,
   superficie elevada, selección, entrada, texto principal/secundario,
   acento/onAccent, atención, error, positivo y código/diff.
3. Hacer que `Palette` y el MaterialTheme raíz lean el mismo conjunto. No romper
   las llamadas actuales en una migración masiva innecesaria.
4. Sustituir consultas dispersas al tema del sistema por modo efectivo del tema
   local cuando la apariencia deba obedecer el selector manual.
5. Resolver color dinámico solo donde la API lo soporte, con paleta fija de
   fallback. Material You no convierte por sí solo el diseño en Android.
6. Mantener tipografía en sp y escala del sistema; no aplicar la misma escala
   dos veces a texto, iconos y contenedores.

La API concreta y las dependencias deben verificarse contra las versiones
fijadas del proyecto. Fuente: [Material 3: dynamic color](https://developer.android.com/develop/ui/compose/designsystems/material3#dynamic_color_schemes).

## 3. Forma, espacio y tipografía

- Usar una escala pequeña y consistente de radios; reservar círculo para avatar
  y controles que lo justifiquen, no para encerrar todo el chrome.
- Propuesta inicial de ritmo: unidades de 4/8 dp y márgenes de pantalla de 16 dp,
  ajustables tras prototipo; son valores de trabajo, no reglas ya aceptadas.
- Área táctil mínima propuesta de 48 dp aunque el icono sea más pequeño.
- Títulos alineados con contenido y acciones principales fácilmente reconocibles.
  No centrar todos los encabezados por herencia del iPhone.
- Inter puede conservarse: cambiar de fuente no es requisito para diferenciar
  plataformas. Usar jerarquía de tamaños/pesos y probar nombres largos.
- Formularios por secciones de contenido, con etiquetas persistentes y errores
  junto al campo; no depender exclusivamente de placeholders o menús ocultos.
- En lectura larga, priorizar columna legible y separación de mensajes;
  en escritorio ancho no estirar cada línea de conversación a toda la ventana.

## 4. Navegación e información

Propuesta de raíz: Chats, Actividad, Extensiones y Ajustes. Definir Actividad
como resumen operativo de bots, permisos y rutinas; State/widgets/uso deben
tener una ubicación explícita antes de quitar el destino actual.

- Una raíz con historial independiente por destino; selección guardada y rutas
  tipadas. Enlaces a bot/hilo no deben terminar en una pestaña incoherente.
- Barra inferior estable en pantalla compacta. El gesto de ocultarla y el pager
  global son opcionales posteriores, no requisitos para lograr identidad Android.
- En ventanas amplias, rail y lista/detalle con el mismo store, sin duplicar
  sesiones. Usar tamaño de ventana/postura, no solo un umbral de orientación.
- Crear bot/grupo desde una acción inferior alcanzable reutilizando los callbacks
  actuales. No llenar la pantalla de acciones flotantes competidoras.
- Atrás cierra primero menú, luego hoja/detalle y después navega por la pila.
  Si hay borrador o edición, conservar las reglas de recuperación existentes.

Fuentes: [Adaptive apps](https://developer.android.com/develop/adaptive-apps/guides/get-started-with-adaptive-apps)
y [Predictive back](https://developer.android.com/develop/ui/compose/system/predictive-back-setup).

## 5. Pantallas y controles a migrar

### Roster y Constelación

Cabecera simple con equipo/contexto, resumen y acceso a buscar. Lista y
Constelación comparten filtros y acciones. Actividad identificada por texto e
icono además de halo/color; no convertir todos los avatares en adornos móviles.
Respetar el plan de constelaciones por grupo para geometría y estados.

### Conversación

Cabecera de atrás e identidad con acciones secundarias agrupadas; compositor
con superficie tonal, adjuntos y estados claros de envío. Código y archivos
usan componentes especializados y no heredan indiscriminadamente burbujas.
No cambiar a streaming de burbujas: la política actual muestra mensajes finales.

### Formularios e inspector

Ajustes cortos en una hoja de contenido adecuado; edición extensa en pantalla
con navegación. No aplicar media altura cuando teclado y validación necesiten
más espacio. Mantener foco, errores, cancelación y una estrategia de guardado
explícita según el editor existente.

### Extensiones y conexión

Separar catálogo, instalado, autenticado y disponible para usar. Cada problema
debe ofrecer una próxima acción concreta: iniciar sesión, instalar dependencia,
volver a comprobar o abrir instrucciones en la PC. No usar un mismo mensaje
“No instalado” para condiciones diferentes.

### Widgets, notificaciones y elementos del SO

Conservar sus contenedores y limitaciones nativas. Los widgets resuelven su
apariencia por el mecanismo que permita Glance/RemoteViews, no por composición
de pantalla. Si usan preferencias de la app, forzar actualización de instancias
al cambiarlas y validar proceso muerto/launcher. No prometer cristal ni animación
de pantalla completa en widgets.

## 6. Estados y feedback

Cada pantalla deberá definir: primera carga, contenido guardado, vacío real,
vacío por filtro, error recuperable, falta de autorización y capacidad no soportada.
Mantener contenido válido mientras se refresca; no reemplazar todo por un spinner.

Éxito simple puede usar feedback breve con acción pertinente. Errores persistentes
se quedan junto a la operación hasta resolverlos. Deshacer se ofrece solo para
acciones realmente reversibles; no fingir deshacer ejecución de una herramienta.
Una operación remota sigue pendiente hasta confirmación del host.

## 7. Windows: coherencia sin copiar Android

Reutilizar semántica de tokens en `renderer/styles/theme.css`, pero conservar
la escala de interfaz desktop. El selector puede compartir nombres conceptuales
y el estado efectivo debe actualizar main y renderer, con cambios de `nativeTheme`
expuestos por un puente acotado. No usar Node directamente desde React.

- Integrar contraste del sistema, foco de teclado y selección legibles.
- Comprobar barra de título, áreas arrastrables, menús, inspector y escalado
  de Windows por monitor. El sistema tray sigue siendo menú nativo.
- Alto contraste requiere estilos compatibles con el renderer web, no copiar
  recursos XAML de WinUI. Verificar `forced-colors` y las APIs de la versión
  Electron fijada, sin desactivar globalmente el ajuste de color del sistema.
- Preferencias se guardan localmente y no fuerzan el modo del teléfono.

Fuentes: [Electron nativeTheme](https://www.electronjs.org/docs/latest/api/native-theme)
y [Windows contrast themes](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/high-contrast-themes).

## 8. Plan incremental de implementación

1. Capturar pantallas actuales con fixtures y registrar temas, tamaño de fuente
   y estados. Son el baseline real, no una imagen antigua del README.
2. Implementar resolvedor/preferencia y pruebas de fallback; mantener el tema
   actual como default durante la migración técnica.
3. Migrar una pantalla piloto de Ajustes/Apariencia para validar tokens y
   controles; preview de cambios sin alterar datos del host.
4. Migrar chrome/navegación y roster; validar enlaces, teclado y Constelación.
5. Migrar chat/compositor, luego formularios, inspector y extensiones.
6. Resolver las superficies independientes: notificaciones, widgets y barras
   del sistema. Comprobar que no queden colores del sistema mezclados con modo manual.
7. Completar equivalentes funcionales de otros clientes y contrastes Windows.
8. Retirar rutas antiguas solo cuando el conjunto sea consistente, revisar
   documentación de “port 1:1” y publicar según versionado del repositorio.

No introducir en la misma entrega proxy web, pantalla Windows o multi-equipo.
El rollback del tema debe cambiar solo presentación, no borrar preferencias
de cuenta, cachés, borradores ni datos compartidos.

## 9. Aceptación y límites

- Modo manual aplicado a chat, formularios, menús y compositor con el SO en el
  modo contrario; sin mezcla visible ni destello de tema al recrear la actividad.
- Texto ampliado, TalkBack, teclado, reducción de movimiento y orientación
  conservan acceso a todas las funciones. Fuente: [Android scalable content](https://developer.android.com/develop/ui/compose/accessibility/scalable-content).
- Luz/oscuro/negro y fallback dinámico legibles; estados no dependen solo de color.
- Navegar atrás, abrir widget/notificación y cambiar de bot conserva contexto.
- Medir frames en la misma carga y dispositivo antes/después; no prometer ahorro
  de batería ni 120 Hz por quitar blur sin medir.
- Windows: teclado, Narrator y contraste del sistema, con escala de pantalla y
  más de un monitor cuando esté disponible.
- Checks y e2e de clientes afectados conforme a `AGENTS.md`, más las revisiones
  de limpieza/refactor exigidas después de cambiar código.

La elección final de acento, radios y composición necesita prototipo y validación
visual; este documento establece una dirección, no afirma que esas decisiones
ya hayan sido probadas con usuarios.
