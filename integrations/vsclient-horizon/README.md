# VSClient Native — VIS MCRust / Horizon 26.3

**Este es un mod nativo de Horizon en Rust**, no un archivo Fabric .jar
renombrado. Se compila dentro del cliente Rust de la instancia usando el
sistema de mods nativos [api] de Horizon. Minecraft Java Edition conserva
el VSClient Fabric y sus servicios de servidor originales.

## Estado — primera adaptación funcional

| Función | MCRust |
|---|---|
| Protocolo 8, Minecraft VarInt + UTF-8 | Implementado y probado |
| Canal virgoland:auth, ticket de una sesión VIS Nexo | Soporte de envío opcional; requiere que Nexo suministre el ticket |
| Canal virgoland:hello; capacidades | Implementado: anuncia solo CAMERA_V1 |
| Canal virgoland:camera_v1; 64 keyframes y revisión creciente | Implementado |
| Cámara visual: posición, yaw, pitch | Hook nativo |
| FOV cinematográfico | Hook nativo |
| Smoothstep, ángulo más corto, expiración, reinicio | Implementado |
| cinematic_v3, timeline y edición completas | Pendiente |
| Roll independiente del filtro de distorsión | Pendiente |
| Filtros GPU, multicámara y efectos | Pendiente |
| Controles SDL3, bindings y menús | Pendiente |
| BBModel, animaciones, brazos, armas y feedback | Pendiente |
| UI completa, HUD y overlays | Pendiente |
| VIS Ceibo, skins y cuentas | Pendiente de Nexo/servicios |
| Android ARM64 | Aún no compilado ni probado |

El adaptador solo anuncia CAMERA_V1. No anuncia CONTROLS_V1, VIEW_V1,
CINEMATIC_V3 ni otras funcionalidades pendientes. La cámara y el FOV no
alteran el movimiento ni la autoridad del servidor.

## Archivos

- horizon-mixin.toml: manifiesto de mod nativo con [api].
- api/Cargo.toml: crate Rust para compilar con Horizon.
- api/src/lib.rs: hooks de login, paquetes, cámara, FOV y salida.
- api/src/core.rs: codec y timeline sin dependencias Fabric/Horizon.
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

- Solo procesa Clientbound CustomPayload en virgoland:camera_v1.
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

El workflow visclient-native.yml compila y ejecuta tests de protocolo/cámara,
comprueba el adapter contra la API real de Horizon y genera el bundle de
fuentes. Eso **no equivale a una prueba gráfica en MCRust**.
