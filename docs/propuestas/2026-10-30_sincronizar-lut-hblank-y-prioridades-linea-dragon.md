# Propuesta: Sincronizar parada Halt/IRQ en rt_interp_one y orden de staging en PPU nativa para eliminar el parpadeo del fuego en la cinemática del dragón

Fecha: 2026-10-30
Nivel: 2
Estado: discrepada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La textura naranja de fuego (`BG3`) sobre el dragón (`BG2`) en la cinemática de apertura (`"sacudió los cielos"`) permanece visible en todos los fotogramas de Android sin desaparecer aleatoriamente cuando avanza el texto.

### 2. Contexto

- Contexto consultado: `../gba-recomp/crates/gba-core/src/capi.rs`, `../gba-recomp/crates/gba-core/src/mem.rs`, `src-c/1575 - Fire Emblem (E)(Independent)-bios.0.c`, `src/recomp.rs`, `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`, `examples/ppu_nativa_atribuye.rs`, `volcados/atribucion-769.txt`, `volcados/atribucion-770.txt`, `docs/propuestas/2026-10-30_fix-parpadeo-fuego-objwin-y-conmutacion-oam.md`.
- Intención: Evitar que el camino AOT (`rt_interp_one` en `capi.rs`) ejecute prematuramente la instrucción posterior a `HALTCNT` (`0x000001B4` en `b_000001ac_a`) o retrase la entrega de interrupciones VBlank/VCount dejando `BLDCNT = 0x0144` (sin mezcla entre `BG2` y `BG3`) durante las scanlines `0..99`, y corregir el orden de subida de `staging_filas_comp` en `src/ppu_nativa.rs`.
- Restricciones y decisiones previas (máx. 3):
  - Mantener la composición directa en GPU con `NativePpu` y la captura de `io_lineas` alineada con el oráculo de software en `VPhase::Hblank` (`AGENTS.md §1`) → implica: no desplazar la captura de `io_lineas[l]` respecto a `ppu::render_scanline` en `mem.rs` → protegida por: `rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar`
  - No regenerar los 144.875 bloques C11 de `src-c/` (`AGENTS.md §1`) → implica: interceptar el estado `halted` e `irq_pending` directamente en `rt_interp_one` y `rt_tick` dentro de `../gba-recomp/crates/gba-core/src/capi.rs` → protegida por: `rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar`
  - Preservar la visibilidad del fuego a 60 FPS en el dispositivo Android real (origen: capturas del usuario, 2026-10-30) → implica: verificar visualmente la escena `"sacudió los cielos"` en Android → protegida por: no testeable
- Alternativas descartadas (solo si las hubo):
  - Mover la captura de `io_lineas[l]` a `VPhase::LineEnd` en `mem.rs` → descartada tras la auditoría porque `ppu::render_scanline` se ejecuta al inicio de `VPhase::Hblank` (`mem.rs:877`), y mover `io_lineas[l]` a `LineEnd` desplazaría los efectos HBlank en 1 línea respecto al oráculo de software.
  - Forzar `DISPCNT` u `OBJWIN` constante en todo el cuadro (`2026-10-28`, `2026-10-29`, `2026-10-30`) → descartada porque en `volcados/atribucion-769.txt:18-25` y `770.txt:18-26`, `DISPCNT` ya vale `0x9F00` en las 160 líneas y `OAM` tiene 0 cambios; el fuego desaparece únicamente cuando `BLDCNT` (`0x50`) conserva en las líneas `0..99` el valor `0x0144` / `0x3F28` de la línea `100` (banda del subtítulo) por desincronización de `IntrWait` / IRQ en AOT.
- Ante imprevistos, prioriza: exactitud del despacho de Halt/IRQ en `capi.rs` y coherencia de los búferes por línea en `src/ppu_nativa.rs`.

### 3. Ejemplos

- Caso normal (escritura a `HALTCNT` en `b_000001ac_a` dentro de `VBlankIntrWait` / `IntrWait` de la BIOS):
  - Entrada:El bloque recompilado `b_000001ac_a` (`bios.0.c:2729-2746`) escribe `0` en `0x04000301` (`HALTCNT`), lo que activa `m.bus.halted = true`, y a continuación ejecuta `a->tick(m, 3u); c[15] = 0x000001b4u; return a->interp_one(m);`.
  - Antes:`rt_interp_one` (`capi.rs:116-126`) no comprueba `m.bus.halted` ni `m.bus.irq_pending()`: ejecuta inmediatamente la instrucción en `0x000001B4` (`bx lr`) mientras la CPU está en estado `halted`, antes de que ocurra la interrupción y antes de que el vector `0x00000018` tome el control. Al avanzar el subtítulo `"sacudió los cielos"`, esto desincroniza `IntrWait` entre la interrupción `VCount=100` (que apaga el fuego con `BLDCNT = 0x0144` para la caja de texto inferior) y la interrupción `VBlank` (que restaura `BLDCNT = 0x0844` para las líneas `0..99`).
  - Ahora:En `capi.rs`, `rt_tick` no consume ciclos de instrucción adicionales cuando `m.bus.halted` ya está activo, y `rt_interp_one` retorna inmediatamente `m.cpu.regs[15] | m.cpu.thumb() as u32` sin ejecutar la instrucción cuando `m.bus.halted` es `true` o cuando hay una interrupción entregable (`m.bus.irq_pending() && m.cpu.cpsr & FLAG_I == 0`), devolviendo el control a `run_frame_native` para que `m.step()` detenga la CPU en `0x000001B4` y despache la ISR en `0x00000018` antes de ejecutar `0x000001B4`.

```rust
// Antes en ../gba-recomp/crates/gba-core/src/capi.rs (líneas 80-82 y 116-126):
extern "C" fn rt_tick(m: *mut core::ffi::c_void, cycles: u32) {
    mach!(m).bus.tick_fast(cycles as u64)
}
// ...
pub extern "C" fn rt_interp_one(m: *mut core::ffi::c_void) -> u32 {
    let m = unsafe { &mut *(m as *mut Machine) };
    let region = (m.cpu.regs[15] >> 24) as usize & 0xF;
    let instr = if m.bus.real_bios {
        exec::step_cached(&mut m.cpu, &mut m.bus, &mut m.decode_cache)
    } else {
        exec::step_hle_cached(&mut m.cpu, &mut m.bus, &mut m.decode_cache)
    };
    m.bus.tick_fast(crate::machine::instr_cost(region, &instr));
    m.cpu.regs[15] | m.cpu.thumb() as u32
}

// Ahora en ../gba-recomp/crates/gba-core/src/capi.rs:
extern "C" fn rt_tick(m: *mut core::ffi::c_void, cycles: u32) {
    let mach = mach!(m);
    if !mach.bus.halted {
        mach.bus.tick_fast(cycles as u64);
    }
}
// ...
pub extern "C" fn rt_interp_one(m: *mut core::ffi::c_void) -> u32 {
    let m = unsafe { &mut *(m as *mut Machine) };
    if m.bus.halted || (m.bus.irq_pending() && m.cpu.cpsr & crate::cpu::FLAG_I == 0) {
        return m.cpu.regs[15] | m.cpu.thumb() as u32;
    }
    let region = (m.cpu.regs[15] >> 24) as usize & 0xF;
    let instr = if m.bus.real_bios {
        exec::step_cached(&mut m.cpu, &mut m.bus, &mut m.decode_cache)
    } else {
        exec::step_hle_cached(&mut m.cpu, &mut m.bus, &mut m.decode_cache)
    };
    if !m.bus.halted {
        m.bus.tick_fast(crate::machine::instr_cost(region, &instr));
    }
    m.cpu.regs[15] | m.cpu.thumb() as u32
}
```

- Caso límite (recuperación de prioridad por scanline y consolidación de sprites antes de subir `filas_comp_buffer` en `src/ppu_nativa.rs`):
  - Entrada:En `subir_buffers_lineas`, `io_fin` tiene una capa de fondo desactivada o con prioridad distinta a la de las líneas activas `0..99`, o `decode_oam` detecta sprites `OBJWIN` (`num_sprites_ventana > 0`).
  - Antes:En `src/ppu_nativa.rs:1080-1124`, `consolidar_dispcnt_fondos` y `propagar_fondos_a_staging` toman `self.capas[bg].priority` únicamente de `io_fin`; además, en `src/ppu_nativa.rs:1881-1908`, `queue.write_buffer(&self.filas_comp_buffer, ...)` se ejecuta antes de `consolidar_dispcnt_sprites` y `decode_oam`.
  - Ahora:`consolidar_dispcnt_fondos` recupera la prioridad de la primera línea donde la capa estuvo activa en `comps` cuando `self.bg_prio[bg] == 0xFF`, y `consolidar_dispcnt_sprites` junto con la propagación del bit `0x8000` (`OBJWIN`) cuando `num_sprites_ventana > 0` se ejecutan antes de subir `filas_comp_buffer` a la GPU.

```rust
// Antes en src/ppu_nativa.rs (líneas 1879-1895):
if let Some((a, b)) = filas_comp {
    let tam = std::mem::size_of::<ConfigCompGpu>();
    queue.write_buffer(
        &self.filas_comp_buffer,
        (a * tam) as u64,
        bytemuck::cast_slice(&self.staging_filas_comp[a..=b]),
    );
    self.filas_comp_previa[a..=b].copy_from_slice(&self.staging_filas_comp[a..=b]);
}
{
    self.estado
        .consolidar_dispcnt_sprites(&self.staging_filas_comp[..]);

// Ahora en src/ppu_nativa.rs:
self.estado
    .consolidar_dispcnt_sprites(&self.staging_filas_comp[..]);
if self.estado.num_sprites_ventana > 0 {
    self.estado.dispcnt |= 0x8000;
    for fila in self.staging_filas_comp.iter_mut() {
        fila.dispcnt |= 0x8000;
    }
}
if let Some((a, b)) = filas_comp {
    let tam = std::mem::size_of::<ConfigCompGpu>();
    queue.write_buffer(
        &self.filas_comp_buffer,
        (a * tam) as u64,
        bytemuck::cast_slice(&self.staging_filas_comp[a..=b]),
    );
    self.filas_comp_previa[a..=b].copy_from_slice(&self.staging_filas_comp[a..=b]);
}
```

### 4. Pasos

1. En `../gba-recomp/crates/gba-core/src/capi.rs`, modificar `rt_tick` para no avanzar `tick_fast` si `mach.bus.halted` es `true`, y modificar `rt_interp_one` para que, si `m.bus.halted || (m.bus.irq_pending() && m.cpu.cpsr & crate::cpu::FLAG_I == 0)`, retorne inmediatamente `m.cpu.regs[15] | m.cpu.thumb() as u32` sin ejecutar la instrucción en `regs[15]`, devolviendo el control al bucle de despacho (`run_frame_native`).
2. En `src/ppu_nativa.rs` (`consolidar_dispcnt_fondos` y `propagar_fondos_a_staging`), cuando una capa `bg` tiene `self.bg_prio[bg] == 0xFF` en `io_fin` pero está activa en alguna fila de `comps`, recuperar su prioridad desde la primera fila de `comps` donde `bg_prio` no sea `0xFF` (con fallback a `self.capas[bg].priority`).
3. En `src/ppu_nativa.rs` (`subir_buffers_lineas`), ejecutar `consolidar_dispcnt_sprites` y la decodificación condicional de `OAM` antes de calcular `rango_filas_modificadas` y subir `self.filas_comp_buffer`, propagando el bit `0x8000` (`OBJWIN`) a `staging_filas_comp` cuando `self.estado.num_sprites_ventana > 0`.
4. En `examples/ppu_nativa_atribuye.rs` (líneas 378-387), corregir la tabla de nombres de registros de diagnóstico (`0x40=WIN0H`, `0x42=WIN1H`, `0x44=WIN0V`, `0x46=WIN1V`, `0x48=WININ`, `0x4A=WINOUT`, `0x4C=MOSAIC`, `0x50=BLDCNT`, `0x52=BLDALPHA`, `0x54=BLDY`).

### 5. Qué más podría verse afectado

- `src/recomp.rs` (`run_frame_native`): recibe el retorno temprano de `rt_interp_one` cuando `m.bus.halted` o `irq_pending()` se activan dentro de un bloque C11, entrando de inmediato en la rama `if m.bus.halted || ... { m.step(); continue; }` (líneas 139-145).
- `src/platform.rs` (`render_frame_lineas`) y `src/bin/ppu_doble.rs`: consumen `subir_buffers_lineas` sin cambios en su firma pública.
- `src/ppu_nativa_tests.rs`: se mantienen en verde todos los tests existentes (`test_propagacion_fondos_consolidados_a_staging_buffers`, `test_dispcnt_consolida_bit_objwin`) y se agregan los nuevos tests de aceptación.

### 6. Suposiciones

- Retornar en `rt_interp_one` sin ejecutar la instrucción cuando `m.bus.halted` o `irq_pending()` están activos reproduce exactamente el comportamiento del intérprete (`Machine::step` en `machine.rs:102-121`) para todos los bloques de BIOS y ROM → prototipar: no
  → si es falso: aplicar el retorno temprano en `rt_interp_one` únicamente cuando `m.bus.halted` es `true`, manteniendo el chequeo de `irq_pending` en `rt_chain_gate`.

### 7. Alcance

- Incluye:
  - Corrección de `rt_tick` y `rt_interp_one` ante estados `halted` e `irq_pending` en `../gba-recomp/crates/gba-core/src/capi.rs`.
  - Recuperación de prioridad activa por línea en `consolidar_dispcnt_fondos` / `propagar_fondos_a_staging` y reordenamiento de `consolidar_dispcnt_sprites` + `decode_oam` antes de `write_buffer(&self.filas_comp_buffer)` en `src/ppu_nativa.rs`.
  - Corrección de las etiquetas de registros `0x40..0x54` en `examples/ppu_nativa_atribuye.rs`.
  - Pruebas unitarias en `src/ppu_nativa_tests.rs`.
- No incluye:
  - Mover la captura de `io_lineas` en `../gba-recomp/crates/gba-core/src/mem.rs`.
  - Regenerar los 144.875 bloques C11 en `src-c/`.
  - Modificar el shader `src/ppu_nativa.wgsl`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: `rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar`
Caso de uso: Cuando un bloque AOT escribe `HALTCNT` (`0x04000301`) o deja una IRQ pendiente antes de invocar `rt_interp_one`, la instrucción en `PC` no se ejecuta hasta que `Machine::step` despacha el Halt o la interrupción | Tipo: normal
  Dado una instancia de `Machine` donde se escribe `0` en `0x04000301` (`HALTCNT`, activando `bus.halted = true`) y `cpu.regs[15]` apunta a `0x000001B4` (`bx lr`)
  Cuando se invoca `gba_core::capi::rt_interp_one` seguido de `rt_tick`
  Entonces `rt_interp_one` retorna `0x000001B4` sin ejecutar `bx lr`, `cpu.regs[15]` permanece en `0x000001B4` y `bus.halted` sigue siendo `true`
Test: `src/ppu_nativa_tests.rs::rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar`
Estado previo: falla

Escenario: `consolidar_fondos_recupera_prioridad_activa_cuando_io_fin_la_altera`
Caso de uso: Si en `io_fin` una capa de fondo está apagada y su `BGxCNT` fue puesto a `0`, pero en las líneas `0..99` estuvo activa con prioridad `3`, `consolidar_dispcnt_fondos` y `propagar_fondos_a_staging` preservan la prioridad `3` | Tipo: límite
  Dado un `EstadoPpuNativa` donde `io_lineas[0..100]` tiene `DISPCNT = 0x9F00` y `BG3CNT = 0x1F03` (prioridad 3), mientras `io_fin` tiene `DISPCNT = 0x9700` (`BG3` apagado) y `BG3CNT = 0x0000` (prioridad 0 en `io_fin`)
  Cuando se ejecuta `decodificar_lineas(&io_lineas)`, `decode_io_regs(&io_fin)`, `consolidar_dispcnt_fondos(&comps)` y `propagar_fondos_a_staging(&mut capas, &mut comps)`
  Entonces `estado.bg_prio[3]` es `3` y `comps[50].bg_prio3` es `3` (no `0` ni `0xFF`)
Test: `src/ppu_nativa_tests.rs::consolidar_fondos_recupera_prioridad_activa_cuando_io_fin_la_altera`
Estado previo: falla

Escenario: `consolidar_dispcnt_sprites_propaga_objwin_a_staging_filas_comp`
Caso de uso: Cuando hay sprites en modo `OBJWIN` (`gfx_mode == 2`), las filas de composición preparadas para GPU incluyen el bit `0x8000` en `staging_filas_comp` antes de la subida del búfer | Tipo: regresión
  Dado un `EstadoPpuNativa` con un sprite `OBJWIN` activo (`num_sprites_ventana = 1`) y `staging_filas_comp` con `dispcnt = 0x1F00` (sin el bit `0x8000`)
  Cuando se ejecuta `consolidar_dispcnt_sprites(&mut staging_filas_comp)`
  Entonces `estado.dispcnt & 0x8000` es `0x8000` y `staging_filas_comp[0].dispcnt & 0x8000` es `0x8000`
Test: `src/ppu_nativa_tests.rs::consolidar_dispcnt_sprites_propaga_objwin_a_staging_filas_comp`
Estado previo: falla

Escenario: Verificación visual en Android de la escena `"sacudió los cielos"` (visible)
Caso de uso: La capa de fuego naranja (`BG3`) sobre el dragón (`BG2`) se mantiene estable a 60 FPS mientras aparece el texto `"sacudió los cielos"` | Tipo: normal
  Dado el APK compilado e instalado en el dispositivo Android
  Cuando se reproduce la cinemática de introducción en la escena del dragón y el fuego
  Entonces la textura de fuego naranja no parpadea ni desaparece en ningún fotograma
Test: (visible)
Estado previo: falla

### 9. Cómo lo comprobaré

- `rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar` → herramienta o comando: `cargo test --lib ppu_nativa::tests::rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar` via subagente `Test Runner`
  → pasa si: el test compila y termina en `ok` comprobando que `rt_interp_one` no ejecuta la instrucción en `0x000001B4` mientras `bus.halted` o `irq_pending` están activos → no detecta: diferencias visuales en el dispositivo físico Android.
- `consolidar_fondos_recupera_prioridad_activa_cuando_io_fin_la_altera` → herramienta o comando: `cargo test --lib ppu_nativa::tests::consolidar_fondos_recupera_prioridad_activa_cuando_io_fin_la_altera` via subagente `Test Runner`
  → pasa si: el test termina en `ok` con `estado.bg_prio[3] == 3` y `comps[50].bg_prio3 == 3` → no detecta: fallos de renderizado en el shader WGSL.
- `consolidar_dispcnt_sprites_propaga_objwin_a_staging_filas_comp` → herramienta o comando: `cargo test --lib ppu_nativa::tests::consolidar_dispcnt_sprites_propaga_objwin_a_staging_filas_comp` via subagente `Test Runner`
  → pasa si: el test termina en `ok` con `staging_filas_comp[0].dispcnt & 0x8000 == 0x8000` → no detecta: errores fuera de la consolidación de `DISPCNT`.
- Verificación visual en Android de la escena `"sacudió los cielos"` (visible) → revisión humana: reproducir la escena del dragón en el dispositivo Android y confirmar que la capa de fuego naranja no desaparece mientras avanza el texto `"sacudió los cielos"`
  → no detecta: regresiones en otros capítulos no visitados durante la prueba.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Tras la auditoría del `Proposal Reviewer`, se descartó mover la captura de `io_lineas[l]` a `VPhase::LineEnd` en `mem.rs` (que habría desplazado 1 línea los efectos respecto a `ppu::render_scanline`) y se incorporó en Pasos, Alcance y Criterios de aceptación la corrección de `rt_interp_one` y `rt_tick` en `../gba-recomp/crates/gba-core/src/capi.rs`.

### Cambios tras prototipos (solo si el plan cambió)

- Ninguno.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- `rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar`: coincide (`Estado previo: falla` → `ok` tras sincronizar `rt_tick` y `rt_interp_one` en `capi.rs`). Método usado: el anunciado (`cargo test --lib ppu_nativa::tests::rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar` vía `Test Runner`).
- `consolidar_fondos_recupera_prioridad_activa_cuando_io_fin_la_altera`: coincide (`Estado previo: falla` → `ok` tras recuperar la prioridad activa desde `comps` en `consolidar_dispcnt_fondos`). Método usado: el anunciado (`cargo test --lib ppu_nativa::tests::consolidar_fondos_recupera_prioridad_activa_cuando_io_fin_la_altera` vía `Test Runner`).
- `consolidar_dispcnt_sprites_propaga_objwin_a_staging_filas_comp`: coincide (`Estado previo: falla` → `ok`, más 60/60 tests de `ppu_nativa::tests` en verde). Método usado: el anunciado (`cargo test --lib ppu_nativa::tests::consolidar_dispcnt_sprites_propaga_objwin_a_staging_filas_comp` vía `Test Runner`).
- Verificación visual en Android de la escena `"sacudió los cielos"` (visible): no pudo comprobarse en entorno local sin el dispositivo físico (pasa a revisión humana).

### Contexto: ¿se respetó?

- Restricción 1 (`Mantener la composición directa en GPU con NativePpu y la captura de io_lineas alineada con el oráculo de software en VPhase::Hblank`): respetada (no se modificó `mem.rs` y `rt_interp_one_respeta_halt_e_irq_pendiente_antes_de_ejecutar` pasa en `ok`).
- Restricción 2 (`No regenerar los 144.875 bloques C11 de src-c/`): respetada (ningún archivo en `src-c/` fue modificado).
- Restricción 3 (`Preservar la visibilidad del fuego a 60 FPS en el dispositivo Android real`): pendiente de revisión humana en dispositivo.
- No reapareció ninguna alternativa descartada ni surgieron imprevistos.

### Archivos modificados

- `../gba-recomp/crates/gba-core/src/capi.rs`
- `src/ppu_nativa.rs`
- `src/ppu_nativa_tests.rs`
- `examples/ppu_nativa_atribuye.rs`
- Coinciden exactamente con los anunciados en la sección Alcance (§7).

### Discrepancias

- Discrepancia 1:
  - Abierta por: persona usuaria (2026-10-30).
  - Punto afectado: Criterio visible / Escenario `Verificación visual en Android de la escena "sacudió los cielos"`.
  - Motivo: La textura naranja de fuego (`BG3`) sigue parpadeando y desapareciendo intermitentemente en el dispositivo Android real a pesar de la sincronización de `capi.rs` y `src/ppu_nativa.rs`.
  - Clasificación: Imprevisto (el parpadeo persiste en el dispositivo físico, lo que indica que la causa raíz restante involucra la composición en GPU, la alimentación de texturas o el canal de renderizado en Android y no solo la temporización de Halt/IRQ).
  - Enlace a la corrección: pendiente de nueva investigación diagnóstica y propuesta.

### Para la próxima vez

- Al diagnosticar parpadeos que ocurren únicamente en ejecución AOT (`[REC] 60 FPS - NAT`) y no en secuencias del intérprete, aislar si la causa es la composición por GPU (shaders / texturas de capas) o la lógica de CPU antes de proponer cambios de temporización.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno.

---

## Revisión humana

- Verificación visual en Android de la escena `"sacudió los cielos"` (visible): ✘ [No, no permaneció estable, siguen los parpadeos]
- Restricción no testeable (`Preservar la visibilidad del fuego a 60 FPS en el dispositivo Android real`): ✘
- Detectado solo por el humano: Parpadeo persistente en el dispositivo Android real.
