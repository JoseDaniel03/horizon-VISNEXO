# Candidata Android y PC: controles, ciclo de vida y conexión con Nexo

## Cambios de esta tanda

- Motor común: el host Android entra por `horizon-game`, con las mismas funciones de juego que PC. Los detalles JNI viven en el host; el motor conserva `forbid(unsafe_code)`.
- Editor táctil: mover botones, ajustar tamaño y sensibilidad, espejo, guardar, cancelar y restablecer. Se guarda `vis-touch.json` en el perfil del jugador. Los controles dibujados usan la misma geometría y visibilidad que la entrada.
- Chat, cámara, depuración y cambio de mano conectados a las acciones existentes; teclado mediante IME. Un solo dedo controla los menús para evitar clics y liberaciones cruzados.
- Primer inicio: perfiles conservadores según RAM (4/6/8 chunks; presupuestos de texturas y cargas), resolución dinámica y objetivo de 60 FPS. No sobrescribe opciones existentes ni fuerza siempre seis chunks desde los argumentos del host.
- Estado térmico Android 10+: puede bajar el límite a 60/45/30 FPS. La recuperación espera 30 segundos por nivel y nunca sube el límite elegido por el usuario. API no disponible conserva el funcionamiento normal.
- Memoria baja: libera caché de sonidos y staging gráfico reutilizable; mantiene muestras en reproducción y copias gráficas aún no enviadas.
- Suspensión: limpia entradas y mandos, pausa según las reglas existentes, pide un checkpoint y flush al servidor dueño del mundo sin bloquear el callback Android. Se conserva el mundo y se recrea la superficie al volver. Un cierre inmediato del proceso por Android todavía puede interrumpir un checkpoint en curso.
- Nexo PC: la acción explícita Actualizar / reparar consulta la última publicación con contrato PC/Android válido. La instancia se bloquea contra ejecuciones concurrentes. Inicio habitual conserva su motor fijado; mundos y preferencias no se eliminan.
- Nexo Android: Instalar / Jugar ya consulta ese mismo catálogo, verifica SHA-256 y firma instalada y solicita la instalación mediante Android. Después de confirmar la instalación se vuelve a tocar Jugar.

## Verificación realizada

- `cargo check -j 1 -p horizon-android --target aarch64-linux-android --features audio`: pasa, NDK r28c y target ARM64 real, no una simulación de cfg en Linux.
- Dos tests de política térmica/RAM: pasan.
- Cinco tests del contrato de release compartido: pasan.
- `tools/check_layers.py`: pasa, 53 crates, sin excepciones nuevas.
- `tools/check_public_tree.py`: pasa.
- Validación frontend/puente Nexo: pasa.

Los resultados adicionales de CI y comprobaciones finales se registran en el PR. Los APK de una revisión anterior no contienen estos cambios. Publicar las fuentes no equivale a publicar una nueva versión descargable.

## Límites pendientes

No se certificó juego en dispositivos Adreno/Mali, teclado de distintos fabricantes, presión real de memoria, batería ni FPS. No se atribuyen mejoras porcentuales sin medirlas. El editor aún no incluye perfiles de gestos avanzados ni zonas seguras automáticas por recorte de pantalla.

La sesión Minecraft premium todavía requiere un canal nativo entre aplicaciones Android; verificar un nombre Microsoft en Nexo no autentica el juego en servidores online-mode. El canal social del juego Android también requiere esa integración. Los decodificadores de mandos existentes no equivalen a soporte físico completo y probado.

No se trasladaron optimizaciones visuales de Perforizon indiscriminadamente. Tampoco se retiraron límites GPU de LOD: requieren pruebas comparables por dispositivo y controles de calidad visual.
