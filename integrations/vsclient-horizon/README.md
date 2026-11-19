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
| Roll de cámara | Aproximación visual limitada a 10°, usando cámara-shake nativo y respetando ajustes de distorsión |
| Filtros GPU y multicámara | Pendientes; se mantienen deshabilitados |
| Controles del Bridge | State/profile; press/release; antirreplay; suprime mouse y movimiento tomados por acciones |
| Brazos, armas y feedback | Action recoil, recarga visual local y destellos de `vsrender:weapon_feedback` sobre entidades conocidas; BBModel/animación de actores pendientes |
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

## Instalación por instancia con VIS Nexo (PC)

El launcher actualizado ya detecta el mod antes de iniciar McRust. No es
necesario instalar Rust ni ejecutar scripts Python para usarlo.

1. Descargá el artifact `VSClient-MCRust-0.4.1-HNMOD.zip` desde la compilación
   oficial del mismo fork.
2. Poné **el ZIP exterior tal cual** o su `VSClient-MCRust-0.4.1.hnmod`
   en la raíz de tu instancia Horizon o en `mods/`.
3. Iniciá la instancia desde VIS Nexo. El launcher valida límites,
   manifiestos y versión, coloca `horizon-native-mods/vsclient.hnmod`
   y activa `mod.vsclient:true` sin sobrescribir un mod distinto.
4. El ejecutable oficial de McRust tiene que estar compilado con
   `horizon-official/vis-vsclient` (versión nativa 0.4.1). Si no lo está,
   VIS Nexo solicita actualizar el motor en vez de ejecutar Rust desde ZIP.

El contenido Rust del ZIP **nunca se carga ni se compila al iniciar**:
solo activa los hooks del VSClient que forman parte del motor verificado.
La activación pertenece a la instancia seleccionada, no a otras cuentas.

**Android:** el motor ARM64 debe contener VSClient y VIS Nexo Android debe
pasar su opción de activación validada en el inicio. Android no permite
escribir en la carpeta privada de otra app. Las comprobaciones de compilación
no sustituyen pruebas en un teléfono real.

**Desarrolladores de Horizon independiente:** el CLI `native-build` sigue
existiendo para compilar mods nativos confiables con Rust, pero no es el
mecanismo requerido al usar VIS Nexo con el motor precompilado.

## Mejoras de estabilidad del adaptador

- El diagnóstico **F7** contabiliza perfiles aceptados y rechazados del
  Bridge; el estado «Bridge activo» solo se muestra tras perfil y estado
  válidos.
- Los perfiles y cambios de estado con revisiones repetidas o antiguas no
  sueltan acciones mantenidas por el teclado o la pantalla táctil.
- El motor da prioridad a `--game-dir` frente a `--home` al buscar el
  mod: una instancia inexistente no habilita accidentalmente el mod de
  otra carpeta.
- Las transiciones de estado nuevas siguen liberando las acciones retenidas
  para evitar aceleradores, disparos o controles de vehículos atascados.
- Sigue faltando QA gráfico real de cámaras y efectos en Windows,
  Linux y Android; tampoco se anuncian vistas GPU ni BBModel completos.

## Autenticación opcional del Bridge

Cuando el servidor usa launcher-auth.enabled=true, el Bridge de VSClient exige
un ticket de una sola utilización expedido por la plataforma; mandar hello
sin ticket NO autentica al jugador. El mod puede leer una variable privada del
proceso del juego llamada VIS_NEXO_CLIENT_BRIDGE_TICKET y transmitirla por el
canal virgoland:auth ANTES del hello, en protocolo 8. El ticket se envía como máximo
**una vez por proceso**, incluso si se cambia de servidor, y nunca se guarda en logs.

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

## Feedback visual remoto

Se interpretan ahora los paquetes `vsrender:weapon_feedback` con los límites de Fabric (`actor`, recoil, recuperación, luz 0–15 y flash máximo de seis ticks). Durante el flash se dibuja una pequeña geometría traslúcida y temporal frente a la posición de la entidad recibida en Horizon. El efecto no modifica la iluminación física del mundo, la lógica de daño ni el inventario. Las animaciones esqueléticas de recarga de otras entidades y la generación del modelo BBModel requieren una capa 3D nativa adicional.

**Limitaciones importantes:** la conexión real a Virgoland, los permisos y autenticación de Microsoft, GPU multivista, skins y renderizado de modelos, gamepad analógico, Android multitáctil y QA del Motorola no están validados. Los informes F6 no contienen tickets ni direcciones; el código actual tampoco verifica que un ticket de VIS Nexo esté vinculado al host: el proceso que lo inyecta debe garantizarlo.

## Corrección 0.4.1: paquete reconocido por Horizon

**El artifact de GitHub Actions es un ZIP contenedor, no el archivo nativo.** Abrí el ZIP y extraé **`VSClient-MCRust-0.4.1.hnmod`**. No pegues el ZIP de Actions ni el `.tar.gz` en la carpeta de mods de Java.

El formato nativo se verifica con el parser `horizon_mods::index::native::Bundle::read`, pero **eso no lo instala ni compila**. El fork de Horizon ahora acepta un fichero `.hnmod` directamente al invocar `native-build`, y cuando su manifiesto pincha un SHA exclusivo de `JoseDaniel03/horizon-VISNEXO` descarga el código de dicho fork, en lugar de fallar buscando el commit en el proyecto upstream. El paquete deja de declarar compatibilidad genérica de `hooks` porque VSClient requiere APIs específicas de CustomPayload del fork.

En una instancia de Horizon compatible, usá:

```bash
horizon-launcher native-build /ruta/VSClient-MCRust-0.4.1.hnmod --instance "Nombre de instancia" --allow-version-mismatch --trust-native-code
```

Alternativamente, copiá el archivo **renombrado a `vsclient.hnmod`** en `<carpeta-de-la-instancia>/horizon-native-mods/`, activalo en el administrador de mods de esa instancia y elegí **Native build / Build**. Debe verse como un mod nativo, **no como un mod Fabric en `mods/`**. La compilación produce una nueva versión propia de Horizon dentro de esa instancia. El cliente de Horizon oficial que no incluya los cambios de nuestro fork **no es compatible**. En Android, esta distribución `.hnmod` tampoco se carga automáticamente como una APK; hace falta integración y compilación nativa para la arquitectura ARM64.

La versión 0.4.1 resuelve rutas de detección e instalación; **no agrega las capacidades gráficas pendientes** (composición multicámara, BBModel, filtros GPU) y todavía no se verificó con el juego abierto en un dispositivo real.

**Importante para el fork:** cuando la instancia arranca de un Horizon oficial, el SHA del VSClient se considera otra revisión. El launcher muestra su advertencia de incompatibilidad y exige la aceptación explícita `--allow-version-mismatch`; el motor resultante se construye sobre el código de nuestro fork. No activar `--ignore-pins`: perdería las APIs específicas de VSClient. Si el paquete no aparece en la lista tras copiarlo, comprobar que `horizon-mods.txt` contenga exactamente `mod.vsclient:true` o usar `INSTALAR_VSCLIENT.py` desde el artifact corregido.

## Modo automático para VIS Nexo (opción recomendada)

En las compilaciones VIS del **juego oficial de Horizon** se incluye VSClient 0.4.1 con el feature de Cargo `horizon-official/vis-vsclient`. Ese código Rust se enlaza **una sola vez durante el build del motor**, antes de distribuirlo. El archivo `.hnmod` de la instancia **no contiene código que se ejecute al iniciarla**: únicamente identifica la versión y activa el código ya incluido. No requiere Rust, Python, el Horizon Launcher separado ni el comando `native-build` en la computadora del jugador.

**Uso desde VIS Nexo actualizado:** copiá el archivo extraído `VSClient-MCRust-0.4.1.hnmod` a la **raíz de la instancia Horizon** o a su carpeta `mods/`, y pulsá **Jugar**. VIS Nexo valida el archivo, lo registra bajo `horizon-native-mods/vsclient.hnmod` y activa `mod.vsclient:true`. El juego oficial reconoce el paquete y vincula los hooks precompilados durante el arranque. En otra instancia sin el archivo, VSClient queda inactivo. No se tocan los perfiles ni los mundos.

**Requisito:** el motor debe tener `vsclient_embedded: "0.4.1"` en su manifiesto verificado `vis-horizon-source.json`; una versión vieja de McRust no puede aceptar mods Rust por simple copia. VIS Nexo muestra una advertencia de actualización en ese caso, en lugar de iniciar silenciosamente sin VSClient. Los instaladores de Android necesitan una versión ARM64 con esta integración compilada en la APK; **no** basta con copiar `.hnmod` en Android.

El modo manual `native-build` sigue siendo útil para desarrolladores que quieran usar código fuente de otros mods nativos. El modo automático solamente activa la implementación de VSClient que fue compilada y auditada con el propio juego, sin ejecutar arbitrariamente los archivos entregados por usuarios.
