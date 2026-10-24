# Medir working tree de gba-core (nivel 2)

Fecha: 2026-10-18
Estado: verificada por agente

## Objetivo
Comprobar si la contradiccion entre H210 y la medicion de HEAD se debe a la diferencia entre el working tree de gba-recomp y HEAD, o a la configuracion de aislamiento anterior (oam_con).

## Contexto
- Medicion en HEAD c9f2847 (volcados/gba-recomp-limpio): afines 0-4 de 7500 escriben pixeles (44/39/24/5/12 escritos en linea).
- Medicion anterior en working tree: 0 pixeles aportados por los 5 afines.
- gba-recomp NO se modifica. Los hashes de ppu.rs y Cargo.toml estan en volcados/despues-ppu-hash.txt.

## Hipotesis
- H260: el working tree tambien escribe pixeles de los 5 afines. Entonces H210 queda descartada y el fallo estaba en el aislamiento (oam_con).
- H261: el working tree no escribe pixeles. Entonces la diferencia HEAD frente a working tree es la causa, y hay que revisar que cambio en ppu.rs sin commit.

## Metodo
1. Crear volcados/gba-recomp-wt/ como copia de volcados/gba-recomp-limpio.
2. Sustituir su crates/gba-core/src/ppu.rs por el ppu.rs del working tree de gba-recomp.
3. Reaplicar solo los contadores de diagnostico (feature diag-afin), sin cambiar la logica de render.
4. Ejecutar el mismo ejemplo afin_etapas sobre la escena 7500, con los mismos contadores.
5. Comparar con la tabla de HEAD.

## Criterios
- Control: 0 diferencias diagnostico apagado frente a encendido.
- Si el working tree coincide con HEAD en escritos_en_linea, H260 no aplica a la diferencia de version.

## Lo que NO hace
- No modifica gba-recomp, no hace commits ni stash.
- No implementa nada en el shim.

## Resultado
(pendiente)

## Resultado (2026-10-18)
- Escena: secuencias/intro.txt, cuadro 7500 (los mismos numeros que HEAD, lo que confirma la escena). escena2 no llega al cuadro 7500; escena3 da 0 en todos los contadores (escena distinta, no usada).
- Copia: volcados/gba-recomp-wt = working tree completo de gba-core (src y Cargo.toml) mas solo los contadores diag-afin.
- Control: 0 diferencias diagnostico apagado frente a encendido.
- Afines 0-4 (intro, 7500): texel_no_transparente 44/51/42/5/12; escrito_en_linea 44/39/24/5/12; descartado_prioridad 0/12/18/0/0. IDENTICO a HEAD.
- H260: CONFIRMADA. El working tree tambien escribe pixeles de los 5 afines. La diferencia HEAD frente a working tree no explica la medicion anterior.
- H261: DESCARTADA.
- Consecuencia: la medicion de H210 (0 pixeles) es sospechosa. Probablemente el metodo de aislamiento (oam_con) altera el resultado. Hay que revisar shim_afin_pixeles antes de dar H210 por buena o mala.
- Limite: los contadores prueban que se escriben pixeles, no que sean correctos. Falta comparar visualmente o con referencia.
- Estado: pendiente de revision humana.
