# VSClient Native — VIS MCRust / Horizon 26.3

**Este es un mod nativo de Horizon en Rust**, no un archivo Fabric .jar
renombrado. Se compila dentro del cliente Rust de la instancia usando el
sistema de mods nativos [api] de Horizon. Minecraft Java Edition conserva
el VSClient Fabric y sus servicios de servidor originales.

## Estado — cuarta adaptación funcional (sin QA in-game en Android/PC)

| Función | MCRust |
|---|---|
| Protocolo 8, Minecraft VarInt + UTF-8 | Implementado y probado |
| Canal virgoland:auth, ticket de una sesión VIS Nexo | Soporte de envío opcional; requiere que Nexo suministre el ticket |
| Canal virgoland:hello; capacidades | Anuncia CAMERA_V1, CONTROLS_V1, CONTROLS_UI_V1 y VIEW_V1 |
| Canal virgoland:camera_v1; 64 keyframes y revisión creciente | Implementado |
| Cámara visual: posición, yaw, pitch | Hook nativo |
| FOV cinematográfico | Hook nativo |
| Smoothstep, ángulo más corto, expiración, reinicio | Implementado |
| cinematic_v3 | Parser validado; cámara principal, subtítulos y efectos GUI. Multivista/actores 3D pendientes, no se anuncia al Bridge |
| Roll independiente del filtro de distorsión | Pendiente |
| Filtros GPU y multicámara | Pendientes; se mantienen deshabilitados |
| Controles del Bridge | State/profile; press/release; antirreplay; suprime mouse y movimiento tomados por acciones |
| Brazos, armas y feedback | Recoil/action_effect y recarga de brazos local; BBModel, animación de actores y destellos por entidad pendientes |
| HUD y overlays | Fades y bandas, GUI V3 con subtítulos multilínea y opacidad; acceso VS en pantalla, F8 y perfil persistente |
| VIS Ceibo, skins y cuentas | Pendiente de Nexo/servicios |
| Android ARM64 | Layout de acciones táctiles con retención y liberación compartida con el teclado; sin build/QA en un dispositivo real |

El adaptador anuncia CAMERA_V1, CONTROLS_V1, CONTROLS_UI_V1 y VIEW_V1. La pantalla nativa de controles se abre con F8 o por mensaje del Bridge, permite reasignar teclas, eliminar una asignación con Supr y restaurar los controles. Los cambios se guardan por instancia/perfil. Todavía no hay botones táctiles propios de VSClient. No anuncia VEHICLE_INPUT_V1,
CINEMATIC_V3 ni otras funcionalidades pendientes. La cámara y el FOV no
alteran el movimiento ni la autoridad del servidor.

## Archivos

- horizon-mixin.toml: manifiesto de mod nativo con [api].
- api/Cargo.toml: crate Rust para compilar con Horizon.
- api/src/lib.rs: hooks de login, paquetes, cámara, FOV y salida.
- api/src/core.rs: codec y timeline sin dependencias Fabric/Horizon.
- api/src/controls.rs: contrato de perfiles y acciones, reasignación persistida, con pruebas standalone.
- api/src/ui.rs: ventana nativa paginada para teclado/ratón, con entrada por el Bridge.
- api/src/cinematic.rs: decoder validado de frames V3 y simulación limitada de presentación.
- api/src/feedback.rs: efectos visuales VSRender y cinemática de recarga de brazos.
- api/src/lib.rs: recibe canales state/profile/view, usa hooks nativos para input y dibujo de overlays.
- crates/horizon-client/tests/vsclient_native_port.rs: compila contra Hooks.

El fork exporta CustomPayload desde horizon_client::mod_api, evitando
duplicar tipos Rust del paquete horizon-protocol.

## Instalación de prueba

1. Actualizar la instancia VIS MCRust al fork JoseDaniel03/horizon-VISNEXO
   que incluya la API de payloads de esta rama (mínimo fa614607).
2. Descomprimir el artifact VSClient-MCRust-native-source o copiar la carpeta
   integrations/vsclient-horizon del repositorio.
3. Con el CLI nativo de Horizon, ejecutar el comando equivalente a:

       horizon-launcher native-build integrations/vsclient-horizon --instance INSTANCIA --trust-native-code

   Requiere un build nativo del juego para la instancia seleccionada. La
   integración automática de esta acción en el flujo VIS Nexo está pendiente:
   **copiar un JAR Fabric o una carpeta fuente a mods no basta**.
4. Probar el protocolo 8 contra CanvasMC/Folia 26.3 y el Bridge de VSClient.
   Reproducir una cámara, confirmar la expiración y desconexión sin efectos
   residuales, y verificar el comportamiento del servidor con cuentas.
5. Hacer QA de FPS, cámara y render con jugador real antes de producción.

## Autenticación opcional del Bridge

Cuando el servidor usa launcher-auth.enabled=true, el Bridge de VSClient exige
un ticket de una sola utilización expedido por la plataforma; mandar hello
sin ticket NO autentica al jugador. El mod puede leer una variable privada del
proceso del juego llamada VIS_NEXO_CLIENT_BRIDGE_TICKET y transmitirla por el
canal virgoland:auth ANTES del hello, en protocolo 8.

**VIS Nexo todavía debe obtener ese ticket del servicio autorizado y pasarlo
al proceso MCRust**; esta entrega no crea, renueva ni almacena credenciales.
Los tickets ausentes o inválidos se ignoran y un servidor con auth obligatoria
rechazará la sesión; nunca se genera uno falso. El ticket no se registra en logs.
No debe incluirse en repositorios ni comandos compartidos: Nexo debe inyectarlo
directamente en el entorno de su proceso hijo y limitar su vida útil.

## Seguridad

- Procesa únicamente CustomPayload de cámara, estado, perfil y zoom con límites explícitos.
- Longitud con VarInt exacto, UTF-8 válido y máximo de 8192 bytes por cue.
- Hasta 64 keyframes y 8 efectos: números finitos, tiempos crecientes,
  coordenadas dentro del mundo y FOV válido.
- Revisión creciente para evitar replays; al desconectar se limpia la sesión.
- No toca otros canales, usuarios, skins, juegos Java ni Resource Packs.
- El manifest no fuerza volver a un commit viejo de Horizon, evitando
  retroceder la instancia que VIS Nexo haya instalado.
- Como todo mod Rust nativo, se compila con permisos locales del usuario.
  Instalá solamente paquetes cuya procedencia hayas verificado.

## Validación

El workflow visclient-native.yml compila y ejecuta tests de protocolo/cámara/controles/feedback,
comprueba el adapter contra la API real de Horizon y genera el bundle de
fuentes. Eso **no equivale a una prueba gráfica en MCRust**.

## Funciones de la etapa 0.2

El Bridge envía el perfil contextual al cliente. Este port procesa los códigos de teclas heredados sobre nombres de entrada de Horizon y solo envía acciones semánticas a `virgoland:input` (nunca ejecuta acciones administrativas por su cuenta). El servidor sigue siendo dueño de la validación, los permisos y la lógica de juego. Los pulsos de vehículo del tractor se refrescan cada cinco ticks; otros controles no se repiten automáticamente. La pérdida de foco o apertura del menú libera las acciones mantenidas. `virgoland:view` modifica solo FOV visual del cliente y se restaura al salir.

Los efectos de software actuales son `cinema:fade_black`, `cinema:black`, `cinema:fade_white`, `cinema:flash_white` y `cinema:letterbox`. No son filtros GPU ni Cinematic V3. Un servidor que requiere `launcher-auth.enabled` todavía necesita un ticket firmado real de VIS Nexo, no basta el hello.

**Aún falta para paridad completa con Java:** cinemáticas V3 multivista, actores/skins/BBModel/animaciones, transiciones completas, menús de controles, personalización persistente de bindings, gamepad y entrada táctil específica, HUD de diagnósticos y pruebas de rendimiento Android. No deben anunciarse capacidades de esas funciones sin implementarlas y probarlas.

## Uso directo 0.3

- Abrir **F8** durante el juego o desde el mensaje de apertura `virgoland:controls` del servidor.
- Clic en una acción, luego tecla/botón a asignar. **Supr/Backspace** desasigna; **Esc** cancela la captura.
- Anterior/Siguiente navega las acciones disponibles. **Restaurar** limpia overrides del perfil actual.
- Las preferencias viven bajo la carpeta de datos nativos de VSClient dentro de la **instancia** de Horizon. No se comparten con otras cuentas/instancias por defecto.
- Al cambiar el perfil/estado, abrir un menú, perder foco o desconectar se liberan acciones mantenidas. El ratón asignado a una acción contextual cancela el clic Vanilla correspondiente, y la entrada de movimiento ligada a controles del servidor no mueve al jugador de forma accidental.

## Compatibilidad real con cinemáticas

Se interpreta el JSON V3 con presupuesto de 5 vistas, 16 actores, 16 subtítulos y 8 efectos, verificando keyframes, revisiones, tiempos y geometría. **Por ahora solo se puede renderizar la cámara principal del mundo**, efectos GUI básicos y subtítulos multilínea con opacidad; falta capturar y componer las otras cuatro vistas y aplicar los rig/skins animados de actores. El renderer nativo de Horizon no proporciona ese compositor directamente por el protocolo del Bridge, por lo que **NO se anuncian CINEMATIC_V3 ni CINEMATIC_TRANSITIONS_V1**. Los mensajes V3 pueden recibirse de manera experimental, pero esto no garantiza que escenas antiguas reproduzcan igual que en Fabric.

La recarga de brazos `vsrender:reload_arms` y los impulsos `vsrender:action_effect` son estrictamente visuales. El estado de slots, la mano activa y las animaciones de entidad todavía requieren integración con el renderer y no deberían considerarse equivalentes a la versión Java. Los tests de cargo/check no prueban la composición de GPU ni la compatibilidad Android ARM64.

## Versión 0.4 — acciones Android y diagnóstico

- En Android el mod muestra automáticamente, si hay un perfil válido del Bridge, una banda de hasta **cuatro botones de acciones**. Pulsar y soltar envía los mismos `virgoland:input` con secuencia que el teclado, respetando la simultaneidad de ambos orígenes. El botón **Más controles** cambia de página. Los clics no interceptan el resto de la pantalla.
- En PC la banda comienza desactivada y se puede alternar mediante **F9**; la preferencia se guarda en el directorio del mod dentro de la instancia.
- El botón **VS** permite abrir la ventana de controles desde la HUD sin usar un teclado físico, si el toque del sistema operativo llega como evento de puntero de Horizon.
- Dentro de la ventana ahora se pueden **reasignar botones del ratón** además de teclas. Los perfiles se guardan por instancia y no necesitan reconstruir un JAR Fabric.
- **F7**: ver enlace con el Bridge, estados, paquetes aceptados/rechazados, envíos y errores. No equivale a verificar la autenticación de Microsoft ni el servidor. Cuando el panel está visible, **F6** guarda `last_diagnostic` en el directorio de preferencias de VSClient; no conserva IP, chat, paquetes, sesiones ni tickets.
- **F7**, durante una cinemática V3, muestra cajas 3D transitorias para revisar posiciones, escalas y nombres de los actores sin entidades reales: **son únicamente marcadores de depuración, no BBModel ni personajes renderizados**.

**Límite de entrada:** los controles virtuales son de **un puntero**. El sistema Android/Horizon debe emitir los eventos de puntero; no se implementa aquí la pila multitáctil del sistema operativo ni el joystick analógico del cliente. El compositor V3 de vistas múltiples, actores con skins, animaciones BBModel y GPU shaders siguen pendientes; no se anuncian al Bridge. Nunca se modifica la autoridad del servidor. Las verificaciones actuales prueban el protocolo, los hooks y los layouts, no la conexión real a Virgoland.
