# Glosario de Blazing Recomp

Vocabulario propio del port. Cada término lleva la fuente que lo define, porque la
palabra sola no basta para saber qué se está midiendo.

## Telemetría

Las mediciones que el motor escribe en `blazing-perf.txt` mientras se ejecuta el juego,
para diagnosticar el rendimiento sin depurador y sin parar la partida. Es un artefacto de
texto plano, pensado para abrirse a mano en el dispositivo y para leerse con
`scripts/diagnostico/analizar_telemetria.py`.

## Línea de segundo

Una línea `t=<n>s ...` de `blazing-perf.txt`: el resumen de la ventana de **un segundo**
de ejecución (FPS, tiempo de CPU, PPU, APU, dibujo y swapchain). No es una línea por
fotograma: cada fotograma se acumula y la ventana se cierra al cumplirse el segundo. Su
contrato lo declara [`telemetria/esquema.txt`](telemetria/esquema.txt).

## Esquema de telemetría

El fichero [`telemetria/esquema.txt`](telemetria/esquema.txt): la lista de campos de la
línea de segundo, con su tipo, su unidad y la versión del contrato que los introdujo. Es el
**dueño único** de esa lista. Lo leen los dos lados del contrato —el escritor
([`src/telemetria.rs`](src/telemetria.rs), que lo embebe con `include_str!`) y el lector
([`scripts/diagnostico/analizar_telemetria.py`](scripts/diagnostico/analizar_telemetria.py))—
así que añadir un campo es tocar un fichero, no dos sitios que tienen que coincidir.

## Sello de versión

El campo `formato=<n>` con el que termina cada línea de segundo: dice con qué versión del
contrato se escribió. Sin él, «una línea sin los campos nuevos» y «una línea a la que le
faltan campos» son indistinguibles. Una línea sin sello se lee en modo tolerante (los
campos que falten no se echan de menos); una línea sellada se comprueba contra la lista de
campos de **su** versión.

## Anomalía de formato

Lo que el analizador reporta cuando una línea no encaja con el esquema: un campo que no
está declarado, un valor que no encaja con su tipo, un campo que falta en una línea sellada
o una versión que el esquema no conoce. No se descarta la línea: se anota el motivo en la
sección `ANOMALÍAS DE FORMATO` del informe y la línea sigue contando para las medias.
Antes de esto, un desajuste hacía que el regex no casara y el segundo **desaparecía del
informe sin ningún aviso**.

## Camino de composición

Uno de los recorridos con los que la PPU compone una línea de pantalla, repartidos por la
línea `# ppucamino` de la telemetría: `rectas` (vía rápida, con los registros constantes en
el tramo), `mezcla` (efecto de color que necesita las dos capas), `ventana` (ventana de
sprites activa) y `semi` (sprite translúcido). Los cuatro suman las líneas dibujadas y cada
línea cae en uno solo: el primero que impide usar la vía rápida.
