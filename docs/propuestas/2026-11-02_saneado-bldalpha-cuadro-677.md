# Propuesta: comprobar si el saneado de BLDALPHA causa el color claro en el cuadro 677

Fecha: 2026-11-02
Nivel: 2
Estado: descartada. El saneado no se aplica en la rama `Some(io_lineas)` que usa el cuadro 677; ver `2026-11-02_gpu-bldalpha-crudo-cuadro-677.md`.

## Contexto

- En el cuadro 677, las líneas 70–100 tienen `DISPCNT=0x1F00`, `BLDCNT=0x3F68` y `BLDALPHA=0x0610` (EVA=16, EVB=6). Medido con `examples/ppu_bldalpha_linea.rs`.
- Esa combinación cumple la firma de `sanear_bldcnt_cinematica_dragon` (`src/ppu_nativa.rs:1168`). La función cambia `BLDALPHA` a `0x0810` (EVB=8) antes de subirlo a la GPU.
- La GPU difiere de la referencia en 37 435 de 38 400 píxeles del cuadro 677. En el píxel 70/15 la referencia da `[31,42,B5]` y la GPU `[42,52,CE]`.
- La CPU (`gba_core::ppu::render_scanline`) con `BLDALPHA=0x0610` da la referencia en ese píxel (`0x5906`).

## Hipótesis

El saneado sube EVB de 6 a 8. Con más peso de la capa de abajo, la mezcla sale más clara. Eso explicaría la diferencia de `+0x10` por canal.

## Cambio propuesto

1. En `examples/ppu_bldalpha_linea.rs`, para las líneas 70–100 del cuadro 677, dibujar con la CPU la línea capturada con dos valores de `BLDALPHA`: `0x0610` (tal cual) y `0x0810` (saneado). Contar los píxeles que difieren de la referencia del juego en cada caso.
2. Repetir el conteo con la GPU. Esto requiere forzar `BLDALPHA=0x0610` en la ruta de la GPU, lo que hoy el saneado impide.
3. Si la GPU con `0x0610` iguala la referencia, la causa es el saneado. Si no, la causa está en otro punto de la GPU.

## Punto que requiere decisión

El paso 2 no se puede hacer sin tocar `src/`. La opción más limitada es un interruptor de diagnóstico (por ejemplo, una variable de entorno) que desactive el saneado solo en ese ejemplo. No se propone aún: pido aprobación para esa edición aparte si se quiere seguir por ahí.

Si el paso 1 ya muestra que `0x0810` da más píxeles distintos que `0x0610` frente a la referencia en la CPU, la hipótesis queda reforzada sin tocar `src/`.

## Verificación prevista

- `cargo run --release --features proto --example ppu_bldalpha_linea -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/intro.txt 677 70 100`
- Comparar el conteo de píxeles distintos con cada `BLDALPHA` frente a `volcados/banda-677-limpio.log`.

## Riesgos

- Solo cambia un ejemplo de diagnóstico. No se modifican `Cargo.toml`, `src/` ni el saneado.
- Quitar el saneado puede afectar a otras líneas de la cinemática del dragón, por eso no se propone como arreglo todavía.
- No se crea ningún binario `.gba` ni `.bin`.

## Decisión pendiente

Aprobar el paso 1 (solo ejemplo). El paso 2 requiere una propuesta aparte si implica tocar `src/`.
