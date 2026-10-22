# Fondos y modo de video en las líneas sin afines (cuadro 5300)

Fecha: 2026-10-19
Nivel: 2
Estado: verificada por agente

## Motivo

H350 (mezcla alfa) quedó descartada en `2026-10-19_mezcla-encendida-control-shim-5300.md`: las 2879 diferencias en las 12 líneas sin afines siguen igual con la mezcla encendida o apagada. Queda H352 (fondos y modo de video), sin resolver.

## Pregunta

¿Qué fondos (BG0 a BG3), modo de video y prioridades usa cada una de las 12 líneas sin afines en el cuadro 5300, y en cuáles de ellas están las diferencias shim–PPU?

## Método

1. Copia de solo lectura de `examples/shim_afin_mezcla.rs` (sin cambiar `gba-core`, el shim ni `gba-recomp`).
2. Para cada línea capturada, volcar: DISPCNT (modo y bits de fondos), BGnCNT (prioridad y tamaño), y las diferencias shim–PPU por línea.
3. Comprobar que el control de reconstrucción sigue dando 0 diferencias frente al emulador. Si no, parar.
4. Guardar en `volcados/fondos-sinafin-5300.txt`.

## Criterios

- Si las 12 líneas usan el mismo modo y fondos que las líneas con diferencias, la causa está en el tratamiento de fondos o de prioridad del shim.
- Si las diferencias aparecen solo en líneas con un fondo o prioridad concreta, esa es la pista.
- Si el control no da 0, no se saca conclusión.

## Alcance excluido

- No se modifica el shim, `gba-core` ni `gba-recomp`.
- No se hacen commits.
- No se implementa la transformación afín.

## Resultado

- Ejecución: job job_c520a7973acbe4b8, ejemplo examples/shim_afin_fondos.rs (solo lectura). Exit 0. Salida: volcados/fondos-sinafin-5300.txt y volcados/fondos-log.txt.
- Las 160 líneas tienen el mismo DISPCNT (0x1F00: modo 0, BG0 a BG3 y OBJ activos) y los mismos BGnCNT (BG0 0x2C08, BG1 0x0D01, BG2 0x0E03, BG3 0x0F0A).
- Las 12 líneas sin afines tienen 239 o 240 de 240 píxeles distintos entre shim y PPU. Casi la línea entera.
- Conclusión: **H352 descartada como causa por modo o fondos**. Las 12 líneas usan la misma configuración de fondos que el resto del cuadro, así que el modo o los fondos no explican qué líneas difieren.
- Pista nueva: la diferencia casi total en esas líneas sugiere algo que cambia la línea completa (por ejemplo, desplazamientos de fondo BGnHOFS/BGnVOFS en 0x10 a 0x1F, o ventanas). No se han volcado todavía. Requiere nueva propuesta.
- gba-recomp no se ha modificado.
