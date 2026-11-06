# Propuesta: medir la GPU en el cuadro 677 con el BLDALPHA crudo (paso 2)

Fecha: 2026-11-02
Nivel: 2
Estado: pendiente
- Sustituye al paso 2 de `2026-11-02_saneado-bldalpha-cuadro-677.md`.

## Contexto

- En el cuadro 677, las líneas 70–100 tienen `DISPCNT=0x1F00`, `BLDCNT=0x3F68`, `BLDALPHA=0x0610`.
- Con la CPU, `BLDALPHA=0x0610` reproduce el framebuffer del juego (0 píxeles distintos en 70–100). Medido con `examples/ppu_bldalpha_linea.rs`.
- Con `BLDALPHA=0x0810` (el valor que pone el saneado), la CPU cambia 7 439 píxeles en esa banda.
- Corrección a la propuesta anterior: `sanear_bldcnt_cinematica_dragon` solo se llama en la rama `None` de `preparar_staging_filas` (`src/ppu_nativa.rs:1211`). El cuadro 677 llega por la rama `Some(io_lineas)` (`src/ppu_nativa.rs:2054`), donde el saneado no se ejecuta. Por tanto, el saneado no explica por sí solo el color de la GPU en 677.
- La GPU difiere de la referencia en 37 435 de 38 400 píxeles del cuadro 677 (`volcados/banda-677-limpio.log`). Ejemplo: píxel 70/15, referencia `[31,42,B5]`, GPU `[42,52,CE]`.

## Hipótesis

La GPU no mezcla con el `BLDALPHA` crudo que llega en `io_lineas`, o lo mezcla con otra capa. La causa está en la decodificación `decodificar_lineas` (`src/ppu_nativa.rs:1136`) o en el shader, no en el saneado.

## Cambio propuesto

1. Nuevo ejemplo o ampliación de `examples/ppu_bldalpha_linea.rs`: volcar desde la GPU la composición de las líneas 70–100 del cuadro 677 y contar píxeles distintos frente a la referencia del juego (`fb_juego`).
2. Comparar la GPU con la CPU por línea. Si la CPU con `0x0610` iguala el juego y la GPU no, la diferencia es de la GPU.
3. Revisar los valores que recibe la GPU por línea (`staging_filas_comp`, `ConfigCompGpu.bldalpha`) para confirmar que son `0x0610`.

No se modifica `src/`, `Cargo.toml` ni el saneado. No se crea ningún binario `.gba` ni `.bin`.

## Verificación prevista

- `cargo run --release --features proto --example ppu_bldalpha_linea -- "roms/1575 - Fire Emblem (E)(Independent).gba" secuencias/intro.txt 677 70 100`
- Comparar con `volcados/banda-677-limpio.log`.

## Riesgos

- Solo cambia un ejemplo de diagnóstico. No altera la salida del emulador ni los tests.
- Si la causa está en el shader o en la decodificación, el arreglo necesitará una propuesta aparte que toque `src/`.

## Decisión pendiente

Aprobar el paso 1 (solo ejemplo, sin `src/`).
