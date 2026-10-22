# Propuesta: Medir la aportación de cada capa al cuadro 1000 usando la PPU por software y las capturas por línea

Fecha: 2026-10-17
Nivel: 2
Estado: aprobada (el usuario aprobó el alcance el 2026-10-17)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Con las capturas por línea del cuadro 1000, se redibuja cada línea con la PPU por software en varias variantes de DISPCNT: sin cada capa, y con cada capa sola. Con eso se mide cuántos píxeles aporta cada capa (BG0, BG1, BG2, BG3, OBJ) y cuántos cubre cuando está sola. No se usa el shim en este paso.

Esta propuesta reemplaza el paso 3 de `comparar-capas` (copias de un solo cuadro con DISPCNT modificado), que ya no sirve porque el estado cambia entre líneas.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_captura-por-linea-cuadro-1000.md`, resultado: 160 capturas del cuadro 1000; el replay línea a línea da 0 píxeles distintos. La referencia por línea es exacta.
  - `docs/propuestas/2026-10-17_buscar-estado-faltante-cuadro-1000.md`, resultado: una copia del estado final da 22.185 píxeles distintos; por eso no se usa.
  - `gba-recomp/crates/gba-core/src/ppu.rs`, `render_scanline`: el modo de vídeo está en los bits 0 a 2 de DISPCNT; BG0 a BG3 están en los bits 8 a 11 (`0x100 << bg`); OBJ está en el bit 12 (`0x1000`). Cada línea recarga `aff_ref` desde la captura, así que cada variante es independiente de las demás.
  - `src/ppu_shim.rs`, `decode_io_regs`: lee los registros una vez por cuadro. Por eso no se compara el shim en esta propuesta.
- Intención: saber qué capas son responsables de los píxeles del cuadro 1000, para decidir qué corregir primero en el shim. Éxito: un informe con una fila por capa.
- Definiciones:
  - Aportación de una capa: píxeles de la imagen completa que cambian al apagar esa capa.
  - Cobertura de una capa: píxeles donde la capa sola se dibuja distinta del fondo (la capa con todo apagado).
- Restricciones y decisiones previas:
  - La PPU por software no cambia (respuesta humana, 2026-10-16).
  - El experimento solo lee capturas y usa la PPU por software. No cambia `gba-core` ni el shim.
  - El trabajo corre en el PC con Windows (respuesta humana, 2026-10-16).
- Alternativas descartadas:
  - Comparar el shim capa por capa en este paso → descartada: el shim no usa estado por línea, y cambiarlo es otra propuesta (opción B).
  - Usar el resultado de la composición para saber qué capa "gana" cada píxel → descartada: el renderizador no informa de la capa ganadora, y medirla exige cambiar `ppu.rs`.
- Ante imprevistos, prioriza: la variante completa debe coincidir con el juego, sobre tener el desglose.

### 3. Ejemplos

- Caso normal: el cuadro 1000 tiene BG0 y OBJ activos. Quitar BG0 cambia, por ejemplo, 5.000 píxeles → aportación de BG0 = 5.000. Con solo BG0, ese fondo cubre, por ejemplo, 9.000 píxeles → cobertura de BG0 = 9.000.
- Caso límite: una capa apagada en DISPCNT del cuadro (bit 0) → su aportación debe ser 0, porque quitarla no cambia nada.

### 4. Pasos

1. Crear `examples/ppu_capas_linea.rs` (con `required-features = ["proto"]`):
   - corre el juego hasta el cuadro 1000 con la captura activa, igual que `ppu_linea`;
   - por cada línea y cada variante, reinicia `aff_ref` y `aff_dirty` desde la captura, pone el DISPCNT de la variante y redibuja la línea;
   - variantes por línea: completa; sin BG0, sin BG1, sin BG2, sin BG3, sin OBJ; solo BG0, solo BG1, solo BG2, solo BG3, solo OBJ; nada (todo apagado, solo fondo);
   - calcula aportación y cobertura por capa, y compara la variante completa con el framebuffer del juego;
   - escribe `volcados/capas-cuadro-1000.txt`: una línea de resumen y cinco filas con prefijo `capa `.
2. Añadir la entrada `[[example]]` en `Cargo.toml`.
3. Crear `tests/ppu_capas_linea_tests.rs` con las pruebas de la sección 8.
4. Ejecutar el ejemplo y las pruebas, y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_capas_linea.rs` (nuevo), `Cargo.toml` (una entrada), `tests/ppu_capas_linea_tests.rs` (nuevo).
- Archivos que no cambian: `gba-core` (ni `mem.rs` ni `ppu.rs`), `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, el juego y la APK.
- Limitación: los efectos de color (BLDCNT, alpha y brillo) mezclan dos capas. Al apagar una capa, su efecto también cambia el resultado. La aportación incluye ese efecto, así que no es una medida pura de la capa.
- Limitación: si un píxel de una capa tiene el mismo color que el fondo, no cuenta en la cobertura.

### 6. Suposiciones

- Redibujar una línea con DISPCNT cambiado solo afecta a las capas indicadas → prototipar: sí, la variante "nada" debe dar solo color de fondo → si es falsa: hay que revisar qué más lee DISPCNT → resultado pendiente.
- La variante completa coincide con el juego → prototipar: sí, en el paso 4 → si es falsa: el experimento no es válido y se para → resultado pendiente.

### 7. Alcance

- Incluye: variantes por línea con la PPU por software; aportación y cobertura por capa; informe; pruebas.
- No incluye: el shim (opción B); cambios en `gba-core`; corrección de diferencias; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la variante completa coincide con el juego en las 160 líneas
Caso de uso: confiar en que las variantes parten de la misma escena | Tipo: regresión
  Dado el cuadro 1000 con las 160 capturas
  Cuando se redibuja cada línea con todas las capas activas
  Entonces el resumen de `volcados/capas-cuadro-1000.txt` indica 0 píxeles distintos del juego
Test: `tests/ppu_capas_linea_tests.rs::test_variante_completa_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe tiene una fila por capa
Caso de uso: ver qué capas aportan al cuadro 1000 | Tipo: normal
  Dado el cuadro 1000
  Cuando se abre `volcados/capas-cuadro-1000.txt`
  Entonces tiene 5 filas con prefijo `capa ` (BG0, BG1, BG2, BG3, OBJ), y cada aportación y cobertura está entre 0 y 38.400
Test: `tests/ppu_capas_linea_tests.rs::test_informe_capas_tiene_cinco_filas` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: una capa apagada no aporta píxeles
Caso de uso: comprobar que el experimento respeta DISPCNT | Tipo: límite
  Dado el cuadro 1000
  Cuando se lee la fila de una capa cuyo bit está apagado en DISPCNT
  Entonces su aportación es 0
Test: `tests/ppu_capas_linea_tests.rs::test_capa_apagada_no_aporta` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example ppu_capas_linea -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000 > ..\ppu-capas-linea-output.txt 2>&1` → pasa si: el resumen dice 0 píxeles distintos para la variante completa → no detecta: diferencias de color que solo se notan en la TV.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test ppu_capas_linea_tests` → pasa si: las tres pruebas pasan o se omiten con el aviso.
- Revisión visual: abrir `volcados\capas-cuadro-1000.txt` y comprobar que la capa con mayor aportación coincide con lo que se ve en la imagen completa → no detecta: efectos de color que cambian el resultado de forma no lineal.
- Regresión: `git diff --stat` → pasa si: no aparecen `gba-core`, `ppu_shim.rs` ni `ppu_shim.wgsl` → no detecta: cambios no versionados.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación, el informe por capa del cuadro 1000 está escrito, y el Resultado de la propuesta dice qué capas aportan más píxeles y cuáles no aportan ninguno.

### 11. Parar si

- La variante completa no coincide con el juego: parar, porque el experimento no es válido.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.
- Para medir la aportación hace falta cambiar `ppu.rs` o el shim: parar y pedir aprobación.

### Correcciones previas a presentar

- En `2026-10-17_comparar-capas-del-cuadro-fijo.md`, el paso 3 supone que una copia de un solo cuadro basta para aislar capas. Eso queda descartado por la propuesta de captura por línea. Esta propuesta reemplaza ese paso.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno todavía.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_capas_linea`, cuadro 1000, secuencia `secuencias\intro.txt`.

- Control: la variante completa coincide con el juego: 0 píxeles distintos de 38.400. Modo de vídeo 0 en las 160 líneas.
- Pruebas `tests/ppu_capas_linea_tests.rs`: 3 pasan (`test_variante_completa_coincide`, `test_informe_capas_tiene_cinco_filas`, `test_capa_apagada_no_aporta`).

| Capa | Activa en líneas | Aportación (píxeles) | Cobertura sola (píxeles) |
|---|---|---|---|
| BG0 | 160 | 0 | 0 |
| BG1 | 160 | 0 | 0 |
| BG2 | 160 | 35.503 | 37.920 |
| BG3 | 160 | 0 | 0 |
| OBJ | 160 | 6.000 | 710 |

Lectura:
- BG2 es la capa que sostiene la imagen del cuadro 1000: quitarla cambia unos 35.500 de los 38.400 píxeles.
- OBJ aporta 6.000 píxeles pero, sola, cubre solo 710. Eso quiere decir que OBJ cambia píxeles de otras capas, probablemente por efectos de color o ventanas. Es el límite que la propuesta anotó, y aquí se ve. No se puede leer como "los sprites dibujan 6.000 píxeles".
- BG0, BG1 y BG3 están activos pero no aportan ningún píxel: solos no dibujan nada visible. Hipótesis sin verificar: tiles vacíos o ventanas que los ocultan.

Para el shim, en este cuadro, el primer foco es BG2 (texto, modo 0), y luego la interacción de OBJ con los efectos. Queda por verificar: por qué OBJ aporta más que su cobertura (`BLDCNT`, `BLDALPHA`, ventanas) y si BG0, BG1 y BG3 están enmascarados.

Archivos: `volcados\capas-cuadro-1000.txt` (informe), `..\ppu-capas-linea-output.txt` y `..\ppu-capas-linea-tests-output.txt` (salidas; vienen en UTF-16 por la redirección de PowerShell).

Cambios de código: `examples/ppu_capas_linea.rs` (nuevo), `Cargo.toml` (una entrada `[[example]]`), `tests/ppu_capas_linea_tests.rs` (nuevo). `gba-core`, `ppu.rs`, el shim y el juego no cambiaron.

---

## Revisión humana

- Que el informe identifique qué capas aportan al cuadro 1000, para decidir qué corregir primero en el shim: ✔ | ✘ | ?
