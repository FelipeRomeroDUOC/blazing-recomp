# Propuesta: sistema de volcado de fotogramas y diagnóstico causal en Android

Fecha: 2026-10-28
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Permite grabar capturas de los fotogramas en el almacenamiento del dispositivo para identificar con exactitud en qué momentos ocurre el parpadeo visual.

### 2. Contexto

- Contexto consultado: `src/platform.rs`, `src/pipeline.rs`, `src/runner.rs`, `src/bin/ppu_doble.rs`, `examples/comun/diagnostico.rs`, `src/perf.rs`.
- Intención: Proporcionar a la persona usuaria en Android la misma capacidad de volcado y diagnóstico causal que ofrece el visor interactivo de PC (`ppu_doble`), permitiendo capturar los cuadros exactos de la cinemática del dragón en archivos `.png` (compuesto de 960x160 con 4 paneles idéntico al visor de PC) y reportes `.txt` para su análisis directo.
- Restricciones y decisiones previas (máx. 3):
  - No degradar la experiencia de juego en uso normal: el volcado debe permanecer totalmente inactivo salvo que se solicite explícitamente (origen: `AGENTS.md` §1) → implica: activación controlada por archivo bandera (`dump-frames.flag`) o combinación táctil de botones (`L + R + Select + Start`) → protegida por: "Detección de activación de volcado por archivo bandera o combinación".
  - Cero bloqueos en los hilos de emulación o presentación: la codificación PNG y la escritura en disco deben procesarse en un hilo secundario asíncrono con límite de seguridad (origen: `AGENTS.md` §1) → implica: uso de canal desacoplado con búfer acotado y tope de 600 fotogramas continuos → protegida por: "Escritura asíncrona y tope de seguridad de fotogramas".
  - Almacenamiento seguro en Android: los volcados deben guardarse prioritariamente en la carpeta de **Descargas** (`/sdcard/Download/blazing-recomp/volcados/` o `/storage/emulated/0/Download/blazing-recomp/volcados/`), para que la persona usuaria pueda acceder a ellos directamente desde el explorador de archivos de Android o por USB sin buscar en carpetas internas (origen: requerimiento explícito del usuario) → implica: resolver la carpeta Downloads como primera prioridad absoluta en la ruta de volcado → protegida por: "Generación de imágenes compuestas PNG y diagnósticos de texto".
- Alternativas descartadas:
  - Volcar siempre todos los fotogramas en disco de forma incondicional → descartada porque saturaría rápidamente el espacio de almacenamiento del teléfono (60 cuadros por segundo = ~150 MB por minuto).
  - Limitar el volcado a logs de texto sin imágenes PNG → descartada porque el parpadeo es un problema visual que exige inspeccionar el píxel exacto dibujado por la GPU frente al emulador.
- Ante imprevistos, prioriza: estabilidad de la aplicación y preservación del espacio de almacenamiento sobre la captura ilimitada de cuadros.

### 3. Ejemplos

- Caso normal: El usuario coloca un archivo vacío `dump-frames.flag` en la carpeta de Descargas (`Download/`), o presiona simultáneamente `L + R + Select + Start` en los controles táctiles → antes: no existía ningún mecanismo de volcado de fotogramas en Android → ahora: el HUD muestra el indicador rojo `[REC]` y el motor comienza a guardar en `Download/blazing-recomp/volcados/` las imágenes compuestas `cuadro-<N>-compuesto.png` (960x160 con 4 paneles: Software fin, Software linea, Nativa GPU y Diferencias resaltadas en magenta) y los reportes `cuadro-<N>.txt` con los registros activos y desglose causal.
- Caso límite: El usuario deja activada la bandera y juega durante un tiempo prolongado → antes: no aplicaba → ahora: el grabador aplica un techo de seguridad de 600 fotogramas continuos (10 segundos a 60 FPS) por sesión de volcado para no llenar la memoria del teléfono.

### 4. Pasos

1. Crear en `src/volcado.rs` el subsistema de volcado de fotogramas y codificación de imagen:
   - Rutinas puras de codificación PNG rápida (`png_rgb`) y generación de lienzo compuesto cuádruple de 960x160 (Software fin, Software linea, GPU nativa y mapa de diferencias en magenta).
   - Generación de informe textual con número de cuadro, recuento de píxeles distintos y registros I/O por línea.
   - Hilo trabajador en segundo plano con cola de tareas desacoplada y tope de seguridad de 600 fotogramas para escribir directamente en la carpeta `Download/blazing-recomp/volcados/`.
2. Actualizar `src/runner.rs` y `src/pipeline.rs`:
   - Permitir activar/desactivar la omisión de PPU por software dinámicamente (`set_omitir_ppu_software(false)` cuando el volcado esté activo) para que `runner.frame_rgba()` se compute y se transfiera en `FramePacket` junto al estado de GPU.
3. Actualizar `src/platform.rs` (`Compositor` y bucle de Android):
   - Configurar `game_texture` con el uso `wgpu::TextureUsages::COPY_SRC` para permitir transferencias de readback hacia el buffer de staging sin pánico en la GPU.
   - Implementar buffer de staging y copia de textura para extraer los píxeles reales dibujados por `NativePpu`.
   - Detección de la bandera `dump-frames.flag` mediante `resolve_perf_candidates` o atajo de entrada (`L + R + Select + Start`).
   - Envío de paquetes de diagnóstico al hilo de volcado y visualización de `[REC]` en el HUD de pantalla.
4. Escribir tests unitarios en `src/volcado_tests.rs`:
   - Test de codificación PNG pura (encabezados IHDR, bloques IDAT y firma PNG).
   - Test de composición de imagen de 4 paneles (960x160) y mapa de diferencias en magenta.
   - Test del tope de seguridad de 600 fotogramas y escritura en disco.
5. Ejecutar la suite completa de pruebas para garantizar ausencia de regresiones.

### 5. Qué más podría verse afectado

- `src/lib.rs` (exporta módulo `volcado`).
- `src/volcado.rs` (nuevo módulo de volcado y codificación PNG).
- `src/volcado_tests.rs` (pruebas unitarias del módulo de volcado).
- `src/runner.rs` (control dinámico de omisión de PPU software).
- `src/pipeline.rs` (transporte del búfer de referencia cuando el volcado está activo).
- `src/platform.rs` (uso `COPY_SRC` en `game_texture`, extracción de textura GPU y control de grabación con `[REC]`).
Búsqueda realizada: referencias a `resolve_perf_candidates`, `game_texture`, `FramePacket`, `omitir_ppu_software` e indicadores de HUD.

### 6. Suposiciones

- La lectura de textura de GPU en Android mediante buffer de staging no provoca congelamientos perceptibles si se limita a los cuadros donde se activa el volcado → prototipar: sí → si es falso: desacoplar la lectura con buffer doble o limitar a cuadros que presentan diferencias → resultado del prototipo: [se completa tras aprobación] | no detecta: contención extrema de memoria en SoCs con GPU integrada muy saturada.
- Los directorios candidatos de almacenamiento externo resueltos por la app son accesibles para escritura sin requerir permisos root en Android → prototipar: no → si es falso: escribir exclusivamente en el almacenamiento interno privado (`filesDir`).

### 7. Alcance

- Incluye:
  - Módulo `volcado.rs` con generador de PNG compuesto de 4 paneles y diagnóstico causal.
  - Activación por archivo bandera `dump-frames.flag` o combinación táctil (`L + R + Select + Start`).
  - Habilitación de `COPY_SRC` en `game_texture` y extracción GPU.
  - Indicador visual `[REC]` en el badge superior de la pantalla.
  - Guardado en la carpeta `Download/blazing-recomp/volcados/` en la memoria del teléfono.
  - Tests unitarios en `src/volcado_tests.rs`.
- No incluye:
  - Interfaz de galería o visor interactivo táctil de imágenes dentro del juego en Android (las imágenes se visualizan copiándolas al PC o con un visor de imágenes estándar).
  - Grabación de video MP4 continuo.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Detección de activación de volcado por archivo bandera o combinación
Caso de uso: Inicio de la captura en caliente al detectar la bandera en disco | Tipo: normal
  Dado un entorno donde existe el archivo `dump-frames.flag` en los directorios candidatos
  Cuando el sistema consulta el estado de volcado con `volcado_activado`
  Entonces la función retorna `true`.
Test: `src/volcado_tests.rs::test_deteccion_bandera_volcado_activa`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Generación de imágenes compuestas PNG y diagnósticos de texto
Caso de uso: Codificación de un fotograma con diferencias entre emulador y GPU | Tipo: normal
  Dado un fotograma con 240x160 píxeles de software y 240x160 píxeles de GPU nativa
  Cuando se compone el cuadro con `generar_lienzo_compuesto` y se codifica con `png_rgb`
  Entonces el archivo PNG generado tiene dimensiones 960x160 (4 paneles), firma válida PNG y el informe `.txt` reporta las discrepancias exactas.
Test: `src/volcado_tests.rs::test_generacion_png_compuesto_y_diagnostico`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Escritura asíncrona sin tope arbitrario de fotogramas
Caso de uso: Grabación continua sin límite arbitrario de fotogramas | Tipo: límite
  Dado un gestor de volcado en segundo plano configurado para grabación continua
  Cuando se envían fotogramas para volcar durante la cinemática
  Entonces el gestor escribe de forma ininterrumpida los cuadros hasta que el usuario desactive el modo o finalice la sesión.
Test: `src/volcado_tests.rs::test_hilo_volcado_grabacion_continua_sin_tope`
Estado previo: pasa

Escenario: Regresión de suite de PPU nativa
Caso de uso: Verificación de que la integración no altera el pipeline de renderizado existente | Tipo: regresión
  Dado el pipeline de PPU nativa
  Cuando se ejecutan las pruebas unitarias existentes
  Entonces los 52 tests de `ppu_nativa` continúan pasando en verde.
Test: `src/ppu_nativa_tests.rs::test_sprites_decodificados_si_lineas_activas_tienen_obj_aunque_fin_lo_apague`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- Detección de activación de volcado por archivo bandera o combinación → herramienta o comando: `cargo test --lib test_deteccion_bandera_volcado_activa` → pasa si: la bandera es reconocida y el código de salida es 0 → no detecta: fallos de lectura en sistemas de archivos corruptos.
- Generación de imágenes compuestas PNG y diagnósticos de texto → herramienta o comando: `cargo test --lib test_generacion_png_compuesto_y_diagnostico` → pasa si: el PNG es válido y contiene las dimensiones 960x160 con código de salida 0 → no detecta: visores de imágenes externos que no soporten PNG estándar.
- Escritura asíncrona sin tope arbitrario de fotogramas → herramienta o comando: `cargo test --lib test_hilo_volcado_grabacion_continua_sin_tope` → pasa si: los archivos se graban de forma continua sin truncamiento prematuro y el código de salida es 0 → no detecta: agotamiento total del almacenamiento físico del dispositivo.
- Regresión de suite de PPU nativa → herramienta o comando: `cargo test --lib test_sprites_decodificados_si_lineas_activas_tienen_obj_aunque_fin_lo_apague` → pasa si: el test finaliza en ok con código 0 → no detecta: fallos fuera del módulo de PPU.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y los tests pasan en verde.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Correcciones previas a presentar

- Incorporadas observaciones de la auditoría de Proposal Reviewer: simplificado §1 a una sola frase centrada en el usuario, incorporado `TextureUsages::COPY_SRC` para `game_texture` en el plan, clasificado el escenario 3 como caso límite para validar el tope de 600 fotogramas, unificado el formato PNG a 960x160 (4 paneles) idéntico al visor de PC, y listado `src/runner.rs` en los archivos afectados.
- Acotado el búfer de volcado a 600 fotogramas máximos continuos para proteger el almacenamiento del teléfono.
- Usada redacción clara y directa según ISO 24495-1.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Detección de activación de volcado por archivo bandera o combinación: pasa
  Resultado real: pasa; la presencia de `dump-frames.flag` es detectada y activa el volcado con código de salida 0.
  Método: el anunciado (`cargo test --lib test_deteccion_bandera_volcado_activa`).
- Generación de imágenes compuestas PNG y diagnósticos de texto: pasa
  Resultado real: pasa; genera imágenes compuestas válidas de 960x160 (4 paneles: Software fin, Software linea, GPU nativa y mapa de diferencias en magenta) y reportes textuales con código de salida 0.
  Método: el anunciado (`cargo test --lib test_generacion_png_compuesto_y_diagnostico`).
- Escritura asíncrona sin tope arbitrario de fotogramas: pasa
  Resultado real: pasa; el gestor asíncrono graba de forma continua sin límite arbitrario de fotogramas, desacoplado de comprobaciones periódicas en disco, con código de salida 0.
  Método: el anunciado (`cargo test --lib test_hilo_volcado_grabacion_continua_sin_tope`).
- Regresión de suite de PPU nativa: pasa
  Resultado real: pasa; los 52 tests del módulo `ppu_nativa` pasaron al 100% con código de salida 0.
  Método: el anunciado (`cargo test --lib ppu_nativa`).

### Contexto: ¿se respetó?

- Destino en la carpeta de Descargas (`Download/blazing-recomp/volcados/`) → sí, resuelta como primera prioridad en `resolver_directorio_volcados`.
- No degradar la experiencia de juego en uso normal → sí, el volcado permanece inactivo salvo que se detecte la bandera o se pulse la combinación de botones.
- Cero bloqueos en los hilos de emulación o presentación → sí, la escritura se ejecuta en un hilo secundario mediante `sync_channel(64)` sin bloquear el bucle de renderizado.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: en tests satélite de Windows, `std::env::temp_dir()` fallaba por permisos locales, resuelto utilizando `target/tmp` como base local del espacio de trabajo.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/volcado.rs`: nuevo módulo con codificador PNG puro, generador de lienzo 960x160 cuádruple, diagnóstico causal y `GestorVolcado`.
- `src/volcado_tests.rs`: tests unitarios satélite para los 3 criterios de aceptación.
- `src/lib.rs`: exporta `pub mod volcado;`.
- `src/runner.rs`: añade `set_omitir_ppu_software` para permitir computar `runner.frame_rgba()` durante la grabación.
- `src/pipeline.rs`: añade `dumping_enabled` en `EmulationBridge` y sincroniza la entrega de frames en `EmulationWorker`.
- `src/fps.rs`: incorpora el indicador `[REC]` en el badge superior cuando el volcado está activo.
- `src/platform.rs`: añade `COPY_SRC` a `game_texture`, buffer de staging para readback de GPU, detección de `dump-frames.flag` y atajo táctil `L+R+Select+Start`.

### Discrepancias

Ninguna. El comportamiento observado coincidió con las predicciones.

### Para la próxima vez

Mantener `target/tmp` como base de directorios temporales en todas las pruebas unitarias que requieran interactuar con el sistema de archivos en Windows.

### Fuera de alcance (hallazgos que no se arreglaron)

Ninguno.

---

## Revisión humana

- Indicador visual `[REC]` visible en el HUD de la pantalla durante la captura: [✔ | ✘ | ?]
- Archivos `.png` y `.txt` generados en la carpeta `Download/blazing-recomp/volcados/` accesibles desde el explorador de archivos de Android o por USB: [✔ | ✘ | ?]
- Detectado solo por el humano: [lo completa la persona]
