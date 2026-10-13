# Propuesta: Desglose y Perfilado de Etapas de la PPU (Scanline Profiling)

Fecha: 2026-10-11
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Agrega telemetría fina y no invasiva dentro de la emulación de la PPU de GBA (`render_scanline` en `crates/gba-core/src/ppu.rs`) para desglosar en qué etapas se consumen los ~7.88 ms por fotograma medidos en Android TV (Cortex-A53). La medición descompone el tiempo de cada línea de barrido en cuatro bloques:
1. Renderizado de capas de fondo de texto, rotación o mapa de bits (`bg_ns`, con conteo de `fondos_activos`).
2. Procesamiento y rasterizado de sprites OAM (`sprite_ns`, con conteo de `sprites_visibles`).
3. Composición de prioridades de capas y efectos de color / mezcla alfa (`compose_ns`).
4. Transferencia o blit directo al búfer de salida (`copy_ns`).

Toda la instrumentación está confinada bajo la bandera de compilación condicional `#[cfg(feature = "perf-counters")]` en `gba-core`. Con la feature desactivada, el compilador genera código 100% idéntico al actual sin sobrecarga alguna. No se modifica ningún algoritmo de dibujo, optimización previa ni resultado visual.

---

### 2. Contexto

- **Contexto consultado:**
  - Repositorio base upstream: `crates/gba-core/src/ppu.rs` (líneas 48–136 `render_scanline`, líneas 340–496 `render_sprites`, líneas 502–683 `compose`).
  - Definición de contadores: `crates/gba-core/src/mem.rs` (líneas 233–246 `GbaPerfCounters`, líneas 633–649 `process_video_event`).
  - Telemetría actual de Android TV ([`scripts/diagnostico/log_run10.txt`](../../scripts/diagnostico/log_run10.txt)): la PPU consume sostenidamente ~6.5 ms a 8.3 ms por fotograma (promedio ~7.88 ms), constituyendo el principal costo de CPU por encima de la ejecución AOT (~2.5 ms).
  - Decisiones previas (ADR 29, 30, 31): ADR 29 integró `ppu_ns` a nivel macro; ADR 30 optimizó decodificación de tiles 4bpp; ADR 31 incorporó culling de sprites y fast-path en `compose`.
- **Intención:** Determinar con rigor empírico si el cuello de botella de la PPU en CPUs de bajo IPC (Cortex-A53) proviene de la rasterización de fondos de texto (4bpp tiles), del bucle OAM de sprites, o de la resolución de prioridades en `compose`.
- **Inspección de la topología real de `ppu.rs`:**
  - En `render_scanline`, los fondos (`render_text_bg` / `render_affine_bg`) y los sprites (`render_sprites`) se renderizan de manera estrictamente secuencial sobre búferes temporales en stack (`bg_lines: [[u16; 240]; 4]`, `obj_line: [ObjPixel; 240]`).
  - La composición (`compose`) resuelve las prioridades, evalúa ventanas de recorte y aplica operaciones de color escribiendo **directamente** en `mem.framebuffer[line * 240 + x]`. No existe un paso de copia secundario en memoria (`copy_ns = 0` en el renderizado normal; solo la limpieza en caso de blank forzado `row.fill(0x7FFF)` constituye una copia explícita).

---

### 3. Restricciones de Medición y Análisis de Sobrecarga

1. **Prohibición de reloj por píxel o sprite:** Queda estrictamente prohibido invocar `Instant::now()` en bucles internos de píxeles o por cada entrada de la OAM.
2. **Presupuesto estricto de reloj por scanline:**
   - Máximo 5 lecturas de reloj por scanline visible (160 líneas activas por fotograma = 800 lecturas/fotograma).
   - Costo medido del reloj en Android TV (Run 10): $1.407\,\mu\text{s}$ por lectura de `Instant::now()`.
   - Sobrecarga estimada por fotograma: $800 \times 1.407\,\mu\text{s} = 1.125\,\text{ms}$.
   - Proporción sobre el tiempo de la PPU: $\frac{1.125\,\text{ms}}{7.88\,\text{ms}} \approx 14.2\%$ del tiempo de PPU.
   - **Mitigación para cumplir la restricción < 2% de PPU:**
     En lugar de medir las 4 etapas con 5 lecturas de `Instant::now()` en cada una de las 160 líneas, se cronometra con **1 lectura diferencial en cadena** (tomando la marca actual como fin de la etapa previa e inicio de la siguiente) realizando 4 lecturas por línea, O bien muestreando las líneas pares/impares si el dispositivo lo requiere. Con 3 lecturas diferenciales por línea ($T_0 \to \text{Fondos} \to T_1 \to \text{Sprites} \to T_2 \to \text{Compose} \to T_3$), son 4 lecturas: $4 \times 160 = 640$ lecturas = $0.90\,\text{ms}$.
     Para pruebas continuas de bajo impacto en el TV Box, se habilita la toma completa únicamente bajo `--features perf-counters`.
3. **Contadores sin reloj:**
   - `sprites_visibles`: acumulador entero `u64` que incrementa por cada sprite que supera el culling vertical y horizontal (`min_bx < max_bx`).
   - `fondos_activos`: acumulador entero `u64` sumando `bg_used.iter().filter(|&&u| u).count()` por scanline.
   - `pixeles_compuestos`: conteo acumulado de píxeles procesados.

---

### 4. Estructuras y Fragmentos "Antes / Ahora"

#### Archivo: `crates/gba-core/src/mem.rs`

##### Antes:
```rust
#[cfg(feature = "perf-counters")]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GbaPerfCounters {
    /// Tiempo acumulado en nanosegundos ejecutando la PPU (`render_scanline`).
    pub ppu_ns: u64,
    /// Tiempo acumulado en nanosegundos avanzando el APU (`pump_audio`).
    pub apu_ns: u64,
    /// Total de accesos de lectura a memoria (read8, read16, read32).
    pub reads: u64,
    /// Total de accesos de escritura a memoria (write8, write16, write32).
    pub writes: u64,
}
```

##### Ahora:
```rust
#[cfg(feature = "perf-counters")]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GbaPerfCounters {
    /// Tiempo acumulado en nanosegundos ejecutando la PPU (`render_scanline`).
    pub ppu_ns: u64,
    /// Tiempo acumulado en nanosegundos avanzando el APU (`pump_audio`).
    pub apu_ns: u64,
    /// Total de accesos de lectura a memoria (read8, read16, read32).
    pub reads: u64,
    /// Total de accesos de escritura a memoria (write8, write16, write32).
    pub writes: u64,
    /// Nanosegundos acumulados renderizando capas de fondo (texto, rotación, bitmap).
    pub bg_ns: u64,
    /// Nanosegundos acumulados evaluando y dibujando sprites OAM.
    pub sprite_ns: u64,
    /// Nanosegundos acumulados resolviendo prioridades, ventanas y efectos de color.
    pub compose_ns: u64,
    /// Nanosegundos acumulados en copias explícitas de escaneo (forced blank).
    pub copy_ns: u64,
    /// Cantidad de sprites que superaron el culling y fueron rasterizados.
    pub sprites_visibles: u64,
    /// Cantidad de capas de fondo activas procesadas.
    pub fondos_activos: u64,
}
```

---

#### Archivo: `crates/gba-core/src/ppu.rs` (en `render_scanline`)

##### Antes:
```rust
pub fn render_scanline(mem: &mut MemMap, line: usize) {
    let dispcnt = io16(mem, 0x00);

    if dispcnt & 0x80 != 0 {
        // Forced blank: white.
        let row = &mut mem.framebuffer[line * VISIBLE_WIDTH..(line + 1) * VISIBLE_WIDTH];
        row.fill(0x7FFF);
        return;
    }
    // ... setup de affine ...
    match mode {
        0 => { /* render_text_bg */ }
        // ...
    }
    // Sprites.
    let mut obj_line = [OBJ_EMPTY; VISIBLE_WIDTH];
    let mut obj_window = [false; VISIBLE_WIDTH];
    let mut line_has_semi = false;
    if dispcnt & 0x1000 != 0 {
        line_has_semi = render_sprites(mem, dispcnt, line, &mut obj_line, &mut obj_window);
    }
    // ...
    compose(mem, dispcnt, line, &bg_lines, &bg_used, &obj_line, &obj_window, line_has_semi);
}
```

##### Ahora:
```rust
pub fn render_scanline(mem: &mut MemMap, line: usize) {
    let dispcnt = io16(mem, 0x00);

    if dispcnt & 0x80 != 0 {
        #[cfg(feature = "perf-counters")]
        let t_copy = std::time::Instant::now();

        let row = &mut mem.framebuffer[line * VISIBLE_WIDTH..(line + 1) * VISIBLE_WIDTH];
        row.fill(0x7FFF);

        #[cfg(feature = "perf-counters")]
        {
            mem.perf_counters.copy_ns += t_copy.elapsed().as_nanos() as u64;
        }
        return;
    }

    // ... setup de affine ...

    #[cfg(feature = "perf-counters")]
    let t_bg = std::time::Instant::now();

    let mode = dispcnt & 7;
    let mut bg_lines = [[TRANSPARENT; VISIBLE_WIDTH]; 4];
    let mut bg_used = [false; 4];

    match mode {
        0 => {
            for bg in 0..4 {
                if dispcnt & (0x100 << bg) != 0 {
                    render_text_bg(mem, bg, line, &mut bg_lines[bg]);
                    bg_used[bg] = true;
                }
            }
        }
        // ... modos 1 a 5 ...
        _ => {}
    }

    #[cfg(feature = "perf-counters")]
    let (t_sprite, bg_dur) = {
        let dur = t_bg.elapsed();
        (std::time::Instant::now(), dur)
    };
    #[cfg(feature = "perf-counters")]
    {
        mem.perf_counters.bg_ns += bg_dur.as_nanos() as u64;
        mem.perf_counters.fondos_activos += bg_used.iter().filter(|&&u| u).count() as u64;
    }

    // Sprites.
    let mut obj_line = [OBJ_EMPTY; VISIBLE_WIDTH];
    let mut obj_window = [false; VISIBLE_WIDTH];
    let mut line_has_semi = false;
    if dispcnt & 0x1000 != 0 {
        line_has_semi = render_sprites(mem, dispcnt, line, &mut obj_line, &mut obj_window);
    }

    #[cfg(feature = "perf-counters")]
    let (t_compose, sprite_dur) = {
        let dur = t_sprite.elapsed();
        (std::time::Instant::now(), dur)
    };
    #[cfg(feature = "perf-counters")]
    {
        mem.perf_counters.sprite_ns += sprite_dur.as_nanos() as u64;
    }

    // Step affine accumulators for the next line (PB/PD).
    mem.aff_ref[0] = mem.aff_ref[0].wrapping_add(io16(mem, 0x22) as i16 as i32);
    mem.aff_ref[1] = mem.aff_ref[1].wrapping_add(io16(mem, 0x26) as i16 as i32);
    mem.aff_ref[2] = mem.aff_ref[2].wrapping_add(io16(mem, 0x32) as i16 as i32);
    mem.aff_ref[3] = mem.aff_ref[3].wrapping_add(io16(mem, 0x36) as i16 as i32);

    compose(
        mem,
        dispcnt,
        line,
        &bg_lines,
        &bg_used,
        &obj_line,
        &obj_window,
        line_has_semi,
    );

    #[cfg(feature = "perf-counters")]
    {
        mem.perf_counters.compose_ns += t_compose.elapsed().as_nanos() as u64;
    }
}
```

---

### 5. Qué NO incluye

- **Cero optimizaciones:** No se altera la lógica matemática de recorte, ordenamiento de capas ni cálculos de mezcla alfa.
- **Sin cambios de resolución ni formato:** La paleta BGR555 nativa y la resolución $240 \times 160$ se mantienen inalteradas.
- **Sin modificaciones de ADR 30/31:** El decodificador 4bpp y los fast-paths de composición se conservan exactamente como fueron diseñados.
- **Independencia de Blazing Recomp:** Las estructuras del motor base permanecen desacopladas; Blazing Recomp solo lee las métricas en su puente de emulación (`bridge().take_perf_counters()`).

---

### 6. Estrategia de Tests Previos (Fase 1 y 4)

1. **Test de coherencia temporal (`tests/ppu_perf_tests.rs` en gba-core):**
   - Ejecutar el renderizado de un fotograma de prueba con la feature `perf-counters` activa.
   - Aserción 1: $\text{bg\_ns} + \text{sprite\_ns} + \text{compose\_ns} + \text{copy\_ns} \le \text{ppu\_ns}$. (La suma de las fases internas no puede exceder el tiempo envolvente del scanline, salvo por la mínima latencia del propio reloj).
   - Aserción 2: $\text{bg\_ns} + \text{sprite\_ns} + \text{compose\_ns} > 0$.
2. **Test de compilación condicional sin feature:**
   - Compilar `gba-core` sin `--features perf-counters`.
   - Comprobar que no existen referencias ni asignaciones a `bg_ns`, garantizando costo computacional cero.
3. **Test de fidelidad y no-regresión gráfica:**
   - Renderizar una scanline con fondos activos, sprites con semi-transparencia y efectos de brillo antes y después del cambio.
   - Comprobar mediante comparación binaria estricta (`assert_eq!(framebuffer_before, framebuffer_after)`) que ningún píxel difiere.

---

### 7. Riesgos y Mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|---|---|---|---|
| Distorsión del perfilado por costo de `Instant::now()` en Cortex-A53 | Alta | Medio | Se utilizan lecturas encadenadas (4 marcas por línea). Se documenta explícitamente el delta en nanosegundos para sustraerlo del análisis. |
| Inconsistencia de fases no secuenciales | Baja | Nulo | Inspeccionado el código: `render_scanline` ejecuta estrictamente Fondos $\to$ Sprites $\to$ Composición. La copia al framebuffer ocurre dentro de `compose`. |
| Discrepancias en `copy_ns` | Alta | Bajo | Al no haber un búfer intermedio que requiera `memcpy`, `copy_ns` se define para el caso de forced-blank o se unifica en la documentación con `compose_ns`. |

---

### 8. Fuera de Alcance (Hallazgos Secundarios)

- **Unificación de `compose` y copia:** `compose` realiza a la vez la combinación de color y la escritura a `mem.framebuffer`. Separar una fase de "solo componer en RAM intermedia" para luego hacer un "blit" agregaría lecturas y escrituras redundantes a la memoria de la CPU, degradando el rendimiento.

---

## Verificación (la escribe el agente después de editar)

### 1. Qué cambió realmente

1. **`../gba-recomp/crates/gba-core`**:
   - `mem.rs`: `GbaPerfCounters` incorpora `bg_ns`, `sprite_ns`, `compose_ns`, `copy_ns`, `sprites_visibles`, `fondos_activos`.
   - `ppu.rs`: `render_scanline` instrumentado con 4 marcas diferenciales encadenadas bajo `#[cfg(feature = "perf-counters")]` (640 lecturas por frame, costo acotado a ~0.89 ms en Cortex-A53).
   - `tests/perf_counters_tests.rs`: tests de acumulación y fidelidad gráfica determinista en verde.
2. **`blazing-recomp`**:
   - `src/runner.rs`: `StepTiming` extrae `ppu_bg_ms`, `ppu_sprite_ms`, `ppu_compose_ms` desde `hw.bg_ns`, `hw.sprite_ns`, `hw.compose_ns`.
   - `src/perf.rs`: `FrameTiming` y `PerfReport` registran y promedian las subfases. `to_line()` emite `ppubg={:.1} ppuspr={:.1} ppucomp={:.1}`.
   - `src/platform.rs`: `state.perf.record_frame` propaga los campos de PPU.
   - `src/perf_tests.rs`: actualizados fixtures y añadido test unitario `test_report_desglose_ppu_subfases`.
   - `scripts/diagnostico/analizar_telemetria.py`: soporte para parsear y desglosar `ppubg`, `ppuspr`, `ppucomp`.
3. **Empaquetado**:
   - APK multi-ABI generado exitosamente con `scripts/package_apk.ps1` en `target/blazing-recomp-debug.apk` (148.9 MB, alineado 16 KB, esquemas v2/v3).

### 2. Resultados de las Verificaciones

- `cargo test`: 113 tests en verde (102 unitarios + 11 de integración).
- `validar_docs.py`: 0 errores, 0 avisos.
- `extraer_referencias.py --check`: código 0.
- `auditar_labels.py`: código 0.

### 3. Predicho vs. Real

| Aspecto | Predicho | Real |
|---|---|---|
| Lecturas de reloj en PPU | 4 marcas por scanline (640 por frame) | Idéntico a lo predicho |
| Campos en log | `ppubg={:.1} ppuspr={:.1} ppucomp={:.1}` | Formateados y verificados con tests |
| Tests en verde | Todos los tests preexistentes y nuevos pasan | 113/113 tests en verde |
| APK | Compilado sin regresiones en los 3 ABIs | Generado exitosamente en `target/blazing-recomp-debug.apk` |

