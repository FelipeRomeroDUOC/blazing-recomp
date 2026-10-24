# Propuesta: El reloj que cronometra la PPU — sonda en el dispositivo y coste fuera de aarch64

Fecha: 2026-10-13
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El port gana una **sonda de relojes**: mide, una sola vez al arrancar, cuánto cuesta una lectura del reloj
del motor (`gba_core::perf_clock::ticks()`) y una del reloj del sistema (`Instant::now()`, que ya mide), y
publica los dos números en la telemetría. Con ese número —medido en el dispositivo, no supuesto— se decide
si el cronómetro de la PPU cuesta algo en el objetivo real y, solo si cuesta, se repara.

La fase A de la propuesta anterior dejó la pregunta al borde: el grupo del bus no cuesta nada (−0,7 %,
dentro del ruido), el grupo de cronómetros PPU+APU cuesta **+10,1 %** en el arnés de host, y el mecanismo
de ese 10,1 % es la *lectura de reloj* (~140 ns cada una, 640 por fotograma). Pero `perf_clock` **solo usa
`CNTVCT_EL0` bajo `#[cfg(target_arch = "aarch64")]`**: el arnés de host, `armeabi-v7a` y `x86_64` caen en
`Instant::now()`, que en esa familia de TV el propio port mide en 1,4-2,2 µs. ¿Cuánto cuesta `CNTVCT_EL0` en
ese Android TV? Hoy no se sabe, y es la diferencia entre 0,1 % y 5,8 % del fotograma.

### 2. Contexto

- Contexto consultado: [`2026-10-12_telemetria-sin-coste-en-el-apk.md`](2026-10-12_telemetria-sin-coste-en-el-apk.md)
  (cerrada como discrepada, con el reparto medido y el enlace a esta), `2026-10-11_desglose-ppu.md` (donde el
  coste de las 640 lecturas por fotograma quedó acotado en ~0,89 ms sobre Cortex-A53), del motor:
  `crates/gba-core/src/perf_clock.rs` (el reloj y su doc de ~30 ns genéricos), `crates/gba-core/src/lib.rs`
  (`pub mod perf_clock` solo bajo la feature), `crates/gba-core/src/ppu.rs` (los cuatro cronómetros por
  línea visible); del port: [`src/perf.rs`](../../src/perf.rs) (`FrameTiming` y el encabezado con `reloj=`),
  [`src/platform.rs`](../../src/platform.rs) (`init_perf_file` y el encabezado), `Cargo.toml` (la feature),
  [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) y
  [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md) (la tabla de campos).
- Intención: no repetir el error que acaba de costar una propuesta. El +10,4 % se atribuyó al APK sin mirar
  qué camino toma el código por arquitectura; el número pertenecía al host. Esta vez el número que decide se
  mide donde se paga.
- Restricciones y decisiones previas (máx. 3):
  - **La sonda no puede cambiar lo que mide** (origen: el `reloj=` existente, medido una vez al arrancar) →
    implica: se mide al iniciar la sesión, no por fotograma, y las líneas `t=` no cambian → protegida por:
    *La sonda no toca el fotograma*.
  - **El formato del `.txt` sigue siendo legible** (origen: diecisiete sesiones ya medidas y el analizador) →
    implica: si el número va al encabezado o a una línea `#`, el analizador tiene que seguir procesando el
    archivo sin cambios de campos → protegida por: *El analizador no se rompe*.
  - Los dos relojes se miden **en la misma sesión y el mismo dispositivo** (origen: el objetivo es comparar
    relojes, no máquinas) → implica: los dos números salen juntos y se decide con su cociente → protegida
    por: *La sonda da el número en el dispositivo*.
- Alternativas descartadas:
  - **A/B del APK completo con la feature encendida y apagada** → descartada como primer paso: exige dos
    instalaciones y una sesión humana por variante, y no separa *qué reloj* cuesta, que es justo lo que
    falta saber. La sonda responde lo mismo con una sola sesión.
  - **Dar por bueno que `CNTVCT_EL0` es barato porque es un registro arquitectónico** → descartada: el mismo
    razonamiento ya falló una vez en este hilo (los ~30 ns documentados para el camino genérico resultaron
    ser ~140 ns).
  - **Reparar primero el camino genérico (`x86_64`, `armeabi-v7a`)** → descartada como primer paso: es una
    reparación, y elegirla antes de medir es repetir el error que la fase A acaba de documentar.
- Ante imprevistos, prioriza: que la sonda no altere el fotograma por encima de que el número quede en un
  campo nuevo del encabezado.

### 3. Ejemplos

- **Caso normal** (una sesión en el Android TV de las sesiones 13-17, mapa táctico):
  antes: el encabezado informa `reloj=` (el `Instant::now()` del port, 1,4-2,2 µs) y nadie sabe cuánto
  cuesta el reloj que sí usa la PPU;
  ahora (objetivo): el encabezado informa también el reloj del motor, p. ej. `relojppu=28ns` → el
  cronómetro cuesta 640 × 28 ns ≈ **18 µs por fotograma** (~0,1 % de `ejec`) → no se repara nada y el sesgo
  del arnés de host queda documentado.
- **Caso límite** (el TV box atrapa la lectura, por virtualización):
  antes: `relojppu` desconocido;
  ahora (objetivo): si sale ~1,5 µs, el cronómetro cuesta ~0,96 ms por fotograma —el orden que
  `2026-10-11_desglose-ppu.md` acotó— y entonces sí se repara (menos lecturas por línea, o un reloj distinto
  para ese camino), con el guardián intacto.
- **Caso de instrumentación** (el analizador sobre el log nuevo):
  antes: `analizar_telemetria.py` procesa los archivos con encabezado y líneas `t=`/`#`;
  ahora (objetivo): procesa el archivo nuevo igual, con el campo extra en la cabecera o en su propia línea
  `#`, sin perder el desglose por fases.

### 4. Pasos

**Parte 1 — medir (no cambia comportamiento).**

1. Exponer la sonda en el port: al arrancar, 10.000 lecturas de `gba_core::perf_clock::ticks()` y otras
   10.000 de `Instant::now()`, con el mismo patrón que ya usa
   [`src/perf.rs`](../../src/perf.rs) para `reloj=`; publicar ambos (y su cociente) en la telemetría.
2. Que [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) siga
   procesando el archivo: aceptar el campo/línea nueva sin romper el informe.
3. Una sesión en el dispositivo (≥120 s, misma escena que la sesión 17) y leer los dos números.

**Parte 2 — decidir y, solo entonces, reparar.**

4. Si el reloj del motor sale barato (decenas de ns): **no se repara nada en el motor**. Se documenta que el
   ~8-10 % del arnés de host es un sesgo del propio arnés y se cierra la propuesta con el número medido.
5. Si sale caro (~µs): reparar con el camino que elija la medición —(a) menos lecturas de reloj por línea
   visible, manteniendo el significado de `bg_ns`/`sprite_ns`/`compose_ns`, o (b) un camino de reloj barato
   también fuera de `aarch64`— y volver a medir con el mismo A/B intercalado de la fase A.
6. Sea cual sea la rama, sincronizar la tabla de campos de
   [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md) y la decisión en
   [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md).

### 5. Qué más podría verse afectado

Búsquedas hechas: `grep -rn "perf_clock" crates/gba-core/src/` (el módulo y sus cuatro usos en `ppu.rs`),
`grep -rn "perf-counters" crates/` (12 en `mem.rs`, 9 en `ppu.rs`, 1 en `machine.rs`, 2 en `lib.rs`, 2 en
`recomp/src/main.rs`), `grep -rn "reloj" src/ scripts/ docs/` y `grep -rn "init_perf_file" src/`.

- `crates/gba-core/src/perf_clock.rs` — el camino genérico (`Instant::now()` + `OnceLock` + conversión) que
  esta propuesta pone en duda; es lectura, salvo que la rama 5(b) obligue a tocarlo.
- `crates/gba-core/src/lib.rs` — `pub mod perf_clock` y `pub use mem::GbaPerfCounters` viven bajo la feature
  `perf-counters`; la sonda del port depende de esa visibilidad.
- [`src/perf.rs`](../../src/perf.rs) — donde vive la medición de `reloj=` (10.000 lecturas) y las columnas.
- [`src/platform.rs`](../../src/platform.rs) — `init_perf_file` y el encabezado que hay que ampliar.
- [`scripts/diagnostico/analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) — el
  parser del archivo; cualquier campo nuevo del encabezado puede romperlo (se comprueba con un test).
- [`tests/packaging_tests.rs`](../../tests/packaging_tests.rs) — asume conteos del APK; la sonda no cambia
  assets ni ABIs, pero el APK debe seguir empaquetándose.
- Sin más referencias encontradas.

### 6. Suposiciones

- **`CNTVCT_EL0` es barato en ese TV box** (decenas de ns, ~18 µs por fotograma para las 640 lecturas) →
  prototipar: sí (paso 1-3) → si es falso: rama 5 (reparar), con el número ya medido para dimensionar la
  reparación.
  → resultado del prototipo: pendiente (se completa tras la aprobación).
- **Se puede medir el reloj del motor desde el port sin tocar el fotograma** (residuo de la fase A: el port
  ya mide `Instant::now()` al arrancar) → prototipar: sí (paso 1, con el escenario de regresión) → si es
  falso: la sonda se mueve a un binario de diagnóstico aparte y la sesión se repite.
  → resultado del prototipo: pendiente.
- **`armeabi-v7a` y `x86_64` importan** (AGENTS los lista como secundario y
  [`package_apk.ps1`](../../scripts/package_apk.ps1) los construye) → prototipar: no (es una decisión de
  producto) → si es falso: la rama 5(b) deja de tener sentido y solo queda documentar el sesgo del arnés.
  → no detecta: que alguien use un APK de 32 bits en un dispositivo donde el reloj sí cuesta.

### 7. Alcance

- **Incluye:**
  - La sonda de relojes en el port (medición, publicación y su documentación).
  - El arnés de A/B intercalado de la fase A, versionado en `scripts/diagnostico/` para que la comparación
    de variantes sea reproducible sin reconstruirlo.
  - La tolerancia del analizador al campo nuevo.
  - La decisión escrita y, si la medición la pide, la reparación del cronómetro en el motor.
- **No incluye:**
  - Tocar los contadores del bus (medidos en ~0 en la fase A).
  - Cambiar el significado o el formato de los campos de las líneas `t=`.
  - Publicar el motor en el repositorio oficial y re-fijar el SHA: trabajo propio, al final.
  - Repetir el A/B de host de la fase A: sus números ya están medidos y escritos.

### 8. Criterios de aceptación (no se modifican sin aprobación)

**Escenario: La sonda no toca el fotograma** (regresión)
Caso de uso: cualquier cambio de la sonda | Tipo: regresión
  Dado el port con la sonda compilada
  Cuando se ejecutan el arnés del motor (600, 2400 y 7200 fotogramas) y la suite del port
  Entonces los hashes guardianes siguen siendo `31184031261ea2ec`, `b54066dc06f2b76c` y `b54066dc06f2b76c`
  con 1.776.798 / 2.167.398 / 3.208.998 bloques, la suite del port sigue en 122 verdes, y las líneas `t=`
  conservan sus campos
Test: `recomp runc --bios roms/gba_bios.bin --frames 600|2400|7200` desde la raíz del port y `cargo test`
Estado previo: **pasa** (es el guardián; la sonda se añade sin tocar el bucle)

**Escenario: La sonda da el número en el dispositivo** (visible)
Caso de uso: una sesión de ≥120 s en el Android TV | Tipo: normal
  Dado el APK con la sonda y la misma escena de la sesión 17
  Cuando se lee la telemetría
  Entonces aparecen el coste del reloj del motor y el del reloj del sistema, en nanosegundos, medidos en esa
  misma sesión, y el cociente entre ambos queda escrito en el cierre junto a la rama elegida (4, 5a o 5b)
Test: revisión humana (el número lo produce el dispositivo)
Estado previo: **falla** (hoy el encabezado solo trae `reloj=`, que es el del sistema)

**Escenario: El analizador no se rompe** (regresión)
Caso de uso: el log del dispositivo abierto con la herramienta del proyecto | Tipo: regresión
  Dado [`analizar_telemetria.py`](../../scripts/diagnostico/analizar_telemetria.py) sin cambios o con el
  cambio mínimo para el campo nuevo
  Cuando se ejecuta sobre el archivo nuevo
  Entonces procesa el archivo, imprime el desglose por fases y los contadores con los mismos campos que
  sobre un archivo anterior, y sale con código 0
Test: `python scripts/diagnostico/analizar_telemetria.py --archivo <log nuevo> --top 5`
Estado previo: **pasa** (procesa los archivos de hoy; el riesgo lo introduce el campo nuevo)

**Escenario (límite): la rama de reparación deja el cronómetro barato**
Caso de uso: el cronómetro con el reloj que la sonda haya señalado como malo | Tipo: límite
  Dado el camino reparado (solo si aplica la rama 5) y el mismo A/B intercalado de la fase A
  Cuando se corren 6 ciclos de 7200 fotogramas contra la variante sin instrumentación
  Entonces la mediana pareada baja de +8,9 % a **≤1 %**, con los tres guardianes intactos
Test: el bucle de pares intercalados de la fase A, incorporado a `scripts/diagnostico/` por esta propuesta
Estado previo: **falla** (hoy +8,9 % en host), y **no aplica** si la rama elegida es la 4

### 9. Cómo lo comprobaré

- **La sonda no toca el fotograma** → comando:
  `../gba-recomp/target/release/recomp.exe runc "roms/1575 - Fire Emblem (E)(Independent).gba" --bios roms/gba_bios.bin --frames 7200`
  → pasa si: `hash: b54066dc06f2b76c`, `native_blocks: 3208998`, `fallback_steps: 0`; y `cargo test` en el
  port con 122 verdes
  → no detecta: que la sonda se ejecute por fotograma si el arranque es lo bastante corto; por eso el paso 1
  exige una sola medición al iniciar.
- **El analizador sigue procesando el archivo** → comando:
  `python scripts/diagnostico/analizar_telemetria.py --archivo <log nuevo> --top 5` comparado con el mismo
  comando sobre `scripts/diagnostico/log_tv15.txt`
  → pasa si: los campos del desglose son los mismos y sale con código 0
  → no detecta: que el número de la sonda esté en la unidad equivocada; eso lo separa el cociente con
  `reloj=`, que ya tiene unidad conocida.
- **La rama de reparación cumple** → comando: el A/B intercalado de la fase A contra `V0-base`
  → pasa si: mediana pareada ≤1 % con los guardianes intactos
  → no detecta: el efecto en el dispositivo; lo cubre la sonda, no el arnés.

**Blindaje por mutación:** M1 — que la sonda mida el reloj del sistema dos veces en lugar del reloj del
motor → debe morir *La sonda da el número en el dispositivo* (los dos números saldrían idénticos). M2 —
cambiar el factor de conversión de ticks a nanosegundos → debe morir el cociente escrito en el cierre.
Los archivos se respaldan en RAM y se restauran en un bloque `finally`, sin comandos de Git.

### 10. Terminado cuando

El número del reloj del motor está medido en el dispositivo y escrito, la rama elegida (4, 5a o 5b) está
aplicada o descartada con su motivo, los tres hashes guardianes siguen intactos, la suite del port sigue en
122 verdes, el analizador procesa el archivo nuevo, y la tabla de campos y la decisión están documentadas.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta
  como `discrepada`.
- **La sonda no se puede medir en el dispositivo** (por ejemplo, el encabezado no llega al archivo antes de
  que el usuario cierre la sesión): no se encadenan intentos; se documenta la limitación y se decide con el
  número del arnés, declarándolo como sesgo de host.
- Si la rama 5 no baja del listón del 1 % en el arnés, **no se encadenan intentos con relojes distintos**:
  se documenta el reparto y se cierra.

### Correcciones previas a presentar

- **Corregido antes de presentar:** mi primera versión de esta propuesta proponía un interruptor de
  telemetría; la fase A lo descartó con números (el grupo del bus cuesta ~0), así que aquí no aparece.
- **Corregido también:** `perf_clock` solo es público bajo la feature `perf-counters` (`lib.rs`), así que la
  sonda del port depende de la misma feature que la instrumentación: medir el reloj exige el mismo build que
  mide, y eso forma parte del paso 1.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno todavía: la propuesta se presenta antes de tocar código.

---

## Resultado (lo completa el agente después de editar)

Pendiente: se completa tras la parte 1 y, si procede, la parte 2.

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

- El número del reloj del motor en el dispositivo (escenario visible): ✔ | ✘ | ?
- La rama elegida y su motivo son los correctos para ese número: ✔ | ✘ | ?
- ¿Importan `armeabi-v7a` y `x86_64` lo suficiente para reparar su camino de reloj? (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
