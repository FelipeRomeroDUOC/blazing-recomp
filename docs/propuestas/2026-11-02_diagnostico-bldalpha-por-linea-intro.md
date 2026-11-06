# Propuesta: imprimir BLDCNT/BLDALPHA por línea en la intro (cuadro 677)

Fecha: 2026-11-02
Nivel: 2
Estado: aprobada

## Contexto

Tras la reversión del falso positivo de composición del dragón, la intro conserva una pequeña desincronización. El análisis de `volcados/diferencias/` (cuadros 677, 758 y 759) indica que la causa es `render_fin_de_cuadro` con disparo `linea`, y que el residuo desaparece al quitar BG2, BG3 o BLDCNT.

`examples/ppu_banda_nativa.rs` ejecutado sobre el cuadro 677 muestra solo los registros de fin de cuadro (`BLDCNT=0x3F68`, `BLDALPHA=0x0610`) y no los valores por línea. Así no se puede comprobar qué BLDALPHA usa la ruta por línea en las líneas 76–95, donde la nativa y la referencia difieren.

## Cambio propuesto

- `examples/ppu_subcamino.rs`: en el bucle sobre `capturas`, imprimir para cada línea del rango 70–100 su `BLDCNT` y `BLDALPHA` tomados de `cap.io[0x50..0x54]`.
- No se modifica `src/`, `Cargo.toml`, `ppu_nativa.rs` ni el saneado `sanear_bldcnt_cinematica_dragon`.

## Verificación prevista

- Ejecutar `cargo run --release --features proto --example ppu_subcamino -- roms/... secuencias/intro.txt 677`.
- Comparar los valores por línea con las líneas 76–95 del log de `ppu_banda_nativa`.

## Riesgos

- Solo afecta a un ejemplo de diagnóstico; no altera la salida del emulador ni los tests.
