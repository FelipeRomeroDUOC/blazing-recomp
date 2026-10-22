# Propuesta: Reloj barato y ahorro de trabajo muerto en el muestreo de audio

Fecha: 2026-10-14
Nivel: 2
Estado: cerrada
Cerrada: superada por [`docs/propuestas/2026-10-14_bombeo-audio-por-evento.md`](2026-10-14_bombeo-audio-por-evento.md), que elimina las llamadas excesivas a `pump_audio` en lugar de abaratar el reloj. El reloj de TV Box (`armeabi-v7a`, `Instant::now`) se mantiene como coste secundario.

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El muestreo de audio por fotograma deja de pagar dos lecturas del reloj del sistema operativo cada 256 ciclos de consola (~1.100 veces por fotograma) y evita trabajo innecesario cuando los mezcladores de diagnóstico o los canales de sonido están apagados, reduciendo el tiempo de emulación en el dispositivo sin alterar un solo píxel ni muestra de sonido.

### 2. Contexto

- Contexto consultado: [`AGENTS.md`](../../AGENTS.md), [`docs/BACKLOG.md`](../BACKLOG.md), [`docs/explicacion/analisis_rendimiento.md`](../explicacion/analisis_rendimiento.md), [`docs/propuestas/2026-10-12_ppu-60fps-estable.md`](2026-10-12_ppu-60fps-estable.md), [`docs/propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md`](2026-10-12_telemetria-sin-coste-en-el-apk.md), [`docs/propuestas/2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md`](2026-10-13_el-reloj-de-la-ppu-en-el-dispositivo.md); del port: [`Cargo.toml`](../../Cargo.toml), [`src/runner.rs`](../../src/runner.rs), [`src/perf.rs`](../../src/perf.rs); del motor: `../gba-recomp/crates/gba-core/src/mem.rs`, `../gba-recomp/crates/gba-core/src/apu.rs`, `../gba-recomp/crates/gba-core/src/perf_clock.rs`, `../gba-recomp/crates/gba-core/src/ppu.rs` y `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs`.
- Intención: eliminar el único punto del camino caliente que seguía llamando a `std::time::Instant::now()` cientos de veces por fotograma (~1,5–2,0 µs por lectura en el Android TV según la cabecera `reloj=`) y suprimir operaciones redundantes por muestra a 65.536 Hz.
- Restricciones y decisiones previas (máx. 3):
  - **El renderizado y las muestras de audio son idénticos bit a bit** (origen: [`docs/propuestas/2026-10-12_ppu-60fps-estable.md`](2026-10-12_ppu-60fps-estable.md) y `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs`) → implica: cualquier atajo en el muestreo debe producir exactamente los mismos valores en `audio_buf`, `psg_tap` y `framebuffer` → protegida por: *El renderizado y las muestras de sonido no cambian (regresión)*.
  - **Ningún campo de la telemetría cambia de formato ni de significado** (origen: [`docs/propuestas/2026-10-12_telemetria-sin-coste-en-el-apk.md`](2026-10-12_telemetria-sin-coste-en-el-apk.md)) → implica: `apu_ns` sigue midiendo nanosegundos acumulados en `pump_audio` y se reinicia en `take_perf_counters()` → protegida por: *El contador de tiempo de audio mide con el reloj ligero y se reinicia*.
  - **Portabilidad y ausencia de rutas absolutas locales** (origen: [`AGENTS.md`](../../AGENTS.md) §6) → implica: no escribir rutas del usuario ni del sistema en ningún archivo versionado → protegida por: no testeable (revisión humana y auditoría pre-commit).
- Alternativas descartadas:
  - **Dejar de llamar a `pump_audio(self.clock)` al final de `MemMap::tick`** → descartada tras leer `../gba-recomp/crates/gba-core/src/mem.rs` (líneas 977 y 1039): llamadas cortas como `mem.tick(256)` no alcanzan `HBLANK_FLAG_CYCLE` (1.006 ciclos) y esperan que la muestra de los 256 ciclos ya esté en `audio_buf` e impacte en los flags activos de `SOUNDCNT_X` (`0x0400_0084`).
  - **Desactivar por completo el muestreo del APU mientras la app no tenga salida de audio** → descartada porque el código del juego lee el registro `SOUNDCNT_X` (`0x0400_0084`, `mem.rs:647`), cuyos bits 0–3 dependen de que la duración (`length`) de los canales PSG avance en cada muestra.
- Ante imprevistos, prioriza: conservar la paridad exacta de `audio_buf`, `SOUNDCNT_X` y `framebuffer` antes que rascar nanosegundos adicionales.

### 3. Ejemplos

- **Caso normal** (un fotograma completo de 280.896 ciclos con `perf-counters` activo en ARM64):
  antes: `pump_audio` entra ~600–1.097 veces por fotograma y en cada entrada invoca `std::time::Instant::now()` y `t_start.elapsed()` (~1.200–2.194 lecturas de reloj del sistema a ~1,5–2,0 µs en el Android TV ≈ **1,5–2,2 ms** de sobrecarga por fotograma), además de extraer y restaurar `mp2k`, `gax` y `rdrv` aunque `tap_channels` sea `false`;
  ahora: `pump_audio` cronometra con `crate::perf_clock::ticks()` y `crate::perf_clock::ns_since(t_start)` (lectura directa del registro `cntvct_el0` en ARM64, igual que `ppu.rs`), y solo extrae `mp2k`, `gax` y `rdrv` cuando `self.tap_channels` es `true`.
- **Caso límite** (muestreo a 65.536 Hz cuando los 4 canales PSG están apagados, `apu.active() == 0`):
  antes: `Apu::sample` (1.097 veces por fotograma) invoca `sq[0].sample`, `sq[1].sample`, recorre los 4 canales para sumar volúmenes `SOUNDCNT_L` y evalúa el divisor `SOUNDCNT_H` para acabar devolviendo `(0, 0)`;
  ahora: `Apu::sample` actualiza únicamente el acumulador de secuenciador de 64 Hz (`frame_acc`) y retorna `(0, 0)` de inmediato cuando ningún canal está activo.

### 4. Pasos

1. En `../gba-recomp/crates/gba-core/src/mem.rs` (`MemMap::pump_audio`, líneas 361–455), reemplazar `std::time::Instant::now()` y `t_start.elapsed().as_nanos() as u64` por `crate::perf_clock::ticks()` y `crate::perf_clock::ns_since(t_start)`.
2. En la misma función `MemMap::pump_audio` (`../gba-recomp/crates/gba-core/src/mem.rs`, líneas 366–368 y 447–449), extraer y restaurar `self.mp2k`, `self.gax` y `self.rdrv` únicamente cuando `self.tap_channels` sea `true`.
3. En `../gba-recomp/crates/gba-core/src/apu.rs` (`Apu::sample`, líneas 212–220), añadir salida temprana cuando `self.active() == 0`: avanzar `self.frame_acc` con desplazamiento/máscara sobre `262144` (`1 << 18`) y retornar `(0, 0)` sin evaluar los canales ni la matriz de volumen.
4. Añadir los tests de los escenarios en `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs` y verificar la suite completa en motor y port.

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "apu_ns" src/ ../gba-recomp/crates/`, `grep -rn "pump_audio" ../gba-recomp/crates/`, `grep -rn "tap_channels" ../gba-recomp/crates/` y `grep -rn "Instant::now" ../gba-recomp/crates/gba-core/src/`.

- `../gba-recomp/crates/gba-core/src/mem.rs` — `pump_audio` es el único sitio restante en `gba-core/src/` (fuera de `perf_clock.rs` y `rtc.rs`) que aún llamaba a `std::time::Instant::now()`.
- `../gba-recomp/crates/gba-core/src/apu.rs` — `Apu::sample` es invocada exclusivamente desde `MemMap::pump_audio` (`mem.rs:372`).
- [`src/runner.rs`](../../src/runner.rs) (línea 143) — convierte `hw.apu_ns` a milisegundos (`apu_ms`); no requiere cambios porque la unidad sigue siendo nanosegundos.
- `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs` — comprueba que `c.apu_ns > 0` tras un fotograma y que `take_perf_counters()` lo deja en 0.

### 6. Suposiciones

- **Cuando `self.active() == 0`, ningún canal PSG modifica estado interno en `Apu::sample` más allá de `self.frame_acc`** → prototipar: no (comprobado leyendo `../gba-recomp/crates/gba-core/src/apu.rs` líneas 65–67, 223 y 257) → si es falso: algún test de `apu.rs` o `mem.rs` fallaría y se retiraría el atajo de `Apu::sample`.
  → resultado del prototipo: no aplica (verificado por inspección exhaustiva de las 4 ramas y validado con tests) | no detecta: canales HLE externos (que van por `mp2k`/`gax`/`rdrv` en `mem.rs`, fuera de `Apu::sample`).
- **`perf_clock::ns_since` devuelve `> 0` tras las ~1.097 muestras de un fotograma en todos los targets** → prototipar: no (ya verificado para `bg_ns` y `compose_ns` en `perf_counters_tests.rs`) → si es falso: `test_perf_counters_accumulate_and_reset` fallaría en `assert!(c.apu_ns > 0)`.

### 7. Alcance

- **Incluye:**
  - Sustituir `std::time::Instant::now()` por `crate::perf_clock::ticks()` / `ns_since()` en `MemMap::pump_audio` (`../gba-recomp/crates/gba-core/src/mem.rs`).
  - Condicionar la extracción temporal (`.take()`) de `mp2k`, `gax` y `rdrv` en `pump_audio` a `if self.tap_channels`.
  - Retorno rápido en `Apu::sample` (`../gba-recomp/crates/gba-core/src/apu.rs`) cuando los cuatro canales PSG están inactivos (`self.active() == 0`).
  - Pruebas unitarias y de regresión en `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs`.
- **No incluye:**
  - Modificar la tabla `BlockTable` en `src/recomp.rs`, la PPU en `ppu.rs` ni el pipeline de `runner.rs` / `pipeline.rs`.
  - Implementar la salida de audio en Android (AAudio/Oboe).
  - Cambiar el formato o las columnas de `blazing-perf.txt`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: El contador de tiempo de audio mide con el reloj ligero y se reinicia**
Caso de uso: telemetría por fase durante la emulación | Tipo: normal
  Dado una instancia de `Machine` con la feature `perf-counters` activa y sin código en `mem.rs` que invoque `std::time::Instant::now`
  Cuando se ejecuta un fotograma completo (`run_frame(300_000)`) y se extraen los contadores con `take_perf_counters()`
  Entonces `apu_ns` es mayor que `0`, `mem.rs` no contiene ninguna llamada a `Instant::now`, y una segunda llamada inmediata a `take_perf_counters()` devuelve `apu_ns == 0`
Test: `crates/gba-core/tests/perf_counters_tests.rs::test_apu_usa_perf_clock_y_acumula_tiempo`
Estado previo: falla (antes de editar `mem.rs:362` contenía `std::time::Instant::now()`)

**Escenario: El muestreo en silencio con canales apagados conserva el reloj de 64 Hz y produce cero**
Caso de uso: el juego corre con canales PSG inactivos o antes de activar música | Tipo: límite
  Dado un `MemMap` con `SOUNDCNT_X = 0x80` (audio maestro encendido) pero los 4 canales PSG apagados (`apu.active() == 0`) y luego un canal con duración (`length`) activado tras 262.144 ciclos
  Cuando se avanza el reloj en pasos de 256 ciclos durante un ciclo completo del secuenciador de 64 Hz (262.144 ciclos) y después se activa el canal 1 con duración mínima
  Entonces todas las muestras en silencio en `audio_buf` son `0` y el acumulador `frame_acc` se conserva alineado de modo que el canal activado después expira exactamente tras el siguiente período de 262.144 ciclos
Test: `crates/gba-core/tests/perf_counters_tests.rs::test_apu_silencio_conserva_frame_acc_y_expiracion`
Estado previo: pasa (comportamiento observable que el atajo de caso límite preserva intacto)

**Escenario: El renderizado y las muestras de sonido no cambian (regresión)**
Caso de uso: ejecución de sonido PSG, FIFO DMA y vídeo en una sesión real | Tipo: regresión
  Dadas dos instancias de `Machine` donde se configuran canales PSG activos, FIFO A con temporizador y `tap_channels = true`
  Cuando se ejecuta un fotograma completo (`run_frame(300_000)`)
  Entonces `audio_buf`, `psg_tap`, `fifo_tap` y `framebuffer` coinciden elemento a elemento con los valores de referencia y todos los tests existentes de `mem::tests` y `apu::tests` pasan en verde
Test: `crates/gba-core/tests/perf_counters_tests.rs::test_regresion_paridad_audio_y_video`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- **El contador de tiempo de audio mide con el reloj ligero y se reinicia** → comando: `cargo.exe test -p gba-core --test perf_counters_tests --features perf-counters`
  → pasa si: el test termina en `ok` (0 apariciones de `Instant::now` en `crates/gba-core/src/mem.rs` y `apu_ns > 0`)
  → no detecta: la reducción exacta en milisegundos dentro del televisor Android TV (requiere sesión en dispositivo).
- **El muestreo en silencio con canales apagados conserva el reloj de 64 Hz y produce cero** → comando: `cargo.exe test -p gba-core --test perf_counters_tests --features perf-counters`
  → pasa si: el test termina en `ok`
  → no detecta: fallos en canales con modulación de barrido activo (cubiertos por `apu::tests`).
- **El renderizado y las muestras de sonido no cambian (regresión)** → comando: `cargo.exe test -p gba-core --lib --features perf-counters` y `cargo.exe test` en la raíz del port
  → pasa si: todos los tests unitarios y de integración de `gba-core` y `blazing-recomp` pasan en verde
  → no detecta: el tiempo real de `apu=` en el archivo `blazing-perf.txt` generado en el televisor.
- **Reducción de `apu=` y `ejec=` en el dispositivo (visible)** → revisión humana: empaquetar el APK e inspeccionar `blazing-perf.txt` en Android TV comprobando que `apu=` baja de ~1,5 ms a ≤0,5 ms
  → no detecta: regresiones en escenas no visitadas durante la partida.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación, las suites de pruebas de `gba-core` y `blazing-recomp` pasan al 100 % en verde y no existen rutas locales absolutas en los archivos tocados.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

### Correcciones previas a presentar

- **Corregido antes de presentar:** inicialmente consideré eliminar la llamada `self.pump_audio(self.clock)` al final de `MemMap::tick` (`mem.rs:348`) para bombear audio solo en `process_events()` (cada ~1.006 ciclos). Al revisar los tests en `../gba-recomp/crates/gba-core/src/mem.rs` (líneas 977 y 1039, `wave_plays_hidden_bank_after_flip`), comprobé que el contrato del bus exige que `mem.tick(256)` produzca inmediatamente la muestra en `audio_buf` aunque no se haya alcanzado un evento de línea o timer. Por tanto, mantengo la llamada en `tick` y ataco el coste real por muestra: el `std::time::Instant::now()` olvidado en `pump_audio`, los `.take()` incondicionales de `mp2k`/`gax`/`rdrv` y la evaluación en silencio de `Apu::sample`.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno: el plan se ejecutó tal cual fue aprobado.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- **El contador de tiempo de audio mide con el reloj ligero y se reinicia**: coincide.
  Resultado real: `test_apu_usa_perf_clock_y_acumula_tiempo` falló en rojo antes de editar y pasó en verde tras la edición (`6 passed; 0 failed; finished in 0.72s`).
  Método: el anunciado (`cargo.exe test -p gba-core --test perf_counters_tests --features perf-counters`).
- **El muestreo en silencio con canales apagados conserva el reloj de 64 Hz y produce cero**: coincide.
  Resultado real: `test_apu_silencio_conserva_frame_acc_y_expiracion` pasó en verde y mató al mutante de control `if true { return (0, 0); }`.
  Método: el anunciado.
- **El renderizado y las muestras de sonido no cambian (regresión)**: coincide.
  Resultado real: `test_regresion_paridad_audio_y_video` en verde; suite de `gba-core` en **93 verdes** y suite de `blazing-recomp` en **131 verdes**.
  Método: el anunciado.
- **Reducción de `apu=` y `ejec=` en el dispositivo (visible)**: **difiere**.
  Resultado real: En la sesión 22 (`blazing-perf-tv-22.txt`, 261 s, `reloj=1683ns`), los 160 s de mapa táctico (`t=103..262s`) sostienen **59,94 FPS de media (0/160 s por debajo de 58 FPS)** y `ejec` baja a **11,42 ms** (`ppu=5,20 ms`, `comp=1,50 ms`), pero **`apu=` sigue en 1,60 ms en mapa y sube a 2,30 ms en los picos de código nativo (`t=44..56s` y `t=90..96s`)**, no cumpliendo el listón de `≤0,5 ms`. La causa queda demostrada por los propios datos del log: en el Android TV (`reloj=1683ns`), `perf_clock::ticks()` cuesta lo mismo que `Instant::now()` (~1,68 µs), y como `tick()` y `skip_to_next_event()` siguen llamando a `pump_audio` entre 470 y 865 veces por fotograma (940 a 1.730 lecturas de reloj/frame), el propio cronómetro dentro de `pump_audio` consume `1,58–2,30 ms/frame`.
  Método: inspección del registro `blazing-perf-tv-22.txt` del APK compilado.

### Contexto: ¿se respetó?

- **El renderizado y las muestras de audio son idénticos bit a bit** → respetada (`test_regresion_paridad_audio_y_video` y 87 unitarios de `gba-core` en verde).
- **Ningún campo de la telemetría cambia de formato ni de significado** → respetada (`test_apu_usa_perf_clock_y_acumula_tiempo` y `test_perf_counters_accumulate_and_reset` en verde).
- **Portabilidad y ausencia de rutas absolutas locales** → respetada (`test_no_local_paths_in_packaging` y `test_propuestas_declaran_estado_valido` en verde).
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: en el dispositivo se reveló que el coste venía de la frecuencia de entrada a `pump_audio` multiplicada por el coste de lectura de reloj en TV Box (`1.683 ns`).
- ¿Reapareció alguna alternativa descartada? sí, evitar llamar a `pump_audio` en cada `rt_tick` y evento de línea vacíos es la palanca necesaria para eliminar las ~1.500 lecturas de reloj por fotograma.

### Archivos modificados

- `../gba-recomp/crates/gba-core/src/mem.rs`
- `../gba-recomp/crates/gba-core/src/apu.rs`
- `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs`
- `docs/propuestas/2026-10-14_reloj-barato-en-pump-audio.md`
¿Coincide con lo anunciado? sí.

### Discrepancias

- `apu=` en el Android TV midió `1,60–2,30 ms` en lugar de `≤0,5 ms` porque cada lectura de reloj en el TV Box cuesta ~1,68 µs y `pump_audio` sigue invocándose 470–865 veces por fotograma desde `rt_tick` y `skip_to_next_event` → suposición falsa.

### Para la próxima vez

En dispositivos donde la lectura de reloj cuesta ~1,68 µs, sustituir el reloj no basta si la función instrumentada se invoca cientos de veces por fotograma: hay que agrupar el trabajo por evento real.

### Fuera de alcance (hallazgos que no se arreglaron)

- `BlockTable::from_blocks` en `src/recomp.rs` aloja un vector `rom` indexado por byte de hasta ~128 MB en 64 bits con <0,2 % de densidad, penalizando la caché L2/L3 en cada despacho | impacto: medio
- `render_scanline` y `render_sprites` en `../gba-recomp/crates/gba-core/src/ppu.rs` evalúan coordenadas y lecturas de VRAM de sprites antes de descartar píxeles ya ocluidos por un sprite de mayor prioridad | impacto: medio
- `EmulationWorker` en `src/pipeline.rs` realiza una copia extra de 153,6 KB por fotograma desde `runner.frame_rgba()` hacia el búfer del pool en lugar de componer directamente en el búfer destino | impacto: bajo
- `record_hotspot` en `src/recomp.rs` adquiere el `Mutex` global `HOTSPOTS` en cada instrucción ejecutada por el intérprete de respaldo | impacto: bajo

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente
Punto afectado: Reducción de `apu=` y `ejec=` en el dispositivo (`§9`, criterio visible: `apu=` de ~1,5 ms a ≤0,5 ms)
Motivo: En la sesión 22 (`blazing-perf-tv-22.txt`), `apu=` midió `1,60 ms` en mapa y `2,30 ms` en los picos de bloques nativos (`t=44..56s` y `t=90..96s`). En este Android TV (`reloj=1683ns`), cada lectura de reloj cuesta `1,683 µs`; al seguir entrando a `pump_audio` desde cada `rt_tick` (cada 256 ciclos) y cada evento de línea en `skip_to_next_event` (470–865 entradas/frame × 2 lecturas de reloj), se pagan `1,58–2,30 ms/frame` solo en lecturas de reloj.
Clasificación: suposición falsa
Propuesta de corrección: [`docs/propuestas/2026-10-14_bombeo-audio-por-evento.md`](2026-10-14_bombeo-audio-por-evento.md)

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona.
     Si no queda nada para revisar: "ninguno: todo se comprobó con tests
     ([cuáles])". -->

- Reducción de `apu=` y `ejec=` en `blazing-perf.txt` al probar el APK en Android TV (visible / discrepado en sesión 22): ✔ | ✘ | ?
- Restricción "Portabilidad y ausencia de rutas absolutas locales" (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
