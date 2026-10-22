# Verificar la comparacion de afin_replay (nivel 2)

Fecha: 2026-10-19
Estado: verificada por agente

## Objetivo
Comprobar si afin_replay calcula bien las diferencias entre variantes. Hoy da 0 pixeles distintos de la base, pero la instrumentacion de compose muestra 124 pixeles de sprite extra en la variante completa que llegan al framebuffer.

## Contexto
- DIAG_COMP (copia gba-recomp-wt, tras compose): completa tiene 2322 pixeles de sprite presentes, base 2198; la diferencia es 124 = 44+39+24+5+12. Todos con el color del sprite.
- afin_replay: pixeles distintos de base = 0 para todas las variantes.
- Es contradictorio. La sospecha es la copia del framebuffer por linea o el calculo de diferencias en afin_replay.

## Hipotesis
- H300: afin_replay copia mal el framebuffer (por ejemplo, fb se rellena con filas de otra variante o con datos stale). Entonces el 0 es un artefacto del ejemplo.
- H301: la comparacion es correcta y los 124 pixeles coinciden en color con la base. Entonces la contradiccion viene de DIAG_COMP (que cuenta obj_line, no el color final de la base).
- H302: el framebuffer del replay no cambia entre variantes porque render_scanline escribe en otro buffer (por ejemplo, el de la linea anterior).

## Metodo (solo en la copia gba-recomp-wt)
1. Revisar afin_replay.rs: como se copia replay.bus.framebuffer en fb y como se calcula dif.
2. Volcar a un archivo binario el framebuffer de la variante completa y de la base (240x160 u16), y contar diferencias con un script simple fuera del emulador.
3. Contar, en la base, cuantos de los 124 pixeles tienen el color del sprite afin.
4. Comparar con los 124 de DIAG_COMP.

## Criterios
- Si el volcado da diferencias, H300 queda confirmada: el ejemplo tenia un error y hay que repetir la medicion de la composicion.
- Si el volcado da 0 diferencias y los 124 pixeles tienen el mismo color en la base, H301 queda confirmada: los afines no cambian la imagen en esta escena.
- Si el volcado da 0 y los pixeles de la base son distintos, H302 o un fallo de copia quedan abiertos.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no toca el shim.
- No dice si el resultado visual es correcto.

## Resultado
(pendiente)

## Resultado (2026-10-19)
- Revision de afin_replay.rs: la copia de framebuffer por linea y el calculo de diferencias son correctos. H300 DESCARTADA como error de copia.
- Volcado binario de 240x160 u16 (volcados/wt-fb): fb-completa.bin, fb-base.bin, fb-emu.bin.
- Bytes distintos: completa-base 0; completa-emu 0; base-emu 0. Los tres framebuffers son identicos byte a byte.
- Por tanto, en la escena intro.txt cuadro 7500, los cinco afines no cambian la imagen final, aunque escriben 124 pixeles en el buffer de sprites.
- Explicacion mas probable (H301 refinada): los pixeles de los afines tienen el mismo color que el fondo o el sprite que tienen debajo, asi que no se ven. No esta confirmado.
- Para confirmarlo: en la base, comparar el color del sprite afin con el color final en las 124 posiciones.
- Estado: pendiente de revision humana.
