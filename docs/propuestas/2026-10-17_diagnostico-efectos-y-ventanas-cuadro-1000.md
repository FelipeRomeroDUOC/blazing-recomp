# Propuesta: Diagnosticar si los efectos de color y las ventanas explican las capas OBJ, BG0, BG1 y BG3 del cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Con las capturas por línea del cuadro 1000 y la PPU por software, se miden dos cosas que `capas-por-aportacion` dejó sin explicar:

- Si la aportación de OBJ (6.000 píxeles, frente a una cobertura de 710) baja al apagar los efectos de color (`BLDCNT`).
- Si BG0, BG1 y BG3, que no dibujan nada, dibujan algo cuando se apagan las ventanas (bits 13 a 15 de DISPCNT).

No se cambia el shim, `gba-core` ni el juego.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_capas-por-aportacion-cuadro-1000.md`, resultado: OBJ aporta 6.000 píxeles y cubre 710 solo. BG0, BG1 y BG3 tienen aportación y cobertura 0. La causa de OBJ y de BG0, BG1 y BG3 no está verificada.
  - `gba-recomp/crates/gba-core/src/ppu.rs`, `render_scanline` y `compose`: los efectos de color y las ventanas se aplican en la composición. La parte de `compose` que trata la semitransparencia de OBJ no se ha leído todavía (paso 1).
  - Registros: DISPCNT en 0x00 (bits 13 a 15 son WIN0, WIN1 y OBJWIN); BLDCNT en 0x50; BLDALPHA en 0x52; BLDY en 0x54.
  - `src/ppu_shim.rs`: no implementa `BLDCNT`, `BLDALPHA` ni `BLDY` (según la revisión anterior de este proyecto). Si los efectos cambian píxeles en este cuadro, son una causa concreta de los glitches del shim.
- Intención: saber qué parte de la aportación de OBJ viene de los efectos, y si las ventanas ocultan BG0, BG1 y BG3. Éxito: un informe que dé cada número por separado.
- Hipótesis:
  - H4: los efectos de color explican la diferencia de OBJ. Se confirma si, sin efectos, la aportación de OBJ baja a unos 710.
  - H5: las ventanas ocultan BG0, BG1 y BG3. Se confirma si, sin ventanas, esas capas cubren píxeles.
  - H6: BG0, BG1 y BG3 no tienen píxeles visibles (tiles vacíos o solo color transparente). Se confirma si, sin ventanas, siguen sin cubrir nada.
- Restricciones y decisiones previas:
  - La PPU por software no cambia (respuesta humana, 2026-10-16).
  - El experimento solo lee capturas y usa la PPU por software. No cambia `gba-core` ni el shim.
  - El trabajo corre en el PC con Windows (respuesta humana, 2026-10-16).
- Alternativas descartadas:
  - Medir los efectos en el shim → descartada: el shim no tiene efectos, así que no sirve de referencia.
  - Leer la VRAM para buscar tiles vacíos → descartada en este paso: la cobertura sin ventanas ya responde H6 con menos trabajo.

### 3. Ejemplos

- Caso normal: BLDCNT apagado → aportación de OBJ pasa de 6.000 a 710 → H4 confirmada: los efectos explican la diferencia.
- Caso límite: ninguna línea del cuadro tiene ventanas activas → la variante sin ventanas es igual a la completa → la diferencia de ventanas es 0 y el informe lo dice.

### 4. Pasos

1. Leído (2026-10-17): `compose` aplica la semitransparencia de un sprite según los segundos objetivos de `BLDCNT` (bits 8 a 13), no según el modo (bits 6 y 7). Por eso `BLDCNT` = 0 apaga las dos cosas a la vez. Para separarlas hace falta una variante que apague solo el modo y deje los segundos objetivos (V1 y V2 abajo).
2. Crear `examples/ppu_efectos_ventanas.rs` (con `required-features = ["proto"]`). Reutiliza la captura de `ppu_capas_linea`. Por cada línea, reinicia `aff_ref` y `aff_dirty`, y redibuja estas variantes:
   - V0: completa (control; debe coincidir con el juego);
   - V1: completa, con el modo de efecto apagado (bits 6 y 7 de `BLDCNT` a 0; los segundos objetivos se mantienen);
   - V2: sin OBJ, con el mismo modo apagado;
   - V3: completa, con `BLDCNT` = 0 (sin efectos ni semitransparencia de sprites);
   - V4: sin OBJ, con `BLDCNT` = 0;
   - V13: sin OBJ, con la configuración original (para volver a medir los 6.000 píxeles).
   - V5: completa, sin ventanas (bits 13 a 15 de DISPCNT a 0);
   - V6: sin BG0, sin ventanas; V7: sin BG1, sin ventanas; V8: sin BG3, sin ventanas;
   - V9: solo BG0, sin ventanas; V10: solo BG1, sin ventanas; V11: solo BG3, sin ventanas;
   - V12: nada, sin ventanas (fondo).
3. Calcular:
   - aportación de OBJ con efectos: V0 frente a (V0 sin OBJ) (ya medido: 6.000);
   - aportación de OBJ con el modo de efecto apagado: V1 frente a V2; con `BLDCNT` = 0: V3 frente a V4. La diferencia entre ambas da la parte de la semitransparencia;
   - para BG0, BG1 y BG3, aportación sin ventanas: V5 frente a V6, V7 y V8;
   - para BG0, BG1 y BG3, cobertura sin ventanas: V9, V10 y V11 frente a V12;
   - líneas con alguna ventana activa, y la diferencia entre V0 y V5.
4. Escribir `volcados/efectos-ventanas-cuadro-1000.txt` con una línea por medida.
5. Crear `tests/diagnostico_efectos_tests.rs`, con las pruebas de la sección 8.
6. Añadir la entrada `[[example]]` en `Cargo.toml`, ejecutar y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_efectos_ventanas.rs` (nuevo), `Cargo.toml` (una entrada), `tests/diagnostico_efectos_tests.rs` (nuevo).
- Archivos que no cambian: `gba-core`, `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego y la APK.
- Limitación resuelta en el paso 1: `BLDCNT` = 0 apaga la semitransparencia de sprites junto con el modo de efecto. V1 y V2 separan los dos efectos, así que la parte de la semitransparencia se obtiene por diferencia entre las dos mediciones.

### 6. Suposiciones

- Con `BLDCNT` = 0, la PPU por software no aplica efectos de color → prototipar: sí, con V1 → si es falsa: la aportación con efectos no se puede aislar y se para → resultado pendiente.
- Con los bits 13 a 15 de DISPCNT a 0, todas las capas se ven en toda la pantalla → prototipar: sí, con V3 y V4 → si es falsa: hay que revisar cómo `compose` usa las ventanas → resultado pendiente.

### 7. Alcance

- Incluye: medidas de efectos y ventanas con la PPU por software, informe y pruebas.
- No incluye: cambios en el shim, en `gba-core` o en la PPU; corrección de los glitches; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el informe da por separado la aportación de OBJ con y sin efectos
Caso de uso: saber si los efectos explican los 6.000 píxeles de OBJ | Tipo: normal
  Dado el cuadro 1000 con sus capturas por línea
  Cuando se corre el diagnóstico
  Entonces `volcados/efectos-ventanas-cuadro-1000.txt` tiene una línea `efectos OBJ con efectos` y otra `efectos OBJ sin efectos`, con sus números
Test: `tests/diagnostico_efectos_tests.rs::test_informe_separa_efectos` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da, para BG0, BG1 y BG3, la cobertura con y sin ventanas
Caso de uso: saber si las ventanas ocultan esas capas | Tipo: normal
  Dado el cuadro 1000
  Cuando se corre el diagnóstico
  Entonces el informe tiene tres líneas `ventanas BG0`, `ventanas BG1` y `ventanas BG3`, cada una con su cobertura con ventanas y sin ventanas
Test: `tests/diagnostico_efectos_tests.rs::test_informe_separa_ventanas` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: sin líneas con ventanas, quitar las ventanas no cambia la imagen
Caso de uso: comprobar que el experimento no inventa diferencias | Tipo: límite
  Dado el cuadro 1000
  Cuando el informe indica 0 líneas con ventanas activas
  Entonces la diferencia entre la variante completa y la variante sin ventanas es 0
Test: `tests/diagnostico_efectos_tests.rs::test_sin_ventanas_no_cambia_sin_lineas` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example ppu_efectos_ventanas -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000 > ..\ppu-efectos-ventanas-output.txt 2>&1` → pasa si: el informe tiene todas las líneas de la sección 8 → no detecta: diferencias de color que solo se ven en la TV.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test diagnostico_efectos_tests` → pasa si: las tres pruebas pasan o se omiten con el aviso.
- Control: V0 debe dar 0 píxeles de diferencia con el juego, como en `capas-por-aportacion`. Si no, se para.
- Regresión: `git diff --stat` → pasa si: no aparecen `gba-core`, `ppu_shim.rs` ni `ppu_shim.wgsl` → no detecta: cambios no versionados.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación, el paso 1 dice si `BLDCNT` = 0 afecta la semitransparencia de OBJ, y el Resultado dice si H4, H5 y H6 se confirman o se descartan.

### 11. Parar si

- V0 no coincide con el juego: parar, porque el experimento no es válido.
- El paso 1 muestra que la semitransparencia de OBJ no se puede separar de `BLDCNT`: parar, explicar el límite y proponer otra variante.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.
- Para completar el diagnóstico hace falta cambiar `ppu.rs` o el shim: parar y pedir aprobación.

### Correcciones previas a presentar

- En `2026-10-17_capas-por-aportacion-cuadro-1000.md`, el Resultado dice que OBJ "cambia píxeles de otras capas, probablemente por efectos de color o ventanas". Esta propuesta lo comprueba. Hasta entonces, ese "probablemente" no está verificado.

### Cambios tras prototipos (solo si el plan cambió)

2026-10-17, tras el paso 1: el plan añade cuatro variantes (V1 a V4) para separar el modo de efecto de la semitransparencia de sprites, y renumera V5 a V13. Los criterios de aceptación no cambian.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_efectos_ventanas`, cuadro 1000. Control: la variante completa coincide con el juego (0 píxeles de 38.400). Pruebas `tests/diagnostico_efectos_tests.rs`: 3 pasan.

| Medida | Píxeles distintos |
|---|---|
| OBJ: quitarlo, con efectos (original) | 6.000 |
| OBJ: quitarlo, con el modo de efecto apagado | 6.000 |
| OBJ: quitarlo, con `BLDCNT` = 0 | 6.240 |
| Modo de efecto apagado vs completa (todas las capas) | 0 |
| `BLDCNT` = 0 vs completa (todas las capas) | 3.823 |
| Sin ventanas vs completa | 0 |
| BG0, BG1, BG3: aportación sin ventanas | 0 cada una |
| BG0, BG1, BG3: cobertura sola sin ventanas | 0 cada una |

Líneas con ventanas activas: 0. Líneas con `BLDCNT` distinto de 0: 160.

Lectura:
- **H4 descartada.** El modo de efecto (aclarar, oscurecer, alpha) no cambia ningún píxel en este cuadro. Los 6.000 píxeles de OBJ no vienen de los efectos de color.
- **Semitransparencia descartada como causa de OBJ.** Con `BLDCNT` = 0 la aportación de OBJ sube a 6.240, no baja. Lo que sí cambia con `BLDCNT` = 0 son 3.823 píxeles en total, que vienen de la mezcla con segundos objetivos, no de OBJ por sí solo.
- **H5 descartada.** No hay ninguna línea con ventanas activas, y quitarlas no cambia la imagen.
- **H6 confirmada.** BG0, BG1 y BG3 no tienen ningún píxel visible en este cuadro, ni siquiera sin ventanas. Sus tiles están vacíos o son transparentes.

**Contradicción en OBJ (resuelta el 2026-10-17):** quitar OBJ cambia unos 6.000 píxeles, pero OBJ solo cubre 710 (medido en `capas-por-aportacion`). Medido con `examples/ppu_subcamino`, 5.290 de esos 6.000 son mezcla alpha de un sprite negro translúcido y los 710 restantes son la cobertura opaca; la cobertura no ve la mezcla porque OBJ sola sobre negro da el mismo negro que no tener OBJ. No es un bug del motor. Detalle en [`2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md`](2026-10-17_localizar-pixeles-obj-y-contenido-bg-cuadro-1000.md).

Archivos: `volcados\efectos-ventanas-cuadro-1000.txt` (informe), `..\ppu-efectos-ventanas-output.txt` y `..\ppu-efectos-ventanas-tests-output.txt` (salidas; vienen en UTF-16 por la redirección de PowerShell).

Cambios de código: `examples/ppu_efectos_ventanas.rs` (nuevo), `Cargo.toml` (una entrada `[[example]]`), `tests/diagnostico_efectos_tests.rs` (nuevo). `gba-core`, `ppu.rs`, el shim y el juego no cambiaron.

---

## Revisión humana

- Que el informe separe la aportación de OBJ con y sin efectos de color, y que BG0, BG1 y BG3 queden confirmadas o descartadas como ocultas por ventanas: ✔ | ✘ | ?
