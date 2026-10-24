# Color de pixeles afines frente al fondo (nivel 2)

Fecha: 2026-10-19
Estado: verificada por agente

## Objetivo
Confirmar si los 124 pixeles que escriben los afines en intro.txt, cuadro 7500, tienen el mismo color que el fondo o el sprite que tienen debajo. Si lo tienen, no se ven y la imagen final es igual con o sin afines.

## Contexto
- Framebuffers completa, base y emulador son identicos byte a byte (volcados/wt-fb).
- Completa escribe 124 pixeles de sprite mas que la base (44+39+24+5+12).

## Hipotesis
- H310: los 124 pixeles tienen el mismo color que la base en su posicion. Entonces los afines no cambian la imagen.
- H311: algunos pixeles difieren de la base pero el resultado final los iguala por otra regla (por ejemplo, efectos de color). Entonces hay que revisar BLDCNT/BLDALPHA.

## Metodo (solo en la copia gba-recomp-wt)
1. En afin_replay, para la variante completa, guardar la posicion (x, y) y el color del sprite afin de cada pixel escrito por un afin (los que aporta cada afin en obj_line).
2. Para esas posiciones, guardar el color de la variante base y el color final de completa.
3. Contar: pixeles cuyo color afin es igual al de base; pixeles distintos; y cuantos de los distintos cambian el framebuffer final.
4. Volcar una muestra de 10 posiciones con sus colores.

## Criterios
- Si todos los 124 pixeles afines tienen el mismo color que la base, H310 queda confirmada.
- Si hay pixeles distintos y el framebuffer final igual, H311 queda abierta: revisar efectos de color.
- Si hay pixeles distintos y el framebuffer final cambia, la comparacion anterior era incorrecta.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no toca el shim.
- No dice si el resultado visual es correcto; eso requiere comparacion con referencia.

## Resultado
(pendiente)

## Resultado (2026-10-19)
- Instrumentacion: diag_pix registra cada pixel escrito por render_sprites (copia gba-recomp-wt). Volcado de la variante completa en volcados/wt-pix/pix-completa.txt.
- Entradas totales (todos los sprites): 2322. Entradas de afines: 124 (44+39+24+5+12).
- Para los 124 pixeles de afin: color del sprite igual al framebuffer de la base en la misma posicion: 124 de 124. Distintos: 0.
- Igual al framebuffer final de la completa: 124 de 124.
- H310 CONFIRMADA: los pixeles de los afines tienen exactamente el color que ya aparece en esa posicion sin afines. Por eso no cambian la imagen.
- H311 descartada para esta escena: no hay pixeles distintos que el resultado final iguale.
- Limite: esto no dice si el color correcto del afin es ese. Podria ser que el afin dibuje el mismo color del fondo por casualidad de paleta o de escena. Falta comparar con referencia o con otra escena donde el efecto sea visible.
- Estado: pendiente de revision humana.
