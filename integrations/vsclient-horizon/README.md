# VSClient Native — VIS MCRust / Horizon 26.3

**Este es un mod nativo de Horizon en Rust**, no un archivo Fabric .jar
renombrado. Se compila dentro del cliente Rust de la instancia usando el
sistema de mods nativos [api] de Horizon. Minecraft Java Edition conserva
el VSClient Fabric y sus servicios de servidor originales.

## Estado — segunda adaptación funcional (revisión pendiente de CI y QA real)

| Función | MCRust |
|---|---|
| Protocolo 8, Minecraft VarInt + UTF-8 | Implementado y probado |
| Canal virgoland:auth, ticket de una sesión VIS Nexo | Soporte de envío opcional; requiere que Nexo suministre el ticket |
| Canal virgoland:hello; capacidades | Anuncia CAMERA_V1, CONTROLS_V1 y VIEW_V1 |
| Canal virgoland:camera_v1; 64 keyframes y revisión creciente | Implementado |
| Cámara visual: posición, yaw, pitch | Hook nativo |
| FOV cinematográfico | Hook nativo |
| Smoothstep, ángulo más corto, expiración, reinicio | Implementado |
| cinematic_v3, timeline y edición completas | Pendiente |
| Roll independiente del filtro de distorsión | Pendiente |
| Filtros GPU y multicámara | Pendientes; se mantienen deshabilitados |
| Controles del Bridge | Recibe state/profile; envía press/release con secuencia; libera al perder foco y cambiar estado |
| BBModel, animaciones, brazos, armas y feedback | Pendiente |
| HUD y overlays | Fades blanco/negro y bandas de software; interfaz completa pendiente |
| VIS Ceibo, skins y cuentas | Pendiente de Nexo/servicios |
| Android ARM64 | Aún no compilado ni probado |

El adaptador anuncia CAMERA_V1, CONTROLS_V1 y VIEW_V1, pero todavía no ofrece la pantalla de reasignación ni controles táctiles específicos de VSClient. No anuncia CONTROLS_UI_V1, VEHICLE_INPUT_V1,
CINEMATIC_V3 ni otras funcionalidades pendientes. La cámara y el FOV no
alteran el movimiento ni la autoridad del servidor.

## Archivos

- horizon-mixin.toml: manifiesto de mod nativo con [api].
- api/Cargo.toml: crate Rust para compilar con Horizon.
- api/src/lib.rs: hooks de login, paquetes, cámara, FOV y salida.
- api/src/core.rs: codec y timeline sin dependencias Fabric/Horizon.
- api/src/controls.rs: contrato de perfiles y acciones, con pruebas standalone.
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

El workflow visclient-native.yml compila y ejecuta tests de protocolo/cámara/controles,
comprueba el adapter contra la API real de Horizon y genera el bundle de
fuentes. Eso **no equivale a una prueba gráfica en MCRust**.

## Funciones de la etapa 0.2

El Bridge envía el perfil contextual al cliente. Este port procesa los códigos de teclas heredados sobre nombres de entrada de Horizon y solo envía acciones semánticas a `virgoland:input` (nunca ejecuta acciones administrativas por su cuenta). El servidor sigue siendo dueño de la validación, los permisos y la lógica de juego. Los pulsos de vehículo del tractor se refrescan cada cinco ticks; otros controles no se repiten automáticamente. La pérdida de foco o apertura del menú libera las acciones mantenidas. `virgoland:view` modifica solo FOV visual del cliente y se restaura al salir.

Los efectos de software actuales son `cinema:fade_black`, `cinema:black`, `cinema:fade_white`, `cinema:flash_white` y `cinema:letterbox`. No son filtros GPU ni Cinematic V3. Un servidor que requiere `launcher-auth.enabled` todavía necesita un ticket firmado real de VIS Nexo, no basta el hello.

**Aún falta para paridad completa con Java:** cinemáticas V3 multivista, actores/skins/BBModel/animaciones, transiciones completas, menús de controles, personalización persistente de bindings, gamepad y entrada táctil específica, HUD de diagnósticos y pruebas de rendimiento Android. No deben anunciarse capacidades de esas funciones sin implementarlas y probarlas.
