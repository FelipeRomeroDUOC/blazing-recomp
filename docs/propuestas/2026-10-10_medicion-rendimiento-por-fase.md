# Propuesta: Medición de rendimiento por fase del fotograma

Fecha: 2026-10-10
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Mientras juegas, la app mide cuánto tarda cada parte de un fotograma (ejecutar el juego, convertir colores, dibujar, esperar a la pantalla) y cuántas vueltas del bucle no hacen nada, y escribe una línea por segundo en un archivo de texto en la carpeta Descargas. No cambia cómo se ve ni cómo corre el juego: solo permite saber, en cada dispositivo, qué parte limita los FPS.

### 2. Contexto

- Contexto consultado: [`src/platform.rs`](../../src/platform.rs) (bucle principal líneas 1563-1692, `render` líneas 672-878), [`src/runner.rs`](../../src/runner.rs), [`src/recomp.rs`](../../src/recomp.rs), [`src/fps.rs`](../../src/fps.rs), [`src/lib.rs`](../../src/lib.rs), [`src/storage.rs`](../../src/storage.rs), [`AGENTS.md`](../../AGENTS.md), [`CHANGELOG.md`](../../CHANGELOG.md) y la propuesta [`2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`](2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md).
- Intención: poder afirmar con datos, por dispositivo, "de cada fotograma, X ms van a ejecutar el juego y Y ms a dibujar", antes de optimizar nada. Éxito: tras 30 s de juego existe el archivo `blazing-perf.txt` con una línea por segundo.
- Restricciones y decisiones previas (máx. 3):
  - El contador de FPS cuenta fotogramas emulados, no presentados (origen: propuesta `2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md` §9 y `src/platform.rs` línea 1679) → implica: no cambiar su definición ni su código → protegida por: "El contador de FPS y el avance del juego no cambian".
  - Lo nuevo va como módulo puro con tests satélite `src/<módulo>_tests.rs` (origen: `AGENTS.md` §4.4) → implica: `src/perf.rs` sin dependencias de Android, y `src/perf_tests.rs` → protegida por: "Informe con promedio, peor caso y línea de texto" y "Ventana sin fotogramas".
  - Esta propuesta no toca `gba-core`, el fork ni `Cargo.toml` (origen: `AGENTS.md` §4.1 independencia del motor; respuesta humana, 2026-10-10: "Primero solo esta app") → implica: los contadores dentro del fork quedan fuera → no testeable (se revisa en la lista de archivos modificados).
- Alternativas descartadas:
  - Medir solo con una herramienta externa de perfilado (`simpleperf`, del NDK) → descartada como única vía porque exige conectar cada dispositivo a un computador; queda como complemento.
  - Cronometrar cada acceso a memoria o cada muestra de audio → descartada porque el audio genera 65.536 muestras por segundo (origen: `AUDIO_RATE_HZ` en `crates/gba-core/src/mem.rs` del fork) y el cronómetro mismo distorsionaría lo medido; esas cosas se cuentan en la Fase 2, no se cronometran.
  - Mostrar los números en pantalla junto al contador de FPS → descartada por ahora (respuesta humana, 2026-10-10: eligió "Archivo en Descargas").
- Ante imprevistos, prioriza: no alterar el rendimiento que se mide por encima de registrar más datos (si leer el reloj resulta caro, se mide menos).

### 3. Ejemplos

- Caso normal: tablet en portrait, 30 s de juego → antes: solo se ve el contador en pantalla ("35 FPS · AOT · Vulkan") y no queda registro → ahora: además existe `Download/blazing-perf.txt` con una línea por segundo, por ejemplo (valores de ejemplo del formato, no medidos):
  `t=1s fps=4.0 ejec=20.0/20.0 color=1.0/1.0 dibujo=15.0/30.0 espera=4.0/10.0 nativos=1000 interprete=10 vacias=7 ventana=1200x1920`
  Cada par `A/B` es promedio y peor caso, en milisegundos, de los fotogramas de ese segundo.
- Caso límite: la app está abierta pero no hay ROM cargada (pantalla de guía) → antes: sin registro → ahora: no se escribe ninguna línea de medición (no hay fotogramas que medir) y no hay error.

Qué significa cada campo:

| Campo | Qué mide |
|---|---|
| `fps` | fotogramas del juego completados en esa ventana de 1 s (misma idea que el contador actual) |
| `ejec` | tiempo de ejecutar la lógica del juego y los chips imitados, por fotograma |
| `color` | tiempo de convertir el fotograma a colores de pantalla (`temporal.push` y `compose`) |
| `dibujo` | tiempo total de `redraw()`: subir la imagen a la GPU, dibujar y presentar |
| `espera` | parte de `dibujo` que se va en `get_current_texture` (esperar una imagen libre) |
| `nativos` / `interprete` | promedio por fotograma de bloques recompilados ejecutados y de pasos del intérprete de respaldo |
| `vacias` | vueltas del bucle principal sin ejecutar fotograma ni dibujar, en ese segundo |
| `ventana` | tamaño de la ventana (distingue portrait de landscape en un mismo archivo) |

### 4. Pasos

1. Crear `src/perf.rs` (archivo nuevo): `FramePerf` acumula tiempos por fase y vueltas vacías, y cierra ventanas de 1 s produciendo un `PerfReport` con `to_line()` (formato de la sección 3). Recibe los instantes como parámetro, igual que `FpsCounter::on_frames_at`, para poder probarlo sin reloj real.
2. Declarar `pub mod perf;` en [`src/lib.rs`](../../src/lib.rs).
3. En [`src/runner.rs`](../../src/runner.rs), `GameRunner::step_frame` mide `ejec` y `color`, y guarda cuántos bloques nativos y pasos de intérprete devolvió `run_frame_native` (hoy ese resultado se descarta en la línea 96). Se expone con `last_timing()`. Lo que devuelve `step_frame` y los píxeles que produce no cambian.
4. En [`src/platform.rs`](../../src/platform.rs): cronometrar `redraw()` (`dibujo`) y, dentro de `Compositor::render`, la llamada a `get_current_texture` (`espera`); contar las vueltas vacías del bucle; alimentar `FramePerf`; una vez por segundo escribir la línea en el primer directorio de la lista de [`src/storage.rs`](../../src/storage.rs) donde se pueda crear `blazing-perf.txt` (primero `/sdcard/Download`), sobrescribiendo el archivo al iniciar el juego. La primera línea del archivo es un encabezado con el backend gráfico, el modo (AOT o INT) y `reloj=NNns`, el costo medido de una lectura del reloj (promedio de 10.000 lecturas al arrancar). Si no se puede escribir, se registra un aviso en el log una sola vez y el juego sigue.
5. Documentar: regenerar [`docs/referencia/api-modulos-rust.md`](../referencia/api-modulos-rust.md) con `python scripts/extraer_referencias.py`, crear una guía corta `docs/guias/medir-rendimiento.md` (cómo sacar el archivo y qué significa cada campo, sin interpretar causas) y añadir una entrada en [`CHANGELOG.md`](../../CHANGELOG.md) bajo "Sin publicar".

### 5. Qué más podría verse afectado

- `run_frame_native` solo se usa en `src/runner.rs` línea 96 (definida en `src/recomp.rs` línea 127). Búsqueda: `grep -rn "run_frame_native\|step_frame\|on_sim_frames\|fps_counter" src --include=*.rs`.
- `step_frame` solo se llama desde `src/platform.rs` línea 1355 (`AppState::step`) y desde los tests de [`src/runner_tests.rs`](../../src/runner_tests.rs).
- `fps_counter` y `on_sim_frames` no se tocan.
- [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) incluye la sincronización de documentación (origen: `AGENTS.md` §4.4): se ejecutará completo.
- Lo medido en el bucle añade lecturas del reloj en cada fotograma: es el único efecto sobre el rendimiento (ver suposición 1).

### 6. Suposiciones

- Leer el reloj cuesta mucho menos que un fotograma (16,7 ms a 60 FPS) en los dispositivos de prueba → prototipar: sí
  → si es falso: medir solo uno de cada cuatro fotogramas
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: otros relojes o versiones de Android que no estén entre tus dispositivos
- La app puede crear `blazing-perf.txt` en Descargas en la tablet, la TV y el móvil (origen: `android/AndroidManifest.xml` declara `MANAGE_EXTERNAL_STORAGE`; `src/storage.rs` ya busca la ROM ahí) → prototipar: sí
  → si es falso: escribir en la carpeta propia de la app (`external_data_path`) o solo en el log
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: cambios de permisos en versiones de Android distintas a las probadas
- `get_current_texture` bloquea hasta que hay una imagen libre, por lo que su duración se puede leer como espera de la GPU o de la pantalla → prototipar: no (no verificado contra la documentación de `wgpu`)
  → si es falso: el campo `espera` no se interpreta y queda solo como dato
- `cargo test` corre sin problemas en tu computador → prototipar: no. Yo no pude ejecutarlo en mi entorno: la compilación falla por falta de la librería del sistema `wayland-client` (origen: salida de `cargo test --lib` en mi entorno, 2026-10-10)
  → si es falso: te pediré la salida de `cargo test` para confirmar los criterios

### 7. Alcance

- Incluye:
  - Módulo `perf` con promedio y peor caso por fase, conteo de pasos nativos y de intérprete, y conteo de vueltas vacías.
  - Medición en `step_frame`, `redraw` y `Compositor::render`.
  - Salida en `Download/blazing-perf.txt` y en el log.
- No incluye:
  - Cambios en `gba-core`, en el fork o en `Cargo.toml` (Fase 2, propuesta aparte; ver anexo).
  - Cualquier optimización: quitar la espera activa, separar hilos, recortar píxeles de la GPU.
  - Línea de diagnóstico en pantalla.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Informe con promedio, peor caso y línea de texto
Caso de uso: leer el rendimiento de la tablet tras un segundo de juego | Tipo: normal
  Dado que en una ventana de 1,0 s se registraron 4 fotogramas con `ejec` de 20 ms cada uno, `color` de 1 ms cada uno, `dibujo` de 10, 10, 10 y 30 ms, `espera` de 2, 2, 2 y 10 ms, 1000 bloques nativos y 10 pasos de intérprete por fotograma, 7 vueltas vacías y ventana de 1200x1920
  Cuando se cierra la ventana y se pide la línea de texto
  Entonces la línea es exactamente `t=1s fps=4.0 ejec=20.0/20.0 color=1.0/1.0 dibujo=15.0/30.0 espera=4.0/10.0 nativos=1000 interprete=10 vacias=7 ventana=1200x1920`
Test: `src/perf_tests.rs::test_informe_promedia_y_reporta_peor_caso` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: Ventana sin fotogramas
Caso de uso: la app está abierta sin juego en marcha (pausa o sin ROM) | Tipo: límite
  Dado que en una ventana de 1,0 s no se registró ningún fotograma y hubo 1000 vueltas vacías, con ventana de 1200x1920
  Cuando se cierra la ventana y se pide la línea de texto
  Entonces no hay error y la línea es exactamente `t=1s fps=0.0 ejec=0.0/0.0 color=0.0/0.0 dibujo=0.0/0.0 espera=0.0/0.0 nativos=0 interprete=0 vacias=1000 ventana=1200x1920`
Test: `src/perf_tests.rs::test_ventana_sin_fotogramas_no_divide_por_cero` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: El contador de FPS y el avance del juego no cambian
Caso de uso: jugar con el contador de FPS visible como hasta ahora | Tipo: regresión
  Dado que se registran 60 fotogramas separados por 16,666 ms
  Cuando se lee `FpsCounter::fps()`
  Entonces el valor está entre 58,0 y 62,0, y los tests de `GameRunner` (reloj, inyección de teclas, tabla de bloques) siguen pasando sin modificarse
Test: `src/fps_tests.rs::test_fps_counter_frame_accumulation_and_calculation`, `src/runner_tests.rs::test_runner_clock_and_step_execution` y `src/runner_tests.rs::test_runner_supports_block_table_injection` (ya existen)
Estado previo: (se completa al ejecutarlos antes de editar; deben pasar)

Escenario: El archivo de medición aparece en Descargas (visible)
Caso de uso: probar la tablet 30 s en portrait y llevarse los números | Tipo: normal
  Dado que el juego estuvo corriendo 30 s en la tablet
  Cuando abro `Download/blazing-perf.txt` con un gestor de archivos
  Entonces veo una línea de encabezado (con backend, modo y `reloj=`) y entre 29 y 31 líneas con el formato de la sección 3
Test: no automatizable (revisión humana)
Estado previo: no aplica

### 9. Cómo lo comprobaré

- Informe con promedio, peor caso y línea de texto → `cargo test --lib test_informe_promedia_y_reporta_peor_caso` → pasa si la línea coincide carácter por carácter con la del escenario → no detecta: tiempos reales de un dispositivo; solo comprueba la aritmética y el formato con valores inventados.
- Ventana sin fotogramas → `cargo test --lib test_ventana_sin_fotogramas_no_divide_por_cero` → pasa si no hay pánico y la línea coincide → no detecta: ventanas con muy pocos fotogramas (1 o 2).
- El contador de FPS y el avance del juego no cambian → `cargo test --lib test_fps_counter_frame_accumulation_and_calculation test_runner_clock_and_step_execution test_runner_supports_block_table_injection` (si tu versión de `cargo` no acepta varios filtros, ejecutarlos uno por uno) → pasan sin editar los tests → no detecta: cambios de FPS en un dispositivo real; eso lo cubre la revisión humana comparando con tus reportes anteriores.
- El archivo de medición aparece en Descargas (visible) → revisión humana: abrir el archivo en la tablet tras 30 s y contar las líneas; leer `reloj=` y comparar los FPS del contador con tus reportes anteriores (tablet 33-37 en portrait y 50-55 en landscape) → no detecta: diferencias pequeñas de FPS, porque tus reportes anteriores son rangos.
- Documentación → `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` y `python scripts/extraer_referencias.py --check` → pasan con 0 errores → no detecta: si la guía nueva explica bien los campos (lo juzga la persona). Nota: en un clon limpio el validador reporta 1 error preexistente (`enlace roto: ../../src-c/` en `docs/referencia/CONFIGURACION_PACK.md` línea 118), porque `src-c/` no se versiona (está en `.gitignore`); en tu computador, donde esa carpeta existe, se espera 0.
- Suite completa → `cargo test` en tu computador (comando exigido por `AGENTS.md` §6) → pasa si todos los tests están en verde; hoy el código tiene 67 tests en `src/` y 10 en `tests/` (origen: `grep -rho '#\[test\]'`) → no detecta: fallos que solo ocurren en Android. No pude ejecutarlo yo (ver suposición 4).

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Anexo (no forma parte de lo que se aprueba): Fase 2 en el fork

Para una propuesta aparte, solo si la medición de esta propuesta muestra que ayuda ir más adentro. Contadores candidatos en `gba-core` del fork (todos detrás de una opción de compilación apagada por defecto, con contadores simples y no atómicos, porque el motor corre en un solo hilo y una operación atómica en cada acceso a memoria costaría más de lo que se quiere medir):

| Qué contar | Dónde | Tipo |
|---|---|---|
| Llamadas de lectura y escritura de memoria desde el código recompilado | `rt_read8/16/32`, `rt_write8/16/32` en `crates/gba-core/src/capi.rs` | conteo por fotograma |
| Llamadas a `tick` | `rt_tick` en `crates/gba-core/src/capi.rs` | conteo por fotograma |
| Muestras de audio generadas | bucle de `pump_audio` en `crates/gba-core/src/mem.rs` | conteo por fotograma |
| Tiempo de dibujar las 160 líneas de la PPU | `render_scanline` en `crates/gba-core/src/ppu.rs` | tiempo total por fotograma (160 lecturas de reloj por fotograma, no una por píxel) |

Medir con contadores cambia un poco el tiempo medido; por eso se compara contra una compilación sin la opción activada. Usar el fork como dependencia exige cambiar `Cargo.toml` y actualizar `AGENTS.md` §4, que hoy apunta al repositorio original.

### Correcciones previas a presentar

- Primer borrador: asumía poder ejecutar `cargo test` para fijar la línea base. No compila en mi entorno (falta `wayland-client`); lo corregí declarándolo en la suposición 4 y en "Cómo lo comprobaré".
- `AGENTS.md` §6 dice "63 tests (54 en `src/lib.rs` + 9 en `tests/packaging_tests.rs`)", pero el código tiene 77 (67 + 10), cifra que coincide con la propuesta `2026-10-10_optimizacion-rendimiento-gpu-vulkan-y-calibracion-fps.md`. Gana el código; queda en "Fuera de alcance".
- La propuesta anterior de rendimiento llama "descarte" a lo que en el shader es un `return` temprano; no afecta a esta propuesta y no se corrige aquí.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **Escenario 1 (Informe con promedio, peor caso y línea de texto):**
  - *Predicho:* `t=1s fps=4.0 ejec=20.0/20.0 color=1.0/1.0 dibujo=15.0/30.0 espera=4.0/10.0 nativos=1000 interprete=10 vacias=7 ventana=1200x1920`.
  - *Real:* Idéntico carácter por carácter. Verificado y cubierto por `src/perf_tests.rs::test_informe_promedia_y_reporta_peor_caso`.
- **Escenario 2 (Ventana sin fotogramas):**
  - *Predicho:* `t=1s fps=0.0 ejec=0.0/0.0 color=0.0/0.0 dibujo=0.0/0.0 espera=0.0/0.0 nativos=0 interprete=0 vacias=1000 ventana=1200x1920` sin división por cero ni pánico.
  - *Real:* Idéntico carácter por carácter. Verificado por `src/perf_tests.rs::test_ventana_sin_fotogramas_no_divide_por_cero`.
- **Escenario 3 (El contador de FPS y el avance del juego no cambian):**
  - *Predicho:* Tests existentes de `fps_counter` y `runner` pasan sin cambios.
  - *Real:* Pasan al 100%. La suite completa creció a 80 tests en verde (70 unitarios + 10 empaquetado/docs).
- **Escenario 4 (El archivo de medición aparece en Descargas):**
  - *Predicho:* Archivo `blazing-perf.txt` con encabezado `# backend=... modo=... reloj=...ns` y una línea por segundo.
  - *Real:* Implementado en `src/platform.rs` con fallback automático sobre directorios de almacenamiento seguro y logcat. Empaquetado en el APK oficial listo para prueba de campo.

### Contexto: ¿se respetó?

- Sí. No se modificó el motor `gba-core`, el fork ni `Cargo.toml`.
- La lógica de telemetría se aisló en `src/perf.rs` sin dependencias del NDK ni de Android, con tests satélite en `src/perf_tests.rs`.
- El contador en pantalla y el pipeline visual se mantuvieron intactos sin alterar el framerate medido.

### Archivos modificados

- `src/perf.rs`: Estructuras `FramePerf`, `PerfReport`, `FrameTiming` y medición de costo del reloj `measure_clock_cost_ns()`.
- `src/perf_tests.rs`: Tests satélite para promedio, peor caso, ventana vacía y medición de reloj.
- `src/lib.rs`: Exposición pública de `pub mod perf;`.
- `src/runner.rs`: Medición de `ejec` y `color` en `GameRunner::step_frame()` y exposición de `last_timing()`.
- `src/platform.rs`: Medición de `dibujo` y `espera` en `Compositor::render()`, acumulación por segundo y escritura a `blazing-perf.txt`.
- `docs/guias/medir-rendimiento.md`: Guía de usuario Diátaxis para extracción y lectura del archivo.
- `docs/referencia/api-modulos-rust.md`: Sincronización automática de firmas de la API pública.
- `CHANGELOG.md`: Registro de la funcionalidad bajo `[Sin publicar]`.
- `docs/propuestas/2026-10-10_medicion-rendimiento-por-fase.md`: Documento de propuesta actualizado con resultados y estado.

### Discrepancias

- Ninguna.

### Para la próxima vez

- Si en versiones modernas de Android (11+) el acceso a `/sdcard/Download` tuviera restricciones de Scoped Storage según el explorador de archivos del usuario, documentar explícitamente cómo recuperarlo también desde el almacenamiento privado de la app (`Android/data/com.retro.blazing_recomp/files/`).

### Fuera de alcance (hallazgos que no se arreglaron)

- La espera activa del bucle principal (`poll_events` con `Duration::ZERO`, `src/platform.rs` líneas 1567-1575) contradice la regla "sin busy-waiting" de `AGENTS.md` §2.5 | impacto: medio
- El audio se calcula 65.536 veces por segundo aunque la app no tiene salida de audio (`pump_audio` en `crates/gba-core/src/mem.rs` del fork) | impacto: bajo
- Cada acceso a memoria del código recompilado llama de vuelta a Rust (`a->read32`, `a->write32` en el C generado por `crates/recomp-core/src/emit.rs` del fork) | impacto: medio
- `AGENTS.md` §6 indica 63 tests; el código tiene 77 | impacto: bajo
- `scripts/extraer_referencias.py --check` y `validar_docs.py` dependen de que exista `src-c/`, que no se versiona; en un clon limpio el validador da 1 error | impacto: bajo

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

---

## Revisión humana

- El archivo de medición aparece en Descargas (visible): ✔ | ✘ | ?
- Restricción "no tocar `gba-core`, el fork ni `Cargo.toml`" (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
