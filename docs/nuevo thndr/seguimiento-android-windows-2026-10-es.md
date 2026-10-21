# Seguimiento de integraciones del fork: Android y Windows

Fecha: 20 de octubre de 2026. Base: `774da120`.

Este seguimiento actualiza la [guía de revisión](../integrations/guia-integraciones-upstream-2026-10-es.md).
El alcance solicitado es la app Android existente que incluye Constelación,
el escritorio Windows y su host compartido. No se crea otra aplicación ni se
modifican las aplicaciones o auxiliares de macOS/iOS.

## Correcciones aplicadas

- **P-01, publicación del fork:** Auto Tag escucha los manifiestos del host y
  escritorio y su propio workflow. Lee la versión del paquete Rust y exige que
  coincida con escritorio antes de etiquetar. Así Windows no depende de un
  proyecto Xcode regenerado para disparar su release. La reparación del proyecto
  Apple queda fuera del alcance. No se creó ningún tag ni release durante esta
  intervención.
- **P-02, modelos Android:** el selector del editor conserva Default y el modelo
  guardado al buscar, manteniendo sus etiquetas y sin duplicar resultados. Busca
  por nombre, ID y descripción, sin distinguir mayúsculas y recortando espacios.
  Los IDs manuales sobreviven a cambios del catálogo. La lógica de presentación
  está en `core/ModelSearch.kt` y la utiliza el selector Compose existente.
- **P-03, dependencias VNC:** `npm.cmd ci` instala el lockfile existente; las
  pruebas del puente ejecutan sus aserciones. No se actualizan dependencias.
- **P-04, once fallos del actualizador:** las fixtures mezclaban una plataforma
  simulada con `node:path` de la máquina ejecutora. Ahora usan `path.posix` para
  la simulación macOS y `path.win32` para Windows, incluidas las rutas al binario,
  los datos y el token. No se omiten pruebas ni se modifica el actualizador real.
- **P-05, referencia de pantalla:** el resumen de `CLAUDE.md` remite al protocolo
  de trickle ICE negociado con fallback completo y a la documentación detallada.

## Dónde están las funciones integradas

Las diez incorporaciones ya forman parte de esta base del fork. Constelación
es una presentación del mismo roster Android: abrir un bot llega al mismo chat,
perfil y editor que la vista Lista. No necesita duplicar agentes, cuentas ni
conexiones.

- **Modelos y cuentas:** Android usa `BotEditor.kt`, `AgentModelPicker.kt`,
  `AgentAccountPicker.kt` y `AppStoreBots.kt`. Windows usa `BotEditorView.tsx`,
  `AgentModelPicker.tsx` y `AgentAccountPicker.tsx`. Ambos consultan al host;
  cambiar la cuenta invalida el catálogo y reinicia las opciones del modelo.
- **Hermes ACP:** lo registra e inicia `host/src/agent/backends.rs`. Se ofrece
  mediante el catálogo compartido, también al editar bots desde Android.
  Requiere instalar y autenticar Hermes en el ordenador que ejecuta el host.
- **Rutinas y no leídos:** el host etiqueta el origen del resultado y calcula
  los contadores; Lista, Constelación y escritorio consumen ese mismo estado.
- **CLI send:** está en el host, no requiere copiar una terminal dentro de la
  app Android. Utiliza los mismos bots y conversaciones persistidos.
- **Procesos y sesiones ACP:** pertenecen al host, incluidas la asignación,
  reanudación y terminación de procesos Windows; los clientes envían intenciones.
- **Voz compatible:** los ajustes y motores existentes de Android/escritorio
  usan la ruta Speech del host. `CODYNC_OPENAI_BASE_URL` configura el proceso
  host. Realtime sigue sin admitirse con un endpoint personalizado.
- **Trickle ICE:** Android usa `ScreenSignaling.kt` y las suscripciones del
  canal cifrado. Windows aún no ofrece un auxiliar nativo de captura WebRTC:
  el visor SSH/VNC es otro transporte y no elimina esa limitación.
- **SSH Windows y visor VNC:** están en el proceso principal de Electron y se
  abren desde un ordenador conectado Over SSH. Requieren OpenSSH local y un
  destino con el host disponible; VNC espera RFB en loopback `5900` compatible
  con el visor actual. Android no administra los perfiles SSH de Electron.

## Validación de esta intervención

- Escritorio: `npm.cmd ci`, `npm.cmd run typecheck` y `npm.cmd run build`
  aprobados. `npm.cmd test`: 170 casos, 151 aprobados, 19 omitidos por plataforma,
  cero fallos. Incluye modelos/cuentas/plantillas, SSH Windows y puente VNC.
- CI: YAML de Auto Tag analizado y rutas de activación verificadas; versión
  host/escritorio coherente en `2.20.0`. No se ejecutó GitHub Actions remoto.
  No se vuelve a subir la versión porque ya supera la última etiqueta local,
  `v2.19.1`.
- Android: core, 137 casos sin fallos/errores y una omisión; app, 31 casos sin
  fallos/errores. Incluye los cuatro casos nuevos del buscador, layout de
  Constelación y diez pruebas de señalización de pantalla. Ensamblado de la app,
  lint y ensamblado de la instrumentación aprobados con
  `gradlew.bat --no-daemon --max-workers=1 :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug :app:assembleDebugAndroidTest`.
  APK Dev: `apps/android/app/build/outputs/apk/debug/app-debug.apk`, paquete
  `com.pokai.Codync.android.dev`, versión `2.20.0-dev`.
- APK actualizado con `adb install -r`, conservando datos y con la app anterior
  detenida. Instrumentación en el emulador `codync36`: **10 pruebas aprobadas,
  cero fallos y cero omisiones**. Incluye Default/seleccionado durante una
  búsqueda sin coincidencias, selección y reapertura, carga cancelada, edición
  offline y acciones/scroll/preferencia de Constelación con 30 bots. Se usan
  catálogos y bots de prueba; no acredita OAuth ni un agente remoto real.
- Lint Android: 77 advertencias, cero errores. La captura y los registros están
  en `build/validation/` (artefactos locales ignorados por Git). El renderizado
  de 30 bots registró 41 frames, p50 179 ms y p95 253 ms en este emulador
  limitado a dos núcleos y 1536 MB; no se presenta como una aceptación de fluidez
  en dispositivos reales.
- La primera suite Android informó dos timeouts en `ChannelIntegrationTest`
  durante compilaciones concurrentes y con menos de 500 MB de memoria libre.
  La repetición del core pasó los nueve casos de transporte y la suite completa
  sin ampliar plazos ni omitir casos. El ensamblado final usa un solo trabajo y
  un daemon temporal para no dejar procesos de compilación ocupando memoria.
- Computer Use Windows no pudo enumerar ventanas: `Computer Use native pipe is
  unavailable ... os error 2`. No se acredita aceptación visual Windows con
  pruebas unitarias o una compilación. El intento de `npm.cmd run dev` se detuvo
  durante la compilación del host nativo para liberar memoria; el build Electron
  sí concluyó, pero no se entrega un instalador Windows ni se certifica su arranque.
- Revisión de limpieza y revisión de refactor ejecutadas en paralelo sobre el
  diff. Sin cambios materiales sugeridos; el escapado de las rutas Windows ya
  representa un único separador en ejecución.

## Aceptación que sigue necesitando un entorno real

P-06 a P-10 son procedimientos de diagnóstico y aceptación, no cinco defectos
reproducidos. No se presentan como reparados por compilar la aplicación:

- SSH: servidor y clave autorizados, confianza de host, desconexión y túneles.
- Antigravity: consentimiento OAuth de dos identidades en el host.
- Hermes: binario externo configurado y un turno ACP completo.
- Voz: servidor compatible, clave y captura/reproducción de audio.
- VNC: servidor gráfico real, primer fotograma, entrada y cierre al desconectar.
- WebRTC móvil: auxiliar macOS/Linux, permisos y servidor TURN para probar redes
  separadas. No se implementa captura nativa Windows en este cambio.

No se solicitaron credenciales ni se modificaron perfiles de producción. iOS y
macOS quedan expresamente sin mejoras en esta intervención. Las fixtures
multiplataforma siguen simulando sus rutas para que la suite se ejecute en Windows;
eso no cambia sus aplicaciones. La TUI no tiene buscador gráfico ni superficie
VNC, y los cambios de este seguimiento no alteran su protocolo o sus acciones.
