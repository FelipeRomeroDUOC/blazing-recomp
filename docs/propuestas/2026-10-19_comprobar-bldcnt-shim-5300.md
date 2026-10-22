# Comprobar que el shim lee BLDCNT = 0 en el cuadro 5300

Fecha: 2026-10-19
Nivel: 2
Estado: verificada por agente

## Motivo

La prueba sin mezcla (`2026-10-19_sin-mezcla-shim-5300.md`) dio salida idéntica byte a byte a la original. No sabemos si la escritura de `io[0x50..0x51] = 0` llega a la ruta de render del shim. Si no llega, la prueba no dice nada sobre la mezcla alfa.

## Pregunta

¿Qué valor de BLDCNT (`io[0x50..0x51]`) lee el shim justo antes de renderizar el cuadro 5300, en la prueba sin mezcla?

## Método

1. Copia de `examples/shim_afin_sinmezcla.rs` (solo lectura; no se cambia el shim).
2. En el cuadro objetivo 5300, imprimir `m.bus.io[0x50]` y `m.bus.io[0x51]` antes y después de la escritura a 0.
3. Confirmar en el código del shim de qué arreglo de E/S lee la mezcla (`io` o una copia) y si la lectura ocurre después de la escritura.

## Criterios

- Si el shim lee 0 en el cuadro 5300: la prueba sin mezcla fue válida y H350 queda descartada.
- Si el shim no lee 0: la prueba sin mezcla no es válida; hay que corregir la escritura antes de sacar conclusiones.

## Alcance excluido

- No se modifica el shim ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Ejecución: EXIT 0 (job job_a2a12742dd8ea6a3), ejemplo copia de solo lectura examples/shim_afin_bldcnt.rs.
- Salida: volcados/bldcnt-capturas-5300.txt.
- Las 160 líneas capturadas leen BLDCNT = 0x2261 (mezcla activa) en el cuadro 5300.
- Causa: la escritura BLDCNT = 0 se hace antes de run_frame, pero el juego vuelve a escribir BLDCNT durante el cuadro. El shim lee cap.io de cada línea, así que nunca vio el 0.
- Conclusión: la prueba sin mezcla de 2026-10-19_sin-mezcla-shim-5300.md NO fue válida. Su resultado H361 queda anulado. H350 sigue sin probar.
- Para probarla bien hace falta poner cap.io[0x50..0x51] = 0 en el bucle de replay, no antes de run_frame. Eso requiere nueva propuesta y aprobación.
- gba-recomp no se ha modificado.
