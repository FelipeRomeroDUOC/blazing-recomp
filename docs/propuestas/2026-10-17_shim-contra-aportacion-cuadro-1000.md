# Propuesta: Comparar el shim GPU con los 5.290 píxeles de aportación del cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Hasta ahora sabemos qué hace la PPU por software en los 5.290 píxeles de aportación del cuadro 1000 (H15, capa debajo = fondo 2). No sabemos qué hace el shim GPU en esos mismos píxeles. Esta propuesta mide eso, en dos formas: con el estado de fin de cuadro (lo que hace `pipeline.rs` hoy) y con estado por línea.

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md` (revisada por humano): 5.290 píxeles, H15 confirmada; control de la reconstrucción en 0.
  - `docs/propuestas/2026-10-17_capa-debajo-del-sprite-cuadro-1000.md` (verificada por agente): capa debajo = fondo 2 en los 5.290.
  - `docs/propuestas/2026-10-17_contenido-bg2-cuadro-1000.md` (verificada por agente): BG2 opaco en los 5.290.
  - `examples/ppu_frente_a_frente.rs`: ya renderiza el shim por GPU (`PpuShimPipeline::render_frame`, búfer de lectura con filas de 256 bytes, tolerancia ±1 por canal) y compara con la PPU en toda la pantalla. Es la referencia para leer el shim; no se modifica.
- Intención: para los 5.290 píxeles, saber qué color da el shim frente a la PPU con OBJ completo (V0) y sin OBJ (V1), y si el shim reacciona al cambio de OBJ en esos píxeles.
- Hipótesis:
  - H24: con estado de fin de cuadro, el shim coincide con la PPU V0 en menos del 50 % de los 5.290 píxeles. Se confirma si el conteo es menor que 2.645.
  - H25: con estado de fin de cuadro, el shim casi no cambia entre V0 y V1 en esos píxeles (el shim no reproduce la diferencia de OBJ). Se confirma si menos del 10 % de los 5.290 cambian.
  - H26: con estado por línea, el shim coincide con la PPU V0 en al menos el 90 % de los 5.290. Se confirma si el conteo es al menos 4.761.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia.
  - El trabajo corre en el PC con Windows, con GPU disponible (lo usa `ppu_frente_a_frente`). Si no hay adaptador de GPU, el ejemplo para.
  - El ROM solo en `roms/`. No sobrescribir `ppu_frente_a_frente.rs` ni `volcados/frente-a-frente.txt`.
- Alternativas descartadas:
  - Ampliar `ppu_frente_a_frente` con el cuadro 1000 y la máscara → descartada: ese arnés mide el cuadro entero en un rango y las pruebas de frente-a-frente dependen de su rango; mezclarlo rompería esas pruebas.
  - Usar solo el estado de fin de cuadro → descartada: no responde H26, que es lo que decide si el estado por línea arregla los píxeles de OBJ.

### 3. Ejemplos

- Caso normal: un píxel de la aportación con PPU V0 = `3062` y PPU V1 = `0C40`. El shim de fin de cuadro da `3062` en V0 y `3062` en V1: no reacciona al OBJ. El informe lo cuenta en "shim fin igual PPU V0" y en "shim fin V0 igual V1".
- Caso límite: si el shim no tiene adaptador de GPU, el ejemplo para con un mensaje y no escribe el informe.

### 4. Pasos

1. Crear `examples/shim_aportacion.rs` (con `required-features = ["proto"]`), reutilizando el código de GPU de `ppu_frente_a_frente.rs` (copia, no importación).
2. Correr el juego hasta el cuadro 1000 con captura por línea, igual que `ppu_capa_ganadora`.
3. Calcular la máscara de los 5.290 píxeles con la PPU (V0 distinto de V1 y V2 igual a V3).
4. Renderizar el shim con estado de fin de cuadro en V0 y V1 (cambiando DISPCNT en el búfer de I/O, igual que la PPU).
5. Renderizar el shim con estado por línea en V0 y V1 (agrupando líneas por estado, igual que `ppu_frente_a_frente`).
6. Contar, para los 5.290 píxeles: igual a PPU V0, igual a PPU V1, otro; y cuántos cambian entre shim V0 y shim V1.
7. Control: renderizar el shim dos veces con el mismo estado y comprobar que da lo mismo (determinismo).
8. Escribir `volcados/shim-aportacion-cuadro-1000.txt` (archivo nuevo).
9. Crear `tests/shim_aportacion_tests.rs` con las pruebas de la sección 8.
10. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_aportacion.rs` (nuevo), `tests/shim_aportacion_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/shim-aportacion-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu_frente_a_frente.rs`, `volcados/frente-a-frente.txt`, `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego, la APK.
- Riesgo: el shim puede dar resultados distintos en otra GPU. El control 7 detecta falta de determinismo en esta máquina, no en otras.

### 6. Suposiciones

- El shim recibe DISPCNT modificado en el búfer de I/O y lo interpreta igual que la PPU → prototipar: sí, con el control de determinismo y con el conteo de píxeles de la aportación que coinciden con V0 y V1 → si es falsa: parar.
- La tolerancia de ±1 por canal de `ppu_frente_a_frente` sirve también aquí → prototipar: sí → si es falsa: se reporta el conteo sin tolerancia.

### 7. Alcance

- Incluye: comparación del shim (fin de cuadro y por línea) con los 5.290 píxeles en V0 y V1; control de determinismo; informe y pruebas.
- No incluye: otros cuadros; ventanas, mezcla o residuo del 1,33 %; cambios al shim; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el shim es determinista en esta máquina
Caso de uso: que las diferencias no vengan de la GPU | Tipo: límite
  Dado el cuadro 1000
  Cuando se renderiza dos veces el mismo estado
  Entonces `control determinismo shim` vale 0
Test: `tests/shim_aportacion_tests.rs::test_shim_determinista` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: las categorías del shim suman el total de la aportación
Caso de uso: que ningún píxel quede sin clasificar | Tipo: normal
  Dado el informe
  Cuando se suman las categorías del shim de fin de cuadro
  Entonces la suma es 5290
Test: `tests/shim_aportacion_tests.rs::test_shim_suma_total` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da H24, H25 y H26 por separado
Caso de uso: decidir si el estado por línea resuelve los píxeles de OBJ | Tipo: normal
  Dado el informe
  Cuando se leen las líneas de resultado
  Entonces existen `shim fin igual PPU V0`, `shim fin V0 distinto de V1` y `shim linea igual PPU V0`
Test: `tests/shim_aportacion_tests.rs::test_shim_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_aportacion -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/shim-aportacion-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test shim_aportacion_tests` → pasa si: las tres pruebas pasan.
- Regresión: `volcados/frente-a-frente.txt` no cambia.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice si H24, H25 y H26 se confirman o se descartan, con sus conteos.

### 11. Parar si

- El control de determinismo falla: parar, porque las diferencias no serían atribuibles al shim.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecutado 2026-10-17, cuadro 1000 (`volcados/shim-aportacion-cuadro-1000.txt`):

Controles:
- Determinismo del shim: 0 bytes distintos.
- Control de arnés (cuadro completo, V0 igual a la PPU): con estado de fin, 14.508 de 38.400 píxeles (37,8 %); con estado por línea, 32.870 de 38.400 (85,6 %). El arnés funciona: el shim sí coincide con la PPU en la mayoría de la pantalla.

Aportación (5.290 píxeles):
- Shim con estado de fin, V0 igual a PPU V0: 0. Igual a PPU V1: 0. Categoría otro: 5.290.
- Shim con estado por línea, V0 igual a PPU V0: 0. Igual a PPU V1: 0.
- Shim V0 distinto de V1, con estado de fin: 0. Con estado por línea: 0. El shim da el mismo color con OBJ que sin OBJ en estos píxeles.

Hipótesis:
- H24 (fin coincide con V0 en menos del 50 %): confirmada, 0 de 5.290.
- H25 (el shim casi no cambia entre V0 y V1): confirmada, 0 píxeles cambian.
- H26 (estado por línea coincide con V0 en al menos el 90 %): descartada, 0 de 5.290. Aunque el estado por línea mejora la pantalla completa (85,6 %), en estos 5.290 píxeles no mejora nada.

Lectura: el shim no reproduce la aportación de OBJ. Tampoco coincide con el fondo 2 de la PPU V1, así que no sabemos aún qué color muestra. La hipótesis más directa es que muestra el fondo vacío (V3), pero no está medido.

Nota de proceso: la primera corrida no tenía los controles de cuadro completo; dio los mismos conteos de aportación. Los controles se añadieron después para validar el arnés. Pruebas: las tres de `shim_aportacion_tests` pasan.

---

## Revisión humana

- Que la comparación del shim con los 5.290 píxeles sea suficiente para decidir si el estado por línea es la siguiente corrección, y que H24, H25 y H26 se confirmen o descarten: ✔ | ✘ | ?
