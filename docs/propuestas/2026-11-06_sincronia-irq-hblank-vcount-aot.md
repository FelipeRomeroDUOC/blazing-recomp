# Propuesta: Sincronía de IRQs HBlank/VCount y Escrituras MMIO en AOT (`capi.rs` y `recomp.rs`)

Estado: discrepada
- **ID:** `2026-11-06_sincronia-irq-hblank-vcount-aot`
- **Fecha:** `2026-11-06`
- **Nivel:** `2`
- **Estado:** `discrepada`
- **Hereda de:** `2026-11-06_carga-aot-windows-y-paridad-ppu-doble`

---

## 1. Objetivo y Contexto

Eliminar el parpadeo de capas (`BG2` cuerpo del dragón y `BG3` fuego) que ocurre a partir del cuadro ~150 de la escena del dragón en ejecución AOT (`run_frame_native`), tanto en Android como en ambos paneles de `ppu_doble` en PC (rasterizador por software de `gba-core` y `NativePpu` WGSL).

### Causa raíz demostrada

1. **Al habilitar `load_native_blocks` en PC (`2026-11-06_carga-aot-windows-y-paridad-ppu-doble`), los parpadeos aparecieron simultáneamente en ambos paneles de `ppu_doble`:**
   - Tanto `ppu::render_scanline` (software) como `self.io_lineas[l]` (`NativePpu`) se capturan en `process_video_event` (`../gba-recomp/crates/gba-core/src/mem.rs:869-885`) en el ciclo `HBLANK_FLAG_CYCLE` (`1006`) de cada scanline visible `l`, justo antes de levantar `self.raise_irq(irq::HBLANK)`.
   - La rutina de interrupción (ISR) de HBlank/VCount de *Fire Emblem* dispone de la ventana entre el ciclo `1006` de la línea `l` y el ciclo `1006` de la línea `l + 1` para escribir en `BG2HOFS`, `BG2VOFS`, `BLDCNT` y `BLDALPHA` los valores correspondientes a la scanline `l + 1`.

2. **Por qué falla en AOT (`run_frame_native` + `capi.rs`) frente al intérprete (`Machine::run_frame`):**
   - **Desfase de reloj en escrituras MMIO de video (`rt_write16` / `rt_write32` en `capi.rs:78-91`):** En los bloques C11 generados (`src-c/*.c`), el compilador agrupa el costo de las instrucciones del bloque básico en una única llamada `a->tick(m, N)` **al final** del bloque básico (después de las llamadas `a->write16` / `a->write32`). Cuando un bloque C11 dentro de un callback de HBlank/VCount en ROM escribe en `0x0400_0000..=0x0400_0056` (`BLDCNT`, `BLDALPHA`, `BG2HOFS`, `BG2VOFS`), `rt_write16` llama a `bus.tick_fast(0)` sin haber sumado aún los ciclos transcurridos en el bloque básico actual. Peor aún, si la ISR se ejecuta en bloques C11 cuando `m.cpu.mode() == Mode::Irq` o `m.cpu.flag(FLAG_I)`, las escrituras de registros de video y el retorno de la excepción (`SUBS pc, lr, #4` / BIOS `0x138`) avanzan en saltos gruesos de bloque en vez de instrucción a instrucción.
   - **Resguardo incompleto en `run_frame_native` (`src/recomp.rs:139-149`):** `run_frame_native` solo forzaba `m.step()` (ejecución ciclo a ciclo) para `vblank_irq_esperando && m.cpu.flag(FLAG_I)`, pero ignoraba por completo:
     1. Cuando una interrupción de **HBlank (`0x02`) o VCount (`0x04`)** está esperando (`(m.bus.reg_if & 0x06) != 0 && m.bus.ime`) mientras la CPU aún tiene `FLAG_I` activo (terminando una sección crítica o saliendo de otra ISR).
     2. Cuando la CPU está ejecutando **dentro del modo IRQ (`m.cpu.mode() == Mode::Irq`)** mientras la escena tiene efectos de raster por scanline activos en `DISPSTAT` (`(m.bus.io[4] & 0x30) != 0`, es decir HBlank IRQ o VCount IRQ habilitados en `DISPSTAT`). Ejecutar la ISR por instrucción (`m.step()`) cuando `DISPSTAT` tiene HBlank/VCount activo garantiza que cada escritura a `BLDCNT`, `BLDALPHA`, `BG2HOFS` y `BG2VOFS` ocurra en el ciclo exacto de instrucción, idéntico al intérprete.
     3. Cuando un bloque C11 del hilo principal está por despacharse en una scanline visible con HBlank/VCount IRQ habilitado en `DISPSTAT` y el haz del raster está a punto de cruzar `HBLANK_FLAG_CYCLE` (`1006`): si se lanza un grafo C11 largo en ROM (como `g_080ba7a5` en `0x080BD700..0x080BD840`, donde el 89,87% de los `goto L_...` son saltos hacia adelante sin `chain_gate`), el grafo C11 puede acumular cientos de ciclos antes de consultar `chain_gate`, retrasando la entrada a la ISR de HBlank más allá de la scanline.

---

## 2. Archivos Implicados

1. `src/recomp.rs`
2. `src/recomp_tests.rs`
3. `../gba-recomp/crates/gba-core/src/mem.rs` (hacer `pub vline_start: u64` al igual que `pub clock` y `pub next_event`)

---

## 3. Cambios Propuestos (Antes / Ahora)

### 3.0. `../gba-recomp/crates/gba-core/src/mem.rs` — Visibilidad de `vline_start`

#### Antes (`crates/gba-core/src/mem.rs:153`)
```rust
    pub clock: u64,
    pub next_event: u64,
    vline_start: u64,
```

#### Ahora
```rust
    pub clock: u64,
    pub next_event: u64,
    pub vline_start: u64,
```

### 3.1. `src/recomp.rs` — Sincronización de precisión en `run_frame_native` para escenas con HBlank/VCount IRQ activo

#### Antes (`src/recomp.rs:138-151`)
```rust
        // 1. Estados de reposo (Halt) y entrega de interrupciones (IRQ)
        let en_vblank = m.bus.scanline() >= 160;
        let vblank_irq_esperando = en_vblank && (m.bus.reg_if & 0x01) != 0 && m.bus.ime;

        if m.bus.halted
            || (!m.bus.real_bios && m.cpu.regs[15] == IRQ_RETURN_ADDR && m.cpu.mode() == Mode::Irq)
            || (m.bus.irq_pending() && !m.cpu.flag(FLAG_I))
            || (vblank_irq_esperando && m.cpu.flag(FLAG_I))
        {
            m.step();
            continue;
        }
```

#### Ahora (`src/recomp.rs:138-168`)
```rust
        // 1. Estados de reposo (Halt) y entrega de interrupciones (IRQ)
        let scanline = m.bus.scanline();
        let en_vblank = scanline >= 160;
        let dispstat = m.bus.io[0x04];
        let raster_irq_habilitado = (dispstat & 0x30) != 0 && m.bus.ime;
        let irq_video_esperando = m.bus.ime
            && ((en_vblank && (m.bus.reg_if & 0x01) != 0)
                || ((m.bus.reg_if & 0x06) != 0));
        let ciclo_linea = m.bus.clock.wrapping_sub(m.bus.vline_start);
        let ventana_hblank_inminente =
            raster_irq_habilitado && !en_vblank && (880..1060).contains(&ciclo_linea);

        if m.bus.halted
            || (!m.bus.real_bios && m.cpu.regs[15] == IRQ_RETURN_ADDR && m.cpu.mode() == Mode::Irq)
            || (m.bus.irq_pending() && !m.cpu.flag(FLAG_I))
            || (irq_video_esperando && m.cpu.flag(FLAG_I))
            || (raster_irq_habilitado && m.cpu.mode() == Mode::Irq)
            || ventana_hblank_inminente
        {
            m.step();
            continue;
        }
```

### 3.2. `src/recomp_tests.rs` — Test unitario de despacho oportuno con HBlank/VCount IRQ en `run_frame_native`

#### Antes
No existe un test que verifique que `run_frame_native` delega a paso fino (`m.step()`) cuando `DISPSTAT` tiene HBlank/VCount activo (`0x30`) en modo `Mode::Irq` o con `reg_if & 0x06` pendiente.

#### Ahora
Se agrega `test_run_frame_native_sincroniza_hblank_irq_en_modo_irq` en `src/recomp_tests.rs`, comprobando que cuando `DISPSTAT` tiene HBlank IRQ activo (`io[0x04] = 0x10`, `ime = true`) y la CPU está en `Mode::Irq` o en ventana de HBlank, `run_frame_native` ejecuta con precisión de ciclo sin saltar bloques AOT desfasados.

---

## 4. Criterios de Éxito y Plan de Verificación

- [x] **Criterio 1 (Suite de tests en verde):** `python scripts/run_tests_summary.py recomp` pasa al 100% incluyendo `test_run_frame_native_sincroniza_hblank_irq_en_modo_irq`.
- [x] **Criterio 2 (Compilación de `ppu_doble`):** `cargo check --bin ppu_doble --features="proto"` compila sin errores.
- [ ] **Criterio 3 (Revisión humana en PC con `ppu_doble`):** Al ejecutar `cargo run --release --features proto --bin ppu_doble -- "roms/1575 - Fire Emblem (E)(Independent).gba" "roms/gba_bios.bin"`, los parpadeos de la escena del dragón desaparecen o se reducen drásticamente en ambos paneles (Software PPU y NativePpu WGSL) manteniendo ejecución AOT nativa.

---

## 5. Evaluación

- **Estado:** `discrepada`
- **Resultado:** `parcial`
- **Revisión humana:** `✘` (Parpadeos disminuyeron drásticamente, pero quedan dos parpadeos justo antes de los relámpagos en ambos lados).

### 5.1. Hecho observado vs. predicho
- **Predicho:** Limitar el paso a paso a la ventana `(880..1060)` bastaría para cubrir la atención de la ISR de HBlank.
- **Observado:** Los parpadeos disminuyeron sustancialmente, pero persisten 2 parpadeos previo al relámpago. Entre el ciclo 1006 y el 1232 la máquina permanece en HBlank. Si la ISR concluye y retorna a `Mode::System` en el ciclo ~1150, la condición `(880..1060).contains(&ciclo_linea)` evalúa a `false`, permitiendo a `run_frame_native` despachar un bloque C11 AOT en bulto en el ciclo 1150 que arrolla la frontera de línea (ciclo 1232 / inicio de scanline) desfasando la modulación de `BLDY`/`BLDCNT` del efecto de relámpago.

### 5.2. Recorrido de piezas
- **Sincronía en `Mode::Irq`:** Sigue valiendo.
- **Resguardo `irq_video_esperando` con `FLAG_I`:** Sigue valiendo.
- **Ventana de HBlank `(880..1060)`:** Hay que ajustar para cubrir todo el HBlank activo y el cruce de línea (`ciclo_linea >= 860 || ciclo_linea < 80`).
- **Resumen de la lista:** 2 siguen valiendo, 1 hay que ajustar, 0 dejan de valer.

### 5.3. Opciones
- **Opción B (Recomendada):** Propuesta de corrección que hereda (`2026-11-06_cobertura-completa-hblank-y-cruce-scanlines-aot`), ampliando la ventana a `ciclo_linea >= 860 || ciclo_linea < 80` para proteger todo el HBlank (1006..1232) y la frontera de scanline.
- **Opción C:** Deshacer y volver al intérprete completo. No se recomienda porque la sincronía AOT ya eliminó la gran mayoría de los parpadeos.
