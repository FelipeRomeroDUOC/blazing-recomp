# Control con mezcla encendida y comparación shim–PPU (cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: verificada por agente

## Motivo

En `2026-10-19_sin-mezcla-corregida-shim-5300.md` el control falló (29591 diferencias) porque quitamos la mezcla también en la reconstrucción, y el emulador sí la tiene. El 0 de líneas sin afines compara dos versiones sin mezcla, así que no dice si el juego se ve bien.

## Preguntas

1. Con la mezcla encendida en la reconstrucción, ¿coincide con el emulador (0 diferencias)?
2. Con la mezcla encendida en ambos lados (shim y PPU), ¿cuántas diferencias quedan en las 12 líneas sin afines?

## Método

1. Copia de `examples/shim_afin_sinmezcla2.rs` (solo lectura del shim; no se cambia `gba-core` ni `gba-recomp`).
2. Quitar la puesta a cero de `cap.io[0x50..0x51]` en el replay: la reconstrucción usa `cap.io` tal cual.
3. Ejecutar el control. Si da 0 frente al emulador, seguir. Si no, parar.
4. Comparar shim–PPU con la mezcla encendida y anotar diferencias totales y en líneas sin afines.
5. Guardar el informe en `volcados/afin-mezcla-5300.txt` (el control debe pasar antes de escribirlo).

## Criterios

- Control 0: la reconstrucción es fiable y la comparación es válida.
- Si líneas sin afines con mezcla encendida son 2879 (igual que antes): la mezcla no es la causa; H350 descartada.
- Si bajan de forma clara: el shim y la PPU tratan la mezcla distinto; H350 confirmada en ese punto.
- Si el control no da 0: no se saca conclusión de esta prueba.

## Alcance excluido

- No se modifica el shim, `gba-core` ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Ejecución: job job_ab3162c95673abe4, ejemplo examples/shim_afin_mezcla.rs (mezcla encendida en ambos lados). Exit 0. Salida: volcados/afin-mezcla-5300.txt.
- Control (reconstrucción con mezcla frente al emulador): 0 diferencias (esperado 0). Control válido.
- Shim frente a PPU con mezcla encendida: diferencias totales 33173 (igual que antes); líneas con afines 30294; líneas sin afines 2879 (igual que antes).
- Conclusión: **H350 descartada**. La mezcla alfa no explica las 2879 diferencias en líneas sin afines. Las 2879 diferencias aparecen igual con la mezcla encendida en ambos lados que en la prueba sin mezcla (que no fue válida).
- H352 (fondos y modo de video) sigue sin resolver.
- gba-recomp no se ha modificado.
