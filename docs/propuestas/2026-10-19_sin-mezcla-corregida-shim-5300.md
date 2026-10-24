# Prueba sin mezcla corregida (cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: modificada

## Motivo

La prueba anterior (`2026-10-19_sin-mezcla-shim-5300.md`) no fue válida. Puso `BLDCNT = 0` antes de `run_frame`, pero el juego lo vuelve a escribir durante el cuadro. El shim lee `cap.io` de cada línea y ve `0x2261` en las 160 líneas (ver `2026-10-19_comprobar-bldcnt-shim-5300.md`).

## Pregunta

¿Cuántas diferencias shim–PPU quedan en las 12 líneas sin afines cuando la mezcla alfa está realmente apagada en el shim?

## Método

1. Copia de `examples/shim_afin_sinmezcla.rs` (solo lectura del shim, sin cambiar `gba-core` ni `gba-recomp`).
2. Quitar la escritura antes de `run_frame`.
3. En el bucle de replay, antes de `render_scanline`, poner `cap.io[0x50] = 0` y `cap.io[0x51] = 0` en cada línea capturada (la copia `replay.bus.io`).
4. Comprobar con una lectura previa que el valor leído es 0 en las 160 líneas.
5. Comparar el número de diferencias en las líneas sin afines con los 2879 originales.

## Criterios

- Si la reducción es mayor del 90 %: H350 confirmada (la mezcla alfa es la causa).
- Si es menor del 10 %: H350 descartada.
- Entre 10 % y 90 %: resultado parcial; hay causas mezcladas.

## Control

El control de reconstrucción (replay sin cambios frente al framebuffer del emulador) debe seguir dando 0 diferencias. Si no da 0, la prueba se descarta.

## Alcance excluido

- No se modifica el shim ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Ejecución: job job_7c752e426a85a073, ejemplo examples/shim_afin_sinmezcla2.rs. Sale con exit 1: el control de reconstrucción no pasó y el programa no escribió el informe.
- Comprobación previa: las 160 líneas leídas antes de poner el cero tienen BLDCNT = 0x2261 (volcados/bldcnt-previo-zero-5300.txt).
- Control: 29591 diferencias entre replay y emulador (esperado 0). Causa: el control se hizo con el replay sin mezcla, y el emulador sí tiene la mezcla. El diseño del control estaba mal; el criterio de parada funcionó.
- Dato del log (shim frente a PPU, ambas sin mezcla): diferencias totales 4390 (antes 33173); líneas con afines 4390 (antes 30294); líneas sin afines 0 (antes 2879).
- Lectura cautelosa: el 0 en líneas sin afines compara shim y PPU con la mezcla apagada en los dos. Muestra que la diferencia de 2879 viene de cómo el shim y la PPU tratan la mezcla. No prueba que el juego se vea bien sin mezcla.
- Conclusión provisional: H350 apoyada (la mezcla explica las diferencias en líneas sin afines), pero no confirmada. Falta una prueba con control válido.
- Próximo paso propuesto: control con la mezcla encendida en el replay (debe dar 0 frente al emulador) y comparación shim–PPU con la mezcla encendida, además de la versión sin mezcla. Requiere nueva propuesta.
- gba-recomp no se ha modificado.
