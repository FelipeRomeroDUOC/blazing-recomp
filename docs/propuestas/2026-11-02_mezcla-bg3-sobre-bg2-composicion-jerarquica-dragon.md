# Propuesta: mezclar BG3 sobre BG2 en la composición del dragón

Fecha: 2026-11-02
Nivel: 2
Estado: pendiente

## Contexto

El diagnóstico del cuadro 677 (línea 70, x=15) muestra que la referencia del hardware sale de mezclar BG3 sobre BG2:

- Solo BG2 → `0x624F` (`[7B,94,C5]`, valor de la ruta nativa del log `banda-677.log`).
- Solo BG3 → `0x3420`.
- Mezcla alfa BG3 sobre BG2 con EVA=16, EVB=6 → `0x5906` (`[31,42,B5]`), igual a la referencia.

Con BLDCNT=0x3F68, BG3 es primer objetivo (bit 3) y BG2 es segundo objetivo (bit 10). Por tanto el hardware sí mezcla BG3 sobre BG2 en este píxel.

La intro la compone la GPU con el bloque `es_escena_dragon` de `src/ppu_nativa.wgsl` (líneas ~547–570). Ahí:

1. BG3 se mezcla con el backdrop usando `alpha_blend(c3, backdrop, eva_fuego, evb_fuego)`, con `evb_fuego` fijo a 8 si `evb == 0`.
2. BG2 **reemplaza** el color (`color_acc = c2`), sin mezcla alfa.

Así que el dragón tapa el cielo de fuego en vez de mezclarse con él. Además, el saneado `sanear_bldcnt_cinematica_dragon` fuerza `0x0810` (EVB=8), y en el píxel medido ese valor empeora el color (`0x6548` frente a `0x5906`).

La función CPU `compone_pixel_jerarquica_dragon` (`src/ppu_nativa.rs`, ~línea 514) tiene la misma lógica, pero solo se usa en tests.

## Cambio propuesto

En el paso 2 del bloque `es_escena_dragon` de `src/ppu_nativa.wgsl`, y en su espejo CPU `compone_pixel_jerarquica_dragon`, cuando BG2 es opaco y BG3 está activo como primer objetivo con BG2 como segundo objetivo (bit 3 y bit 10 de `bldcnt`, modo 1, efectos activos), aplicar `alpha_blend(c2, color_acc, eva, evb)` con los coeficientes reales de `BLDALPHA` en lugar de `c2` sin mezcla. El resto de pasos (BG1, BG0, OBJ) no cambia.

No se modifica `sanear_bldcnt_cinematica_dragon`: el valor forzado 0x0810 deja de ser necesario si la mezcla usa los coeficientes reales.

## Verificación prevista

- `cargo run --release --features proto --example ppu_pixel_capa -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/intro.txt 677 70 15`: debe dar `0x5906`.
- Comparar `volcados/diferencias/` de 677, 758 y 759 antes y después.
- Tests en `src/ppu_nativa_tests.rs` (~1420–1530) que esperan 0x0810: revisar si siguen siendo válidos y no debilitar asserts.
- `cargo test --release --features proto --lib ppu_nativa`.

## Riesgos

- Cambia la salida del dragón en toda la cinemática, no solo en la intro. Hay que comprobar otras escenas.
- Los tests del saneado 0x0810 pueden fallar y habrá que decidir si se reescriben (requiere aprobación explícita).
- El píxel 677/70/15 es una sola muestra; 758 y 759 aún no están comprobados con esta mezcla.
- El cambio no se ha probado en la GPU; la cadena WGSL y la CPU deben quedar alineadas.
