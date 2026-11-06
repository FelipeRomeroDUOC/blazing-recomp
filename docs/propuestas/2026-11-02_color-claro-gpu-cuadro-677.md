# Propuesta: diagnosticar por qué la GPU sale más clara en el cuadro 677 de la intro

Fecha: 2026-11-02
Nivel: 2
Estado: pendiente

## Contexto

- La mezcla BG3 sobre BG2 se revirtió: no cambiaba el resultado de la GPU (37 435 de 38 400 píxeles distintos, con y sin ella). El shader está igual que `HEAD`.
- En el cuadro 677 la GPU (`src/ppu_nativa.wgsl`) difiere de la referencia en 37 435 de 38 400 píxeles, con colores unos `+0x10` por canal más claros. Ejemplo: píxel 70/15, referencia `[31,42,B5]`, GPU `[42,52,CE]`.
- La CPU (`gba_core::ppu::render_scanline`) da la referencia en ese píxel (`0x5906`), así que la diferencia está en la ruta GPU.
- La fórmula `alpha_blend` del shader (líneas 121–126) es la misma que la de Rust (`src/ppu_nativa.rs:195`). Por tanto el fallo no está en la aritmética de mezcla.
- Registros del cuadro 677: `DISPCNT=0x1F00`, `BLDCNT=0x3F68`, `BLDALPHA=0x0610` (EVA=16, EVB=6).

## Hipótesis

Los valores de `BLDCNT`/`BLDALPHA` que llegan al shader por línea (`io_lineas`, `src/ppu_nativa.rs:1149`) no son los que ve la CPU en esas líneas, o el sanado `sanear_bldcnt_cinematica_dragon` (`src/ppu_nativa.rs:1168`) los altera. Eso explicaría que la GPU mezcle con otro peso o con otras capas.

## Cambio propuesto

1. Nuevo ejemplo de diagnóstico `examples/ppu_subcamino.rs`, que para el cuadro 677 imprima por línea (rango 70–100) `BLDCNT` y `BLDALPHA` de `cap.io[0x50..0x54]`, y el valor de sanado si difiere.
2. Comparar esos valores con los que usa la GPU en las líneas 76–95, donde difieren.
3. Si difieren, probar en CPU qué combinación reproduce el valor de la GPU; si no difieren, buscar el paso de la GPU que altera el color (ver `split6`/`join6` en el shader).

No se modifica `src/`, `Cargo.toml`, `ppu_nativa.rs`, ni `sanear_bldcnt_cinematica_dragon`. No se crea ningún binario `.gba`/`.bin`.

## Verificación prevista

- `cargo run --release --features proto --example ppu_subcamino -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/intro.txt 677`
- Comparar la salida por línea con `volcados/banda-677-limpio.log`.

## Riesgos

- Solo afecta a un ejemplo de diagnóstico; no altera la salida del emulador ni los tests.
- Si la causa está en la conversión de color de la GPU, el arreglo puede requerir un cambio en `src/`; eso necesitaría una propuesta nueva.

## Decisión pendiente

Aprobar la ejecución del diagnóstico (paso 1–3) sin tocar `src/`.