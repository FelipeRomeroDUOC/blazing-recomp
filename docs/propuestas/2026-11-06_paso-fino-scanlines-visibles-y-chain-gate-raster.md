# Propuesta: Paso Fino en Scanlines Visibles con Raster IRQ y Corte en `rt_chain_gate`

Estado: revisada por humano
- **ID:** `2026-11-06_paso-fino-scanlines-visibles-y-chain-gate-raster`
- **Fecha:** `2026-11-06`
- **Nivel:** `2`
- **Estado:** `confirmada`
- **Viene de:** `2026-11-06_cobertura-completa-hblank-y-cruce-scanlines-aot`
- **Hereda sin cambios:** `R1`, `R2`, `C1`, `C3`, la visibilidad de `pub vline_start` en `gba-core` y el resguardo `irq_video_esperando && m.cpu.flag(FLAG_I)` en `src/recomp.rs`.

---

## 1. Para decidir

### Qué cambia
1. En `src/recomp.rs` (`run_frame_native`), se elimina el hueco `80..860` de `ciclo_linea`: cuando `DISPSTAT` tiene activadas interrupciones de raster (`HBlank` o `VCount`, `dispstat & 0x30 != 0`), todas las scanlines visibles (`0..159`) y la última scanline de VBlank (`227`, previa a la línea `0`) se ejecutan con paso fino (`m.step()`).
2. En `../gba-recomp/crates/gba-core/src/capi.rs` (`rt_chain_gate`), se añade la condición de corte `((mach.bus.io[0x04] & 0x30) != 0 && (mach.bus.scanline() < 160 || mach.bus.scanline() >= 227))` para que cualquier grafo C11 (`g_080ba7a5`) iniciado durante VBlank (`scanline 160..226`) ceda inmediatamente el control a Rust si desborda hacia las scanlines visibles.

### Para qué
Cerrar los dos caminos por los que el megagrafo C11 de descompresión (`g_080ba7a5` en `0x080BD80E` / `0x080BD748`) seguía atravesando HBlank por dentro de C11 justo antes de los relámpagos:
- cuando `run_frame_native` lo despachaba en el ciclo `80` de una línea visible, o
- cuando arrancaba durante VBlank (`scanline >= 160`) y desbordaba más allá de la scanline `227` hacia las líneas visibles `0..159` sin que `rt_chain_gate` lo detuviera.

### Ejemplo
- **Antes:** En el ciclo `80` de una línea visible (o desbordando desde la línea `226` de VBlank), `f(&RT_API, mptr)` entraba en `g_080ba7a5` y corría cientos de ciclos en C11 cruzando el ciclo `1006` (`HBLANK_FLAG_CYCLE`) sin regresar a Rust, retrasando la ISR de HBlank y produciendo los 2 parpadeos previos al relámpago.
- **Ahora:** En VBlank (`160..226`) los bloques C11 corren a máxima velocidad nativa; al llegar a la línea `227` o a cualquier línea visible `0..159` con `dispstat & 0x30 != 0`, `rt_chain_gate` devuelve `1` y `run_frame_native` usa `m.step()` (que salta en 1 paso con `skip_to_next_event` cuando la CPU está `halted` y ejecuta ciclo a ciclo con paridad 100% de intérprete cuando la CPU está activa).

### Pasos en lenguaje de uso
1. El usuario inicia `ppu_doble` en PC y observa la escena del dragón hasta los relámpagos.
2. Durante toda la escena del dragón y la transición previa a los relámpagos, las 160 líneas visibles mantienen sincronía ciclo a ciclo idéntica al intérprete en ambos paneles.

### Qué podría romperse
Nada en escenas sin HBlank/VCount IRQ (`dispstat & 0x30 == 0`), y en escenas con HBlank/VCount IRQ el VBlank (`160..226`) sigue corriendo en AOT C11 mientras las líneas en `halted` avanzan con `skip_to_next_event`.

### Qué no se toca
No se modifican shaders WGSL, `src/ppu_nativa.rs` ni los archivos C11 generados en `src-c/`.

---

## 2. Detalle para verificar

### Suposiciones
- **S1:** En esos dos parpadeos previos al relámpago, `DISPSTAT` mantiene activo HBlank o VCount IRQ (`dispstat & 0x30 != 0`), y el desfase provenía de bloques C11 despachados en el hueco `80..860` o desbordados desde VBlank hacia las scanlines visibles.

### Reglas que se respetan
- **R1:** Heredada de `2026-11-06_cobertura-completa-hblank-y-cruce-scanlines-aot`.
- **R2:** Heredada de `2026-11-06_cobertura-completa-hblank-y-cruce-scanlines-aot`.

### Comprobaciones
- **C1 (Heredada):** Suite `cargo test recomp` en verde.
- **C2 (Extrema — modificada):** Dado que `dispstat & 0x30 != 0` en una línea visible (`scanline < 160`) o en el cierre de VBlank (`scanline == 227`) en cualquier `ciclo_linea` (incluido `ciclo_linea = 400`), cuando se ejecuta `run_frame_native`, entonces selecciona `m.step()` sin abrir huecos AOT en mitad de la línea.
- **C3 (Heredada):** Compilación de `ppu_doble` con `--features proto` sin errores.

### Archivos
- `../gba-recomp/crates/gba-core/src/capi.rs`
- `src/recomp.rs`
- `src/recomp_tests.rs`

---

## 3. Revisión humana

| Criterio | Estado |
| :--- | :--- |
| Tests automáticos en verde | ✔ |
| Verificación visual en `ppu_doble` (0 parpadeos antes del relámpago) | ✔ |
