# Composicion de pixeles de sprites afines (nivel 2)

Fecha: 2026-10-19
Estado: verificada por agente

## Objetivo
Encontrar por que los pixeles que los afines escriben en el buffer de sprites nunca llegan al framebuffer final (0 pixeles distintos de la linea base en la escena intro.txt, cuadro 7500).

## Contexto
- afin_replay: escrito por afin 44/39/24/5/12 con la OAM completa, y 0 pixeles del framebuffer final distintos de la base.
- Los conteos coinciden con la ejecucion directa, asi que render_sprites si dibuja.
- afin-tapado-7500 contaba 0 pixeles tapados por fondo; esa cuenta puede estar mal.

## Hipotesis
- H290: compose descarta los pixeles de sprite por la regla de prioridad o por semi/ventana, sin importar el origen afin.
- H291: los fondos tapan los pixeles afines, y la cuenta de tapados (afin-tapado) subestima el efecto.
- H292: el framebuffer final se escribe antes de componer los sprites de esa linea, o desde otro buffer.

## Metodo (solo en la copia gba-recomp-wt)
1. Leer la funcion de composicion de ppu.rs (solo lectura) y anotar en que orden se mezclan sprites y fondos, y que regla decide cada pixel.
2. Instrumentar compose en la copia con contadores por causa: pixel de sprite afin descartado por prioridad, por fondo, por semi o por ventana, y pixel escrito en el framebuffer.
3. Repetir con la escena intro.txt, cuadro 7500, y comparar con afin_replay.

## Criterios
- Si los contadores de compose muestran descartes por prioridad o fondo, H290 o H291 quedan confirmadas, con el porcentaje por causa.
- Si compose no recibe los pixeles de sprite (contador de entrada en 0), H292 queda confirmada.
- Si los pixeles entran y salen en el framebuffer, habra que revisar el replay o la comparacion.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash, no toca el shim.
- No dice si el resultado visual es correcto; eso requiere comparacion visual o con referencia.

## Resultado
(pendiente)

## Resultado (2026-10-19)
- Lectura de ppu.rs (copia gba-recomp-wt): render_scanline llama a render_sprites y luego a compose (compose_straight / componer_tramo). Los sprites pasan por obj_line y la composicion no los descarta por diseno en el camino directo.
- Instrumentacion en la copia: DIAG_COMP cuenta, despues de compose, los pixeles de obj_line con prioridad valida y si el framebuffer tiene el color del sprite. Escena intro.txt, cuadro 7500, replay.
- presentes / igual / distinto: completa [2322, 2322, 0]; base [2198, 2198, 0]; solo 0 [2242, 2242, 0]; solo 1 [2249, 2249, 0]; solo 2 [2240, 2240, 0]; solo 3 [2203, 2203, 0]; solo 4 [2210, 2210, 0].
- Diferencia completa frente a base: 124 pixeles presentes = 44+39+24+5+12 (coincide con escrito_en_linea). Todos llegan al framebuffer con su color.
- CONTRADICCION: afin_replay sigue dando 0 pixeles distintos de la base en todas las variantes. Esto contradice la instrumentacion. La causa mas probable esta en la comparacion de afin_replay (copia del framebuffer por linea o calculo de diferencias), no en la composicion.
- H290 y H291: no hay evidencia de descarte en composicion (distinto = 0). H292: los pixeles si llegan al framebuffer, asi que tampoco.
- Estado: pendiente de revision humana. Antes de sacar conclusiones hay que verificar la comparacion de afin_replay.
