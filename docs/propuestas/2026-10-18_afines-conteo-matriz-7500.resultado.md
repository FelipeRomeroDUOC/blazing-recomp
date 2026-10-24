Estado: verificada por agente

## Resultado (pendiente de volcar al documento principal: la herramienta de escritura devolvió error)

Ejecutado con la aprobación: corrección de `examples/shim_afin_mide.rs` y nueva medición de 7500. Sin cambios en `src/`.

- Corrección: el parámetro pa de cada grupo está en 0x06 + 0x20·grupo (pb +0x08, pc +0x10, pd +0x18). La propuesta decía 0x08·grupo; era un error mío.
- Control frente al framebuffer del emulador: 0 diferencias.
- Diferencias shim frente a PPU V0: 0 en las 160 líneas.
- Conteo corregido: 64 líneas con afines (filas 24 a 87), 96 sin afines.
- Afines: 5 (índices 0 a 4), todos 32x64 con doble tamaño, x entre 157 y 210, y = 24, visibles en 64 líneas. Matrices no identidad: pa de 0x009D a 0x0080, pd de 0x02AA a 0x2000, pb = pc = 0.
- H190 descartada; H191 confirmada; H192 descartada; H180 no confirmada; H181 cumplida pero trivial; H182 descartada.

Lectura: hay 5 afines con matriz no identidad en pantalla y el shim coincide al píxel con la PPU. Eso no prueba que se dibujen bien: la PPU de referencia podría tampoco aplicar la matriz. Hay que comprobar cómo trata gba-core los afines en `gba-core::ppu` y en `examples/ppu_afin_y_linea.rs` (con su diagnóstico previo `docs/propuestas/2026-10-17_diagnostico-afin-y-por-linea.md`, aún no leído).

Próximo paso propuesto (sin aprobar): leer esos archivos y comprobar si la PPU aplica pa/pb/pc/pd. Requiere propuesta y aprobación.
