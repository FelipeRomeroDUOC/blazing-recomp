# Revisar la comparacion de shim_afin_pixeles (nivel 2)

Fecha: 2026-10-18
Estado: verificada por agente

## Objetivo
Explicar por que shim_afin_pixeles dio 0 pixeles por afin, mientras que los contadores directos de la copia (afin_etapas y afin_aislar) dan 44/39/24/5/12.

## Contexto
- Tres mediciones con la misma escena (intro.txt, cuadro 7500) y el mismo working tree:
  - afin_etapas (copia HEAD y working tree): escrito_en_linea 44/39/24/5/12.
  - afin_aislar (working tree, run_frame completo): 44/39/24/5/12 con cualquier aislamiento.
  - shim_afin_pixeles (replay por linea): 0 pixeles por afin.
- El control de shim_afin_pixeles (variante completa = emulador) da 0 diferencias, asi que la variante completa si se renderiza bien.
- El replay sustituye palette, vram, oam, aff_ref, aff_dirty e io por linea, pero no se sabe si render_scanline usa tambien copia_linea_oam o mascara_oam (mem.rs), que el replay no actualiza.

## Hipotesis
- H280: el replay no refleja la OAM de cada variante porque render_scanline usa caches de OAM (copia_linea_oam, mascara_oam) que no se actualizan. Entonces las variantes parciales no se distinguen y el 0 es un artefacto del metodo.
- H281: el 0 viene de la comparacion (en_ppu compara con la linea base de forma que pixeles iguales se pierden). Entonces el metodo de diferencias subcuenta.
- H282: el 0 es real y los afines solo aportan con la OAM completa. Entonces hay que explicar la dependencia.

## Metodo (solo lectura y medicion en la copia gba-recomp-wt)
1. Leer en mem.rs y ppu.rs como se usan copia_linea_oam y mascara_oam en render_scanline.
2. En la copia, medir la variante base y la variante con un solo afin en el replay, e imprimir cuantos pixeles cambian y cuantos sprites lee render_sprites en cada variante.
3. Comparar esos numeros con afin_aislar.

## Criterios
- Si el replay lee la misma cantidad de sprites que la ejecucion directa, H280 queda descartada.
- Si el replay ignora la OAM variante, H280 queda confirmada.
- Si el conteo de cambios es distinto del conteo directo, H281 queda confirmada.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no toca el shim.
- No dice si los pixeles son correctos.

## Resultado
(pendiente)

## Resultado (2026-10-18)
- Lectura de codigo: mem.rs copia_linea_oam y mascara_oam solo alimentan perf_counters. render_sprites lee mem.oam directamente. H280 DESCARTADA.
- Ejemplo nuevo afin_replay en la copia gba-recomp-wt (features diag-afin,perf-video). Escena intro.txt, cuadro 7500.
- Control replay completa frente a emulador: 0 diferencias.
- Escrito por afin 0-4 (replay):
  completa: 44/39/24/5/12
  solo k (otros prohibidos): 44/0/0/0/0 | 0/51/0/0/0 | 0/0/42/0/0 | 0/0/0/5/0 | 0/0/0/0/12
  base (todos prohibidos): 0/0/0/0/0
- Los conteos del replay coinciden con la ejecucion directa (afin_aislar). H280 DESCARTADA; el replay si dibuja los afines en el buffer de sprites.
- Pixeles del framebuffer final distintos de la base: 0 en TODAS las variantes, incluida completa.
- Consecuencia: los afines escriben en el buffer de sprites, pero ningun pixel llega al framebuffer final. Ni la PPU ni el emulador cambian por la presencia de los afines en este cuadro. H281 (metodo de diferencias) queda descartada como explicacion: el metodo es correcto y el resultado es real.
- Esto apoya H242 (los pixeles no tapados no se pintan). La causa probable esta en la composicion final (compose / prioridad / mezcla con fondos), no en render_sprites.
- Limite: falta leer la funcion de composicion y comprobar por que el ObjPixel no sale en el framebuffer.
- Estado: pendiente de revision humana.
