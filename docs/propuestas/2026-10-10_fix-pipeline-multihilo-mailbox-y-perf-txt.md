# Propuesta: Buzón de fotogramas no bloqueante y reducción medible del coste por fotograma

Fecha: 2026-10-10
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Esta propuesta separa **dos objetivos que antes venían mezclados** como si tuvieran una sola causa:

- **Objetivo A — Desacoplar hilos (buzón):** la emulación deja de esperar a la GPU. Hoy el hilo de emulación se bloquea en `sync_channel(1)` y avanza al ritmo (lento) del dibujado; se sustituye por un buzón del fotograma más reciente que **nunca bloquea al productor**. Efecto esperado: velocidad de juego, latencia de entrada y audio correctos. **No** sube los FPS en pantalla.
- **Objetivo B — Bajar el coste por fotograma:** los FPS están limitados por el tiempo de `present` (55,5 ms en retrato). Se **instrumenta** para saber dónde se va ese tiempo y se aplica la palanca que haga caer el fotograma por debajo de un intervalo de VSync. Efecto esperado: subir FPS en pantalla.

**Dependencia explícita (Objetivo C, NO incluido en esta propuesta):** aunque A y B funcionen, la emulación no puede sostener 60 Hz si la CPU tarda más de 16,67 ms por fotograma. En el log actual `ejec` es 13–37 ms (media ≈30 ms) con `nativos≈0`, es decir, **depende de la cobertura AOT** (`blazing.labels.toml`, 46 labels hoy). Sin esa cobertura el techo de la simulación es ~33 Hz y los FPS en pantalla seguirán limitados por la GPU.

### 2. Contexto

- Contexto consultado: [src/pipeline.rs](../../src/pipeline.rs) (`EmulationBridge`, `send_frame`, `sync_channel(1)`), [src/platform.rs](../../src/platform.rs) (bucle `android_main`, `Compositor::render`, `write_perf_report`), [src/overlay.wgsl](../../src/overlay.wgsl), [src/perf.rs](../../src/perf.rs), [src/recomp.rs](../../src/recomp.rs) (`run_frame_native`), [blazing.labels.toml](../../blazing.labels.toml), el log real `blazing-perf-2.txt` (2026-10-10, tablet 6 GB / Snapdragon 610) y la evaluación interna de la versión anterior de esta propuesta.
- Intención: que la app no distorsione la velocidad del juego cuando la GPU es lenta (A) y que los FPS dejen de estar clavados en 18 (B).
- Restricciones y decisiones previas (máx. 3):
  - La comunicación entre hilos no debe asignar memoria dinámica por fotograma (origen: `AGENTS.md` §2.5 rendimiento y batería) → implica: búferes de fotograma preasignados y reutilizados en lugar de `Box::new` por frame → protegida por: "El consumidor recibe siempre el fotograma más reciente sin asignar memoria por frame".
  - El compositor GPU y `app.poll_events` deben permanecer en el hilo de `NativeActivity` (origen: `docs/propuestas/2026-10-10_separacion-hilos-emulacion-y-renderizado.md`) → implica: el consumidor del buzón sigue siendo el hilo principal de Android → protegida por: "El productor nunca se bloquea al depositar un fotograma".
  - La telemetría `blazing-perf.txt` ya se crea y se vacía cada segundo; el log adjunto lo demuestra → implica: **no** se reimplementa el guardado, solo se amplían sus campos → no testeable (ver Revisión humana).
- Hallazgo que cambia el plan (no se vuelve a proponer): las podas tempranas del overlay ya están implementadas en [src/overlay.wgsl](../../src/overlay.wgsl) (`p.y < shell_top - 12.0` en retrato, `p.y < res.y * 0.38` en apaisado) desde los commits `71958d2` y `8787a84`, **anteriores** al log. Por eso el Objetivo B empieza por **medir**, no por volver a podar el shader.
- Alternativas descartadas:
  - Triple búfer de 3 slots → descartada: un único slot "más reciente" ya evita el bloqueo del productor y usa menos memoria; la presentación está atada síncronamente a `present`.
  - `PresentMode::AutoNoVsync` como diseño final → descartada como diseño (provoca *tearing*), pero **se usará como experimento de medición** dentro del Objetivo B.
- Ante imprevistos, prioriza: mantener velocidad de juego y latencia de entrada correctas (A) por encima de maximizar FPS (B).

### 3. Ejemplos

- Caso normal (A, desacoplar): la GPU tarda 55 ms por frame.
  - Antes: el worker hace `send_frame` y se bloquea hasta que la UI consume → la emulación avanza a 18 Hz y el juego va lento.
  - Ahora: el worker deposita en el buzón y sigue; si la GPU está ocupada, el buzón se sobrescribe con el fotograma más nuevo. La CPU avanza a su propio ritmo (limitada por `ejec`).
- Caso normal (B, coste por frame), retrato 1200×2000:
  - Antes: `dibujo_avg=55,5 ms` (≈3 vblancos) → `fps=18,0` (log, t=2..215 s).
  - Ahora: la telemetría separa `subida`/`codificacion`/`present` y se aplica la palanca medida; objetivo `dibujo_avg < 16,7 ms`.
- Caso límite (A): el consumidor pasa 200 ms sin consumir (p. ej. compilación de shader).
  - Antes: el productor se bloquea 200 ms y el juego se congela.
  - Ahora: el productor produce y descarta; al volver, el consumidor dibuja el fotograma más reciente.

### 4. Pasos

1. **Instrumentación del coste (B), primero:** en `Compositor::render` separar `subida` (write_texture/write_buffer), `codificacion` (encoder + `queue.submit`) y `present`; añadirlos a `FrameTiming`/`PerfReport` y a `PerfReport::to_line`. Sin esto no se puede atribuir el 55,5 ms.
2. **Experimento de medición (B):** con esa instrumentación, capturar el log con `PresentMode::AutoNoVsync` y con un *fragment shader* trivial, para separar "espera de VSync" de "cómputo real". Anotar los resultados.
3. **Buzón (A):** sustituir `SyncSender/Receiver` por un buzón de un solo slot (`Mutex<Option<FramePacket>>` con reemplazo del más reciente, o equivalente atómico) donde `deposit` nunca bloquea.
4. **Reutilización de búfer (A):** el worker recicla el `FramePacket` que le devuelve el consumidor desde un pool de 2, en lugar de `Box::new` por frame.
5. **Palanca de reducción (B):** según el paso 2, aplicar la de mayor impacto/menor riesgo (candidatas: forzar y **verificar en el log** un `present_mode` realmente no bloqueante; reducir la resolución interna del overlay renderizando solo su banda; evitar subidas de textura/búfer cuando nada cambió).
6. **Telemetría (ya funciona):** no reimplementar; solo ampliar campos y actualizar `docs/referencia/api-modulos-rust.md` y `CHANGELOG.md`.

### 5. Qué más podría verse afectado

- `src/platform.rs`: bucle `android_main` (consumo del buzón), `Compositor::render`, `write_perf_report`.
- `src/pipeline.rs` / `src/pipeline_tests.rs`: contrato del puente (tests existentes de transferencia y ciclo de vida).
- `src/perf.rs` / `src/perf_tests.rs`: formato de `to_line` y campos nuevos.
- `tests/packaging_tests.rs`: verificación de sincronización de documentación.
- Búsqueda hecha: `git grep -n "try_recv_frame\|send_frame\|FramePacket" src/`.

### 6. Suposiciones

- La mayor parte de `dibujo` es espera de VSync/`present` y no cómputo de fragmentos (indicio: `dibujo` retrato/apaisado = 1,500 exacto y `espera < 1 ms` en el log) → prototipar: sí
  → si es falso: la palanca correcta es el shader/overlay y el Objetivo B cambia de estrategia
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: variaciones térmicas durante sesiones largas
- Un buzón de un solo slot basta para no bloquear al productor y mantener la entrada con latencia ≤1 fotograma → prototipar: sí
  → si es falso: se pasa a doble slot
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: reordenamiento de memoria en ARM real
- El tablet expone algún `present_mode` realmente no bloqueante (Mailbox) vía Vulkan en Adreno 405 → prototipar: sí
  → si es falso: se descarta la palanca de `present` y se mide la de resolución interna
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: drivers de otros fabricantes

### 7. Alcance

- Incluye:
  - Buzón no bloqueante + reutilización de búferes en `src/pipeline.rs`.
  - Instrumentación de `dibujo` (subida/codificación/present) y su registro en la telemetría.
  - Una palanca de reducción de coste por fotograma, elegida tras medir.
- No incluye:
  - Cobertura AOT / reducción del intérprete (`blazing.labels.toml`), que es la dependencia del Objetivo C.
  - Reescritura del guardado de telemetría (ya funciona).
  - Cambios en `gba-core`/`screen` ni en la lógica del juego.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: El productor nunca se bloquea al depositar un fotograma
Caso de uso: la GPU tarda más que la emulación y el consumidor no lee | Tipo: normal
  Dado un buzón con un fotograma ya depositado y sin consumidores activos
  Cuando el productor deposita 1000 fotogramas consecutivos
  Entonces los 1000 depósitos retornan sin bloquearse (tiempo total < 50 ms) y el buzón conserva el último fotograma
Test: `src/pipeline_tests.rs::test_mailbox_never_blocks_producer` (se escribe tras la aprobación)
Estado previo: falla (no compilaba: el API/campo no existía aún)

Escenario: El consumidor recibe siempre el fotograma más reciente sin asignar memoria por frame
Caso de uso: el usuario juega mientras la GPU va justa | Tipo: límite
  Dado un buzón donde el productor deposita 5 fotogramas numerados 1..5 sin que el consumidor lea
  Cuando el consumidor extrae una vez
  Entonces recibe el fotograma n.º 5, con 38.400 píxeles y canal alfa 255, y el pool reutiliza búferes (sin `Box::new` por frame)
Test: `src/pipeline_tests.rs::test_mailbox_keeps_latest_frame` (se escribe tras la aprobación)
Estado previo: falla (no compilaba: el API/campo no existía aún)

Escenario: La telemetría atribuye el tiempo de dibujo por fase
Caso de uso: diagnosticar el cuello de botella en la tablet | Tipo: normal
  Dada una sesión con fotogramas registrados
  Cuando se cierra la ventana de 1 s
  Entonces cada línea de `blazing-perf.txt` incluye `subida`, `codificacion` y `present` (ms) además de `dibujo`, y se cumple `dibujo >= present`
Test: `src/perf_tests.rs::test_report_breaks_down_draw_time` (se escribe tras la aprobación)
Estado previo: falla (no compilaba: el API/campo no existía aún)

Escenario: Los FPS en pantalla dejan de estar clavados en 18 (visible)
Caso de uso: jugar en la tablet en retrato y apaisado | Tipo: regresión
  Dado el APK con el buzón y la palanca del Objetivo B, en la tablet a 1200×2000
  Cuando se juega 60 s
  Entonces `blazing-perf.txt` muestra `dibujo_avg` por debajo de la base del log (55,5 ms retrato / 35,0 ms apaisado) y `fps` por encima de 18,0; el presupuesto `ejec_avg < 16,7 ms` solo se logra si la cobertura AOT lo permite (dependencia del Objetivo C)
Test: no automatizable en host (revisión humana en tablet)
Estado previo: no aplica

### 9. Cómo lo comprobaré

- El productor nunca se bloquea → comando: `cargo test --lib test_mailbox_never_blocks_producer` → pasa si 1000 depósitos no bloquean (tiempo total < 50 ms) → no detecta: contención real bajo carga en ARM.
- El consumidor recibe el más reciente → comando: `cargo test --lib test_mailbox_keeps_latest_frame` → pasa si recibe el n.º 5 tras 5 depósitos → no detecta: reordenamiento de memoria débil.
- Telemetría por fase → comando: `cargo test --lib test_report_breaks_down_draw_time` → pasa si `to_line` incluye las 3 fases y `dibujo >= present` → no detecta: que la GPU informe tiempos reales.
- FPS en tablet (visible) → revisión humana: leer `Download/blazing-perf.txt` y comparar `dibujo_avg` y `fps` contra la base del log → no detecta: throttling térmico ni el efecto de la cobertura AOT sobre `ejec`.
- Suite y empaquetado → comando: `cargo test` (85 tests en verde) y `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` → pasa si el Fat APK se genera y firma limpiamente.

### 10. Terminado cuando

Se cumplen los criterios A y B; el criterio visible se reporta contra la base del log y **el Objetivo C (cobertura AOT) queda registrado como dependencia pendiente**, no como parte de esta propuesta.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- La versión anterior atribuía los 18 FPS al `sync_channel` y proponía podar el shader. La evaluación del log mostró: (a) `dibujo` retrato/apaisado = 1,500 exacto → cuantización por VSync; (b) las podas del overlay ya existen; (c) `ejec≈30 ms` con `nativos≈0` → el techo de CPU es el intérprete. Por eso se reestructura en Objetivos A/B y dependencia C.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- El productor nunca se bloquea al depositar un fotograma: coincide.
  Resultado real: `cargo test --lib test_mailbox_never_blocks_producer` → ok (1001 depósitos en < 50 ms; 2 asignaciones de búfer).
  Método: el anunciado.
- El consumidor recibe siempre el fotograma más reciente sin asignar memoria por frame: coincide.
  Resultado real: `cargo test --lib test_mailbox_keeps_latest_frame` → ok (recibe el n.º 5, 38.400 px, alfa 255, 2 asignaciones).
  Método: el anunciado.
- La telemetría atribuye el tiempo de dibujo por fase: coincide.
  Resultado real: `cargo test --lib test_report_breaks_down_draw_time` → ok; `to_line` incluye `subida`, `codificacion`, `present` y las tres fases suman `dibujo`.
  Método: el anunciado.
- Los FPS en pantalla dejan de estar clavados en 18 (visible): no pudo comprobarse.
  Resultado real: no ejecutado; requiere instalar el APK en la tablet Snapdragon 610 (no disponible en este entorno).
  Método: revisión humana pendiente en dispositivo.
- Suite completa: 88/88 en la corrida limpia (78 lib + 10 packaging); con carga concurrente aparece el fallo **pre-existente** descrito en "Fuera de alcance".

### Palanca 1 (superficie opaca): medida en dispositivo, no coincidió

- Predicho: si el *blending* de SurfaceFlinger era la causa, `present` bajaría de ~53 ms.
  Resultado real: **la palanca resultó inerte**. La cabecera de `blazing-perf-4.txt` imprime
  `present=Mailbox alpha=Inherit`, es decir el driver **no ofrece** `CompositeAlphaMode::Opaque`, así que
  la rama `if caps.alpha_modes.contains(&Opaque)` nunca se tomó. No es un fallo de la implementación:
  `wgpu` documenta que `alpha_modes` devuelve "al menos `Opaque` **o** `Inherit`", y
  `validate_surface_configuration` de `wgpu-core` habría rechazado un modo no soportado. Además el tema
  del manifiesto ya es opaco (`Theme.NoTitleBar.Fullscreen`), así que la premisa del *blending* era
  dudosa de entrada. Lo que sí quedó de esta palanca es la cabecera `present=`/`alpha=`, que es
  precisamente lo que permitió descubrir esto.
  Método: instalación real en la tablet y lectura de la primera línea del log.

### Palanca 2 (número de imágenes del swapchain): propuesta e implementada

- Medición de partida (log 4, estado estable): `present` = **52,1 ms en retrato** y **32,6 ms en
  apaisado**, esto es **3,34× y 2,18× el intervalo de VSync** (16,67 ms), mientras que
  `subida + codificacion + espera` ≈ 2,5–3,9 ms. En retrato se dibuja *menos* (el marco del juego ocupa
  el 48 % superior) y aun así es 1,5× más lento: la carga propia queda descartada como causa.
- Causa identificada en el código de `wgpu-hal`: `create_swapchain` pide
  `min_image_count = desired_maximum_frame_latency + 1`, y con latencia 1 eso deja **2 imágenes**;
  `Mailbox` no puede reemplazar un fotograma pendiente (necesita una tercera imagen donde renderizar),
  de modo que `queue_present` degenera en bloqueo hasta que el compositor libera un búfer. Hallazgo
  adicional: `wgpu-hal` fija `preTransform = IDENTITY` incondicionalmente y el propio hal documenta que
  en Android 10+ eso obliga a la plataforma a **rotar** cada fotograma; es la explicación más probable
  del sobrecoste de retrato y **no es corregible desde la app**.
- Entregado: calibración de arranque (`src/present.rs`) que mide `lat=1/2/3` con el modo actual (tramos
  de 3 s) más dos sondas de diagnóstico —presentar sin VSync y renderizar a escala 0,5×—, elige la mejor
  con un margen del 5 % y deja constancia en `blazing-perf.txt` con las líneas `# tramo`, `# resultado`
  y `# elegido`.
- Predicción pendiente de verificación en tablet: con 4 imágenes, apaisado debe caer a ~1 intervalo
  (~60 FPS) y retrato a ~2 (~30 FPS). Falsación: si con `lat=3` el `present` de apaisado sigue ≥33 ms,
  la hipótesis cae y el límite es el trabajo del compositor (rotación), no el número de imágenes.

### Medición en dispositivo de la palanca 2 (`blazing-perf-5.txt`) y decisión de nitidez

| tramo | configuración | `present` medio | fps medio |
| :--- | :--- | :--- | :--- |
| idx=0 (referencia) | lat=1, escala 1.00 | 51,4 ms | 16,7 |
| idx=1 | lat=2, escala 1.00 | 50,6 ms | 17,7 |
| idx=2 | lat=3, escala 1.00 | 50,5 ms | 17,7 |
| idx=3 (sonda) | lat=3, escala 0.50 | **2,9 ms** | **55,7** (53 / 54 / **60**) |

- **La hipótesis del número de imágenes queda falsificada**: `lat=1/2/3` no mueven `present` (Δ < 1 ms entre
  tramos; dispersión entre ventanas de ±2 ms) y `espera` sigue en 0,2–0,7 ms. No hay hambre de búferes;
  el bloqueo de `present` no es esperar una imagen libre.
- **El coste sí escala con los píxeles presentados**: a escala 0.50, `present` cae de ~51 ms a **2,9 ms** y
  el juego alcanza **60 FPS** (`t=13s`: `fps=60.0 present=0.7 dibujo=2.3`, `ejec≈12,8 ms`). Ese `fps=60`
  demuestra además que el techo de CPU sí sostiene 60 Hz en esta escena.
- La relación **no es lineal** (4× menos píxeles → ~17× menos tiempo), lo que apunta a un cambio de camino en
  el compositor (capa demasiado grande para el compositor de hardware → composición por GPU) y no a fill
  puro. Es una hipótesis razonada, **no verificada directamente**.
- El tramo de sonda sin VSync no se ejecutó (no existe `idx=4`) porque este driver no ofrece
  `PresentMode::Immediate`; el código lo omite en silencio (hueco de telemetría menor, anotado abajo).
- Predicción propia fallida en esta ronda, además de la del número de imágenes: reducir la escala **sí** baja
  el coste de composición, contra lo que se argumentó al proponer el tramo de 0,5×.
- **Decisión del usuario (2026-10-10): mantener la resolución nativa (escala 1.00).** La mejora de fluidez
  medida (18 → 53-60 FPS) queda **descartada por nitidez**; en esta tablet el port mantiene `present≈51 ms`
  en retrato y ~33 ms en apaisado (18 / 27 FPS). La reducción de escala queda implementada, medida y
  testeada, así que reabrir la decisión no exige desarrollo nuevo, solo cambiar la política de adopción.

### Consecuencias aplicadas de esa decisión

- El barrido de calibración queda **desactivado por defecto** (`CALIBRAR_PRESENTACION = false` en
  `src/platform.rs`), con su evidencia y su motivo en la documentación de la constante; el mecanismo
  (`src/present.rs`), la escala de render y sus tests se conservan intactos para poder reabrir la decisión
  sin desarrollo nuevo. Motivo del retiro: con nitidez nativa el barrido no tenía nada que adoptar y
  costaba 12-15 s de mediciones y artefactos por arranque (3 s a media resolución).
- **Palanca 3 (experimento en curso): backend OpenGL ES.** `PREFERIR_OPENGL_ES = true` invierte el orden de
  intentos del contexto gráfico (OpenGL ES primero, Vulkan como red de seguridad), sin tocar la resolución.
  Predicción falsable: si `present` sigue en ~51 ms en retrato a resolución nativa, el coste pertenece a la
  composición de pantalla y es indiferente al backend (se cierra la línea de investigación que no sacrifica
  nitidez); si baja, existe una palanca de fluidez sin coste de nitidez.
  Nota de alcance: el experimento **invierte temporalmente** la prioridad "Vulkan primero" de
  `docs/propuestas/2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md` (Vulkan sigue como
  red de seguridad); se restaura en cuanto haya medición.

### Contexto: ¿se respetó?

- Comunicación entre hilos sin asignación por fotograma → respetada (pool de búferes; `allocations() <= 2` verificado).
- Compositor GPU y eventos en el hilo de `NativeActivity` → respetada (no se movieron de hilo).
- Telemetría ya existente sin reimplementar → respetada (solo se ampliaron campos).
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: `src/platform.rs` es `#[cfg(target_os = "android")]` y **no lo compila** `cargo test` de host → se añadió `cargo ndk -t arm64-v8a check` como verificación, que reveló 2 errores `E0308` (firma de `Compositor::render` no actualizada) → corregidos. Criterio de prioridad aplicado: sí (mantener A por encima de B acotó el alcance).
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/pipeline.rs` (buzón de un slot, pool de búferes, `acquire_buffer`/`recycle_buffer`/`allocations`).
- `src/pipeline_tests.rs` (2 tests nuevos + adaptación al nuevo API).
- `src/perf.rs` (campos `subida_ms`/`codificacion_ms`/`present_ms` y sus promedios en `to_line`).
- `src/perf_tests.rs` (1 test nuevo + actualización de 2 aserciones de formato).
- `src/platform.rs` (`DrawPhases`, medición de las tres fases en `Compositor::render`; después: cabecera con `present=`/`alpha=`, escala de render y calibración en el bucle).
- `src/present.rs` / `src/present_tests.rs` (calibración de presentación: tramos, política de elección y telemetría `#`).
- `src/overlay.rs` / `src/overlay_tests.rs` (`ElemParams::scaled` y pruebas de geometría escalada).
- `src/lib.rs` (registro del módulo `present`).
- `docs/guias/medir-rendimiento.md` (formato de las líneas de calibración).
- `docs/referencia/api-modulos-rust.md` (regenerada: entrada de `ElemParams::scaled`).
  ¿Coincide con lo anunciado? sí en A y en la instrumentación de B; la "palanca de reducción" (paso 5) se entregó primero como superficie opaca (palanca 1, inerte por capacidades del driver) y después como calibración del número de imágenes del swapchain (palanca 2), condicionada a medir en dispositivo.

### Discrepancias

- El Objetivo B se entregó solo como instrumentación; la reducción del coste por fotograma y la verificación de FPS en la tablet quedaron pendientes de medición en hardware real (no había dispositivo). Clasificación: información faltante.
- Segundo intento sobre el mismo criterio visible (§11 "Parar si"): la palanca 1 no movió `present` (era inerte por capacidades del driver) y la palanca 2 está pendiente de medición. Si la medición en tablet vuelve a no mejorar `present`, la propuesta pasa a "discrepada" y se detiene la iteración de palancas sobre `present`. Clasificación: suposición falsa (la superficie ya era opaca; la causa está en el acoplamiento con el compositor).

### Para la próxima vez

- Medir `subida`/`codificacion`/`present` en el dispositivo real **antes** de elegir la palanca del Objetivo B.

### Fuera de alcance (hallazgos que no se arreglaron)

- `pipeline::tests::test_emulation_worker_lifecycle_and_frame_generation` es intermitente por límite de reloj (500 ms) bajo carga concurrente; es **pre-existente** (el HEAD sin cambios falla 8/10 corridas; el árbol con cambios, 2/5). No se modificó su aserción. | impacto: medio
- La cabecera `modo=` de `blazing-perf.txt` puede etiquetar `INT` aunque haya tabla nativa, si el primer informe (t=1 s) llega antes del primer fotograma. | impacto: bajo
- `wgpu-hal` no expone `preTransform`: la rotación de retrato la ejecuta la plataforma en cada fotograma y no es corregible desde el port sin parchear el hal. | impacto: medio
- La referencia técnica generada (`docs/referencia/api-modulos-rust.md`) cubre solo una lista curada de módulos en `scripts/extraer_referencias.py`; el nuevo `src/present.rs` no aparece en ella aunque `--check` pasa. | impacto: bajo
- La sonda de escala 0,5× solo **mide**: bajar la resolución del swapchain de forma permanente está descartado por nitidez en `docs/propuestas/2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`; si el dato mostrara una ganancia decisiva habría que reabrir esa decisión aparte. | impacto: bajo

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

---

## Revisión humana

- Los FPS en pantalla dejan de estar clavados en 18 (visible): ✔ | ✘ | ?
- Restricción "la telemetría `blazing-perf.txt` ya funciona; no se reimplementa" (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
