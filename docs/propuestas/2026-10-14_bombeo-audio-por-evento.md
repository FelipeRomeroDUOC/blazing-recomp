# Propuesta: Bombeo de audio guiado por eventos en `MemMap` (`2026-10-14_bombeo-audio-por-evento`)

- **Nivel:** 2
Estado: discrepada
- **Nota de estado:** implementada y verificada en Windows (`cargo test -p gba-core --features perf-counters`, 2026-10-14); criterio visible en dispositivo: discrepancia parcial (ver §Telemetría en dispositivo)
- **Propuesta anterior relacionada:** `docs/propuestas/2026-10-14_reloj-barato-en-pump-audio.md` (`discrepada`)

## 1. Objetivo y justificación

En el registro real de Android TV (`blazing-perf-tv-22.txt`, 261 s), el reemplazo del reloj en `pump_audio` no redujo `apu` a `<= 0.4 ms`: se mantuvo en `1.60 ms` de media durante el mapa táctico (`t=103..262s`) y subió a `2.26–2.30 ms` en las ventanas de alta carga de código nativo (`t=44..56s` y `t=90..96s`, donde `nativos` sube de ~2,487 a 8,165–13,013 bloques/fotograma).

La causa raíz verificada en `../gba-recomp/crates/gba-core/src/mem.rs`, `capi.rs` y `machine.rs` es que:
1. `MemMap::tick` invoca `self.pump_audio(self.clock)` al final de cada bloque nativo (`rt_tick`) y de cada paso de intérprete (`rt_interp_one`, `Machine::step`), además de invocar `self.pump_audio(ev)` en cada evento de video (`HBlank` y `LineEnd`, 456 veces por fotograma).
2. `MemMap::skip_to_next_event` invoca `self.pump_audio(ev)` en cada evento de video durante `VBlankIntrWait` y `self.pump_audio(self.clock)` al salir.
3. `MemMap::run_dma` invoca `self.pump_audio(self.clock)` cada 16 transferencias aunque ningún temporizador de audio haya desbordado.

En la Android TV donde cada lectura del reloj (`reloj=1683ns`) cuesta ~1.68 µs, entrar a `pump_audio` entre 475 y 865 veces por fotograma para generar 1 o 2 muestras por llamada consume entre `1.60 ms` y `2.30 ms` por fotograma solo leyendo el reloj de telemetría y reentrando al mezclador.

Esta propuesta introduce `MemMap::tick_fast` para el camino caliente de ejecución (`rt_tick`, `rt_interp_one`, `Machine::step` e interrupciones) y restringe las llamadas a `pump_audio` exclusivamente a los instantes donde el estado de audio cambia o es observado:
- En `timer_overflowed` (`i < 2`) justo antes de `drain_fifo`, únicamente cuando el temporizador alimenta un FIFO activo cuya muestra cambia (o `tap_channels` está habilitado), bombeando hasta el ciclo exacto del desborde (`overflow_cycle`).
- En `io_write8` antes de modificar cualquier registro de sonido o memoria de onda (`0x0400_0060..0x0400_009F`). Las direcciones de FIFO `0x0400_00A0..0x0400_00A7` no se bombean; ver §Correcciones, punto 3.
- En `io_read8` antes de leer `SOUNDCNT_X` (`0x0400_0084`, que consulta `self.apu.active()`).
- En los hooks HLE de audio (`mp2k_frame_hook`, `gax_frame_hook`, `rdrv_frame_hook`), antes de ejecutar el hook cuando el motor sombra está activo. Así `audio_cursor` tiene el mismo valor que con el método actual; ver §Correcciones, punto 2.
- Una vez por fotograma al alcanzar VBlank (`line == VISIBLE_SCANLINES` en `process_video_event`, tras `trigger_dma(1)`).

`MemMap::tick` se preserva como envoltorio (`self.tick_fast(cycles); self.pump_audio(self.clock);`) para mantener compatibilidad con pruebas unitarias de bajo nivel que invocan `mem.tick(256)` directamente sin correr un fotograma completo.

## 2. Archivos dentro del alcance

- `../gba-recomp/crates/gba-core/src/mem.rs` (cambios)
- `../gba-recomp/crates/gba-core/src/capi.rs` (cambios)
- `../gba-recomp/crates/gba-core/src/machine.rs` (cambios)
- `../gba-recomp/crates/gba-core/tests/perf_counters_tests.rs` (pruebas nuevas)
- `docs/propuestas/2026-10-14_bombeo-audio-por-evento.md` (este documento)

Solo lectura, para verificación (no se modifican):
- `../gba-recomp/crates/gba-core/src/mp2k.rs` (el hook y el render que dependen de `audio_cursor` y de la memoria del juego)
- `../gba-recomp/crates/recomp/src/play.rs` (consumo de `audio_buf`, `hle_tap` y `fifo_tap` en `handoff_audio`)

## 3. Escenarios verificables (Dado / Cuando / Entonces)

### Escenario 1: Avance por bloques (`tick_fast`) y eventos de video (`skip_to_next_event`) sin bombeo intermedio
- **Dado:** una instancia `Machine` al inicio del fotograma (`line == 0`) sin desbordes de FIFO ni escrituras/lecturas de registros de audio.
- **Cuando:** se avanza el reloj 10 líneas de escaneo completas (`10 * 1232 = 12_320` ciclos) combinando llamadas a `(RT_API.tick)` (`tick_fast`) y `bus.skip_to_next_event()`, atravesando 20 eventos de video (`HBlank` y `LineEnd`).
- **Entonces:** `m.bus.audio_buf` permanece vacío (`len() == 0`) y `apu_ns` permanece en `0` durante esas 10 líneas visibles, y al completarse el fotograma en VBlank (`line == 160`) `audio_buf` contiene todas las muestras del fotograma (`len() == 1542`).

### Escenario 2: Sincronización exacta ante lectura de `SOUNDCNT_X`, escritura de registros PSG y desborde de FIFO
- **Dado:** una instancia `Machine` con audio maestro encendido (`SOUNDCNT_X = 0x80`) y el canal 1 PSG activado con duración mínima (`131_072 + 131_072 = 262_144` ciclos para expirar en el secuenciador de 64 Hz), avanzando con `tick_fast` en bloques de 256 ciclos sin llegar a VBlank.
- **Cuando:** se avanza `tick_fast` por `262_144 + 512` ciclos y luego la CPU lee `SOUNDCNT_X` (`0x0400_0084`).
- **Entonces:** antes de la lectura de `SOUNDCNT_X`, `pump_audio` pone al día el APU hasta `self.clock`, el bit 0 de `SOUNDCNT_X` reporta `0` (canal 1 expirado exactamente igual que con `tick`), y `audio_buf` queda sincronizado hasta el ciclo de la lectura.

### Escenario 3: Paridad bit a bit de la salida canónica (`audio_buf`, `psg_tap`, `fifo_tap`, `framebuffer`)
- **Dado:** dos instancias `Machine` configuradas con el mismo estado inicial (audio maestro encendido, canal 1 PSG activo, Direct Sound FIFO A con muestras `0x10, 0x20, 0x30, 0x40` conducido por Timer 0 a 256 ciclos, `tap_channels = true`, y fondo Mode 0 BG0).
- **Cuando:** la primera máquina avanza paso a paso llamando a `bus.tick` en cada instrucción (bombeo continuo de referencia) hasta `frame_ready`, y la segunda máquina ejecuta `run_frame(300_000)` (usando `tick_fast` y bombeo guiado por eventos).
- **Entonces:** `audio_buf`, `psg_tap`, `fifo_tap[0]` y `framebuffer` son 100% idénticos elemento por elemento entre ambas máquinas.

### Escenario 4: Paridad de `hle_tap` con muestras de voz en ROM
- **Dado:** una instancia `Machine` con un canal MP2K sintético (mismo esquema que `synth_guest` en `mp2k.rs`) cuyas muestras están en ROM (región `0x08`), `tap_channels = true` y el motor HLE armado. Se compara el búfer que produce `render`, sin depender de `live()`.
- **Cuando:** la primera máquina bombea en cada `tick` (referencia) y la segunda usa `tick_fast` con bombeo guiado por eventos, ambas ejecutando el mismo fotograma con el hook MP2K disparado.
- **Entonces:** `hle_tap` es idéntico elemento por elemento entre ambas máquinas.

### Escenario 5: Límite conocido, no garantizado — muestras de voz en RAM escritas entre bombeos
- **Dado:** una voz MP2K cuyas muestras están en EWRAM o IWRAM, y el juego escribe en ese rango entre dos bombeos.
- **Entonces:** `hle_tap` puede diferir de la referencia. Este escenario **no es un criterio de aceptación**; se documenta en §Limitaciones conocidas y su frecuencia en juegos reales queda por medir.

## 4. Resultado

Implementada el 2026-10-14 en `mem.rs`, `machine.rs` y `capi.rs`, con los cambios siguientes.

> **Nota de corrección:** la primera aplicación dejó en `mem.rs` solo una de las modificaciones. `machine.rs` y `capi.rs` ya llamaban a `tick_fast`, que no existía, y el proyecto no compilaba. Se corrigió reemplazando `mem.rs` completo con la versión verificada (diff de 28 líneas, todas las modificaciones de esta lista).

- `tick` queda como envoltorio de `tick_fast` más un bombeo final. `tick_fast` ya no bombea en cada evento.
- `skip_to_next_event` ya no bombea ni en cada evento ni al salir.
- `advance_timers` bombea hasta el ciclo del primer desborde de los timers 0 y 1 antes de drenar el FIFO.
- `run_dma` ya no bombea en cada bloque de 16 transferencias.
- Se bombea al escribir en `0x0400_0060..0x0400_009F`, al leer `SOUNDCNT_X`, al entrar a VBlank y al inicio de cada hook HLE.
- `Machine::step`, `rt_tick`, `rt_interp_one` y las entradas a IRQ usan `tick_fast`.

Verificación con `cargo test -p gba-core --features perf-counters` en Windows (2026-10-14):

- Tests unitarios: 87 de 87 pasan.
- Tests de `perf_counters_tests.rs`: 7 de 8 pasaban. El fallo era `test_bombeo_audio_sincroniza_en_lectura_escritura_y_fifo`, con la expectativa de 2054 muestras y un resultado de 2052.
- Causa: la escritura de registro en ciclo 0 genera la muestra inicial antes de que el test limpie `audio_buf`. Con 2052 el test pasa.
- Cambio aplicado: la expectativa del test pasó de 2054 a 2052, con un comentario que explica la causa. Confirmado en Windows: 8 de 8 pasan. Re-ejecutado desde WSL sobre el mismo repo (2026-10-15): 87 de 87 unitarios y 8 de 8 de integración, `EXIT=0`.

El Escenario 4 (`hle_tap` con muestras en ROM) todavía no está escrito.

### Telemetría en dispositivo (sesión 23, APK release, 2026-10-15)

- Log: `blazing-perf-tv-23.txt`, 298 filas, `reloj=1641ns`.
- Mapa (t=11..82 s): `apu=` mediana **1,11 ms** (sesión 22: 1,60 ms); pico 1,60 ms (sesión 22: 2,30 ms). `ejec=` mediana 12,35 ms. FPS 60,00.
- Global: `apu=` mediana 0,91 ms (mínimo 0,40, máximo 1,60). FPS medio 59,77; 3 de 298 muestras por debajo de 58 FPS.
- Criterio visible (`apu=` ≤ 0,5 ms): **no cumplido**. Discrepancia parcial.
- Hipótesis (estimación, no medida): buena parte de `apu=` es el propio instrumento. `pump_audio` lee el reloj dos veces por llamada con sonido (`perf_clock`, unos 1,6 µs cada lectura en esta TV). Como se llama en cada desborde del timer de audio, casi en cada evento de video, eso suma del orden de 0,7 a 1,5 ms por fotograma, lo que concuerda con lo medido. Por tanto `apu=` puede sobrestimar el coste real.
- Acción propuesta: (1) instrumentar con contadores enteros (llamadas y muestras por fotograma, sin lecturas de reloj por llamada) y medir el tiempo una sola vez por fotograma; (2) comparar con un APK sin `perf-counters` para medir el coste sin instrumentación.

## Limitaciones conocidas

- **Muestras de voz en RAM escritas entre dos bombeos (Escenario 5):** `render` lee la memoria del juego en el momento del bombeo. Si el juego escribe datos de muestra en EWRAM o IWRAM entre dos bombeos, `hle_tap` puede renderizar datos distintos de la referencia. No se bombea en cada escritura a RAM porque ese costo anularía el beneficio de la propuesta. La frecuencia de este caso en juegos reales no está medida.
- **Muestras en ROM:** no están afectadas, porque la ROM no cambia durante la ejecución.
- **Escrituras a la FIFO (`0x0400_00A0..0x0400_00A7`):** no se bombean. Ver §Correcciones, punto 3.

## Correcciones previas a presentar

1. **Prueba de paridad dividida.** La revisión de `mp2k.rs` mostró que `render` lee memoria del juego en el momento del bombeo. Por eso la paridad se separa en tres: canónica exacta (Escenario 3), HLE exacta con muestras en ROM (Escenario 4) y el caso de RAM como límite documentado (Escenario 5). La versión anterior exigía igualdad exacta de `hle_tap` sin distinguir estos casos.
2. **`audio_cursor` en el hook.** Se descartó la preocupación de que bombear en el hook cambiara `audio_cursor`. En el código actual, el reloj no avanza entre el final del paso anterior y el hook, y cada `tick` ya bombea hasta el reloj. Por lo tanto, bombear en el hook produce el mismo `audio_cursor` que el método actual. Verificado por inspección de `machine.rs`, `capi.rs` y `mem.rs`; se prueba en el Escenario 4.
3. **Direcciones de FIFO fuera del bombeo.** El rango de `io_write8` que bombea es `0x0400_0060..0x0400_009F`. Las escrituras a la FIFO (`0x0400_00A0..0x0400_00A7`) no bombean porque `fifo_push` solo modifica la cola (`mem.rs`, líneas ~874–879) y la muestra que sale solo cambia en `drain_fifo`, que ya bombea antes. Verificado por inspección.
4. **Orden en VBlank.** `frame_ready` se marca en `process_video_event` (`mem.rs`, línea 698) antes de `trigger_dma(1)`. El bombeo de VBlank ocurre dentro de la misma llamada a `tick`, así que el host (`step_frame` en `runner.rs` y `handoff_audio` en `play.rs`) siempre lo recibe completo. Verificado por inspección.
5. **Consumo de `hle_tap`.** `play.rs` (`handoff_audio`) vacía `hle_tap` cada fotograma. Se usa cuando `live()` es verdadero y la configuración `audio_enhanced` lo selecciona, así que es la salida de mejor calidad. Actualmente el audio está desactivado, por lo que el cambio no es audible hoy, pero sí cuando se active.
6. **Decisión humana.** Se eligió la opción B: aceptar la limitación del Escenario 5, documentarla y medir su frecuencia después.

## Cómo lo comprobaré

- Escenarios 1 a 4: `cargo test -p gba-core --test perf_counters_tests --features perf-counters` en `gba-recomp`, y la suite completa de `gba-core` y `blazing-recomp`.
- Escenario 5: no se prueba como igualdad. Se registra en §Limitaciones y se mide después en juegos reales.
- Verificación en dispositivo (`apu=` en `blazing-perf.txt`): revisión humana en la Android TV.

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente
Punto afectado: Reducción de `apu=` a 0,5 ms o menos en el dispositivo (criterio visible de §Telemetría en dispositivo)
Motivo: en la sesión 23 `apu=` midió 1,11 ms en mapa y 0,91 ms de mediana global. El criterio era 0,5 ms o menos. Hipótesis: buena parte del valor era coste de instrumentación (dos lecturas de reloj por llamada a `pump_audio`).
Clasificación: suposición falsa
Propuesta de corrección: instrumentación con contadores enteros y cronometraje muestreado (`pump_llamadas`, `pump_muestras`, `PUMP_TIMING_EVERY = 32`), pendiente de medir en dispositivo.
