# Auditoría de forks de Horizon — 2026-11-14

## Alcance y límites

Censo público: 1.307 forks directos y 1.646 repositorios únicos al recorrer descendientes de 30 padres con forks. Todas las páginas consultadas finalizaron sin errores; no quedaron padres pendientes en esa captura. La API mostraba 1.633 en el contador de upstream: son lecturas no atómicas y el contador no equivale al resultado paginado de una red que cambia. El CSV adjunto registra lo realmente enumerado; no constituye una garantía sobre forks privados, borrados ni cambios posteriores.

Filtro inicial de metadatos por Android/móvil/rendimiento: 57 candidatos, más cinco repositorios encontrados durante la investigación. Se solicitaron 62 README: 60 disponibles y 38 contenidos únicos. Un README idéntico no demuestra código idéntico. Se inspeccionaron árboles de 12 revisiones y 70 archivos seleccionados de ocho variantes. **No se auditó línea por línea todo el código de los 1.646 repositorios.** No se ejecutaron binarios externos ni se midió FPS en teléfonos.

Base comparada: VIS `6bc3838117ec08f963c1ccbd72341e4a710df11c`, rama `codex/vis-portals-worlds`. Los resultados describen fuentes, no garantizan que un APK publicado incluya esa revisión.

## Android: qué hicieron y qué tenemos

| Línea | Trabajo observado | Estado en VIS / decisión |
|---|---|---|
| Starling actual; JuanLuci histórico | Android nativo, JNI, controles editables, perfiles, teclado y mandos. Starling declara integrar las líneas previas de JuanLuci y Horizon Android. | Ya incorporamos adaptadores de una revisión anterior de Starling, con atribución. Falta llevar la experiencia completa de controles a nuestro motor compartido. No reemplazar todo el árbol de gameplay. |
| gamecorner-033/horizon-android | Entrada Android, controles y configuración de arranque; documentación de Vulkan/GLES y autenticación. | Comparar correcciones puntuales de entrada/teclado/arranque. Nuestro renderer ya tiene selección Vulkan/GL; no es una función completamente ausente. |
| Rayan-XOO/horizon | Módulos separados de preparación Android, Microsoft, IME y entrada, además de layout/aim táctil. | Referencia para cerrar autenticación real e IME; encontrar esos archivos no prueba que nuestra sesión premium ya llegue al juego. |
| CornyBirdy/horizon4android | Adaptación táctil en cliente y eventos de juego. | Referencia secundaria frente a Starling actual, sin asumir que sea una implementación independiente. |
| Aurora | Módulos de estado térmico, preferencias/runtime por dispositivo y diagnóstico; documentación de reducción de carga y respuesta a presión de memoria. | No encontramos un gobernador térmico equivalente ni tratamiento explícito de avisos de memoria en nuestro ciclo Android. Es una de las diferencias más valiosas. La propia documentación deja sin validar la presión de memoria real en teléfono. |
| Perforizon | Opciones de meshing, culling, sombras, LOD y autotuning, con decisiones dependientes de la GPU. | Auditar parches concretos y medirlos. Algunas opciones modifican imagen; resultados de GPU de escritorio no se trasladan automáticamente a Android. |
| Otros forks optimizados / LODGPULimit | Se fijaron revisiones y árboles para rastrear diferencias. | Nombre, estrellas o ausencia de un límite no demuestran una mejora. No se recomienda importar estas variantes completas. |

## Evidencia en nuestro código

- `crates/horizon-android`: host NativeActivity fino que arranca el mismo `horizon-client` y registro oficial que PC. Mantener versión, protocolo y formatos comunes.
- `crates/horizon-client-render/src/android_runtime.rs`, `android_gamepad.rs`, `touch.rs`, `touch/layout.rs`, `touch/aim.rs`: infraestructura reutilizada de Starling. La existencia de un decodificador de mandos no demuestra que todos los eventos físicos Android estén conectados y probados.
- `crates/horizon-client/src/game/android_touch.rs`: controles básicos; oculta Chat, Settings, Debug y Perspective. No tenemos el editor táctil completo de Starling.
- `crates/horizon-client-render/src/graphics.rs`: resolución configurable 50–200%, resolución dinámica, FPS objetivo, modo batería, caché de sombras y presupuestos de carga/texturas. Varias opciones están apagadas por defecto: disponible no significa activado ni ajustado para móvil.
- `crates/horizon-client/src/game/frame_autotune.rs`: lógica de ajuste de resolución y culling ya presente. Evitar duplicarla con otro autotuner.
- `crates/horizon-client/src/game/window_handler.rs`: al suspender limpia entrada, libera superficie gráfica y aplica la pausa existente; al reanudar recrea superficie. No se observó guardado/flush explícito ni un tratamiento específico de memoria baja en esa ruta.
- Preparación de recursos y manifiestos compartidos ya existen; eso no demuestra instalación y juego completos en todos los fabricantes.

## Brechas de producto adicionales

La adaptación social dentro del juego Android y el traspaso de sesión Minecraft premium al motor requieren cierre y pruebas reales. Tener nombre/token verificados en Nexo no basta para asegurar entrada a servidores online-mode. Los avisos repetidos de HTTPS/nube pertenecen al launcher y servicios: este censo de forks no demuestra que estén resueltos en la versión instalada por el usuario.

## Prioridad recomendada

1. Jugabilidad: controles editables, inventario/arrastre, chat, teclado, sensibilidad, zonas seguras y mandos con eventos conectados. Conservar el motor común PC/Android.
2. Robustez Android: presión de memoria, liberación de cachés, suspensión/reanudación y persistencia del mundo antes de que Android cierre el proceso.
3. Perfiles por GPU/RAM y control térmico con histéresis; ajustar distancias, presupuestos de carga y límite de FPS. No copiar umbrales sin validar dispositivos.
4. Prueba reproducible: mismo mundo/recorrido, arranque frío, percentiles de tiempo por frame, RAM, temperatura y batería. Separar Vulkan/GL y comprobar al menos Adreno y Mali.
5. Después, portar optimizaciones de renderer por separado, verificando calidad visual y compatibilidad. No activar todos los switches de Perforizon a la vez ni eliminar límites de LOD indiscriminadamente.

## Revisiones fijadas

- [starling-client/starling](https://github.com/starling-client/starling/tree/09fa16c167c4b2f8eb23fddd0faa808d56d0a2f2) — `09fa16c167c4b2f8eb23fddd0faa808d56d0a2f2`
- [JuanLuci/starling](https://github.com/JuanLuci/starling/tree/7c38f33c1ce1fb18ae513c7b60ab2dda178eb282) — `7c38f33c1ce1fb18ae513c7b60ab2dda178eb282`
- [gamecorner-033/horizon-android](https://github.com/gamecorner-033/horizon-android/tree/7d83d697b131ff0ca5ff8329b49992aca4798bf4) — `7d83d697b131ff0ca5ff8329b49992aca4798bf4`
- [Rayan-XOO/horizon](https://github.com/Rayan-XOO/horizon/tree/20bb8f2d6cd5a688509a05155142a28e555ea280) — `20bb8f2d6cd5a688509a05155142a28e555ea280`
- [CornyBirdy/horizon4android](https://github.com/CornyBirdy/horizon4android/tree/e3ba22c197e50f038c81b825473904f254f3ff94) — `e3ba22c197e50f038c81b825473904f254f3ff94`
- [IAmFuckingGenius/horizon-aurora](https://github.com/IAmFuckingGenius/horizon-aurora/tree/db9793d3e8cfb8b9037b679038d35546512c29f4) — `db9793d3e8cfb8b9037b679038d35546512c29f4`
- [Louchatfroff/Perforizon](https://github.com/Louchatfroff/Perforizon/tree/e8a2eead2e56e423df32ce32821eebe083c0c4a4) — `e8a2eead2e56e423df32ce32821eebe083c0c4a4`
- [Arthur6229/horizon-optimized](https://github.com/Arthur6229/horizon-optimized/tree/fc13354764a20cfb6fdaab9528a86a0dcb327982) — `fc13354764a20cfb6fdaab9528a86a0dcb327982`
- [Cottleta/horizon-Cottleta](https://github.com/Cottleta/horizon-Cottleta/tree/084b20d5cfbb104d4a70f61091198831074619f1) — `084b20d5cfbb104d4a70f61091198831074619f1`
- [ferronatin/horizon-optimized](https://github.com/ferronatin/horizon-optimized/tree/0a341cc353dbd2325337efc96758d102ba9a4b5d) — `0a341cc353dbd2325337efc96758d102ba9a4b5d`
- [islam04081998/horizon](https://github.com/islam04081998/horizon/tree/7eb057384c4c2ec6bc864f835eb26900aca54f79) — `7eb057384c4c2ec6bc864f835eb26900aca54f79`
- [ZevroGD/LODGPULimit](https://github.com/ZevroGD/LODGPULimit/tree/173269596d42d280b96c3b02245b7ac725a516a0) — `173269596d42d280b96c3b02245b7ac725a516a0`

## Fuentes y reproducibilidad

- [Red original](https://github.com/horizon-rust/horizon/forks), API pública `repos/{owner}/{repo}/forks?per_page=100`, paginada y deduplicada por nombre; recorrido de descendientes con forks.
- [Starling](https://github.com/starling-client/starling), [JuanLuci](https://github.com/JuanLuci/starling), [documentación Perforizon](https://louchatfroff.github.io/Perforizon/).
- Aurora: `docs/android-performance.md`, `crates/horizon-client/src/android/thermal.rs` en la revisión fijada; en particular sus secciones Thermal governor y Memory pressure.
- [Inventario CSV](fork-network-2026-11-14.csv). Fecha de captura: 2026-11-14; repetir el recorrido puede producir otro total.

Esta entrega documenta la comparación. No modifica runtime, no importa motores extranjeros y no certifica jugabilidad sin pruebas en hardware Android.
