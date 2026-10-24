# ¿El shim aplica un solo scroll por cuadro? (cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: verificada por agente

## Motivo

En `2026-10-19_desplazamiento-fondos-lineas-sin-afines-5300.md` BG1 VOFS cambia línea a línea en todo el cuadro, y las 12 líneas sin afines son casi completas en diferencias. Al leer el código del shim (solo lectura) se ve:

- `src/ppu_shim.rs` (aprox. líneas 387 a 419) decodifica HOFS y VOFS una vez por capa y los guarda como `scroll_x` y `scroll_y`.
- `src/ppu_shim.wgsl` (aprox. líneas 207 a 209) usa ese scroll como un solo uniform para toda la pantalla.

Si esto es así, el shim no aplica el scroll línea a línea, mientras la PPU de `gba-core` sí lo aplica por línea (`ppu_capa_ganadora.rs` lín. 131 a 133). Es una hipótesis a comprobar, no un hecho.

## Preguntas

1. ¿El shim usa un solo valor de VOFS de BG1 en el cuadro 5300, aunque el juego cambie el valor en cada línea?
2. Si el shim usara el valor por línea, ¿bajan las diferencias shim–PPU en las 12 líneas sin afines?

## Método

Paso A (solo lectura):
1. Copia de `examples/shim_afin_desplaz.rs` que registre `scroll_y` de BG1 que recibe el shim en el cuadro 5300 (sin cambiar `src/ppu_shim.rs`).
2. Control: reconstrucción frente al emulador debe dar 0.

Paso B (solo si el paso A confirma un valor único, y con nueva aprobación):
1. Copia del shim de prueba (no el real) que aplique el VOFS por línea.
2. Medir diferencias shim–PPU en las 12 líneas sin afines y en el total.

## Criterios

- Paso A: si hay un único valor de BG1 VOFS usado en todas las líneas, la hipótesis se sostiene.
- Paso B: si las diferencias en las 12 líneas bajan más del 90 %, el scroll por línea es la causa principal.
- Si el control no da 0, no se saca conclusión.

## Alcance excluido

- No se modifica `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, `gba-core` ni `gba-recomp` en esta propuesta. Un cambio al shim real requeriría otra decisión.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Paso A: job job_969692c18c807650, ejemplo examples/shim_afin_scroll.rs (solo lectura). Exit 0. Salida: volcados/scroll-shim-5300.txt. Control: 0 diferencias (válido).
- Valor que recibe el shim (io_fin, el registro al final del cuadro, que es lo que pasa a renderiza): BG1 VOFS = 0x000.
- Valores por línea que escribe el juego: varían de 0x000 a 0x1FF (por ejemplo línea 1 = 0x1FF, línea 98 = 0x1CF, línea 99 = 0x1CE).
- Conclusión: **confirmada**. El shim aplica un único desplazamiento vertical de BG1 por cuadro (0x000), mientras el juego cambia el valor en cada línea. El código lo muestra: src/ppu_shim.rs decodifica io_fin una vez, y src/ppu_shim.wgsl usa ese valor para toda la pantalla.
- Diferencias shim–PPU: 33173 (igual), líneas sin afines 2879 (igual). Esto es lo esperable: el paso A no cambia el shim.
- Paso B (prueba en copia del shim con desplazamiento por línea): pendiente de nueva aprobación.
- gba-recomp y src/ppu_shim.* no se han modificado.
