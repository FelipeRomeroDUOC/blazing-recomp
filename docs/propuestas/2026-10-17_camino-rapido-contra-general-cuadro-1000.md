# Propuesta: Comprobar si el camino rápido y el camino general de la PPU por software dan los mismos píxeles en el cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente (pendiente de revisión humana)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La referencia de software (`gba-core/src/ppu.rs`) compone cada línea por dos caminos: el rápido (`compose_straight`, `compose_alpha_tramos`) y el general (un píxel a píxel). El camino se elige por línea (`razon_camino`). Si los dos caminos dan colores distintos para el mismo píxel, la referencia es inconsistente y todas las medidas que dependen de ella quedan en duda.

Esta propuesta mide si eso ocurre en el cuadro 1000, con los dos caminos forzados sobre las mismas capturas. No cambia `ppu.rs`, `gba-core`, el shim ni el juego.

### 2. Contexto

- Contexto consultado:
  - `gba-recomp/crates/gba-core/src/ppu.rs`, leído en solo lectura el 2026-10-17:
    - `compose` elige `compose_straight` (con o sin efecto), `compose_alpha_tramos` o el camino general según `razon_camino` (0 a 3).
    - `straight_obj` se usa cuando OBJ está encendido (`DISPCNT` bit 12). `straight_bgs` cuando está apagado. Son funciones distintas.
    - Con `perf-video` existe la función pública `forzar_camino_general(activo: bool)`, que manda todas las líneas al camino general.
  - `docs/propuestas/2026-10-17_diagnostico-efectos-y-ventanas-cuadro-1000.md`: `BLDCNT = 0` deja 6.240 píxeles de diferencia en OBJ. La mezcla no explica por sí sola la diferencia de OBJ.
  - Salida existente `volcados/pixeles-obj-cuadro-1000.txt`: 6.000 píxeles de aportación de OBJ, 710 dentro de cobertura y 5.290 fuera, en 25 líneas. Esa salida la generó una versión del ejemplo que no es la de esta propuesta, y no se ha revisado. Su informe de "cambio de ruta: 0" puede estar mal calculado, porque quitar OBJ cambia si hay sprites translúcidos. Esta propuesta no lo usa como dato.
- Intención: separar dos causas posibles de la aportación de OBJ fuera de cobertura: una inconsistencia entre caminos (error de la referencia) o un efecto real de composición. Éxito: un informe con los conteos de diferencia entre caminos y la aportación con cada camino.
- Hipótesis:
  - H13: el camino rápido y el general difieren en píxeles de las líneas del cuadro 1000. Se confirma si, con el mismo estado, `D0` o `D1` (definidos abajo) es mayor que 0.
  - H14: la aportación de OBJ fuera de cobertura es un efecto real de composición, igual en los dos caminos. Se confirma si `D0 = 0` y `D1 = 0`, y la aportación con el camino general es distinta de 0.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia (decisión previa, 2026-10-16). Esta propuesta no la cambia; solo la mide.
  - El trabajo corre en el PC con Windows (decisión previa, 2026-10-16).
  - Cualquier cambio en `ppu.rs` requiere otra propuesta y aprobación (regla del proyecto).
- Alternativas descartadas:
  - Leer `razon_camino` desde el ejemplo → descartada: es variable local de `compose`. Se usa el interruptor público en su lugar.
  - Usar la salida existente de 5.290 → descartada: no se revisó su versión y su medida de ruta es dudosa.

### 3. Ejemplos

- Caso normal: `D0 = 0` y `D1 = 0`. Entonces los caminos coinciden en V0 y V1. Si además la aportación con el camino general es 5.290, H14 se confirma: la diferencia es real y no viene de la ruta.
- Caso límite: `D0 = 0`, `D1 = 37`. Entonces el camino rápido se equivoca solo cuando OBJ está apagado. H13 se confirma y el informe lo dice por separado.

### 4. Pasos

1. Crear `examples/ppu_camino_general.rs` (con `required-features = ["proto"]`). Reutiliza la captura por línea del cuadro N, como `ppu_capas_linea`.
2. Renderizar cada línea en cuatro variantes:
   - V0r: completa, camino por defecto;
   - V1r: sin OBJ, camino por defecto;
   - V0g: completa, `forzar_camino_general(true)`;
   - V1g: sin OBJ, `forzar_camino_general(true)`.
   Restaurar `forzar_camino_general(false)` al terminar.
3. Calcular:
   - control: V0r frente al framebuffer del juego (debe ser 0);
   - `D0` = píxeles distintos entre V0r y V0g;
   - `D1` = píxeles distintos entre V1r y V1g;
   - aportación rápida = V0r frente a V1r;
   - aportación general = V0g frente a V1g;
   - líneas con `D0` o `D1` mayor que 0.
4. Escribir `volcados/camino-general-cuadro-1000.txt` con una línea por medida, con prefijos `camino control`, `camino D0`, `camino D1`, `camino aportacion rapida`, `camino aportacion general` y `camino lineas con diferencia`.
5. Crear `tests/camino_general_tests.rs`, con las pruebas de la sección 8.
6. Añadir la entrada `[[example]]` en `Cargo.toml`, ejecutar y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_camino_general.rs` (nuevo), `tests/camino_general_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/camino-general-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `gba-core`, `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego y la APK.
- Riesgo: `forzar_camino_general` es global. El ejemplo es de un solo hilo, así que no hay carrera, pero hay que restaurar el valor siempre.
- Riesgo: si el ejemplo se compila sin `perf-video`, la función no existe. Por eso `required-features = ["proto"]`.

### 6. Suposiciones

- `forzar_camino_general(true)` manda todas las líneas por el camino general → prototipar: sí, comparando `razon_camino` implícita con el resultado: si una línea coincide con V0r siempre, el interruptor no hace nada → si es falsa: parar.
- El cuadro 1000 de esta secuencia es el mismo que el de las demás mediciones → prototipar: sí, con el control de V0r frente al juego → si es falsa: parar.

### 7. Alcance

- Incluye: medida de diferencia entre caminos, aportación con cada camino, informe y pruebas.
- No incluye: cambiar `ppu.rs` o corregir el camino rápido; el shim; otros cuadros; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el informe da la diferencia entre caminos para las dos variantes
Caso de uso: saber si el camino rápido se equivoca en el cuadro 1000 | Tipo: normal
  Dado el cuadro 1000 con sus capturas por línea
  Cuando se corre el ejemplo
  Entonces `volcados/camino-general-cuadro-1000.txt` tiene `camino D0` y `camino D1`, cada una con un número entero
Test: `tests/camino_general_tests.rs::test_informe_da_diferencias` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: la variante de control coincide con el juego
Caso de uso: comprobar que el experimento es válido antes de medir | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `camino control` es 0
Test: `tests/camino_general_tests.rs::test_control_coincide_con_juego` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: la aportación con el camino general se da por separado
Caso de uso: separar la inconsistencia de ruta del efecto real de composición | Tipo: normal
  Dado el informe del ejemplo
  Cuando se leen sus líneas
  Entonces existen `camino aportacion rapida` y `camino aportacion general`
Test: `tests/camino_general_tests.rs::test_aportaciones_separadas` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example ppu_camino_general -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000 > ..\camino-general-output.txt 2>&1` → pasa si: el informe tiene todas las líneas de la sección 8 → no detecta: diferencias que solo aparezcan en otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test camino_general_tests` → pasa si: las tres pruebas pasan.
- Control: V0r debe dar 0 diferencias con el juego. Si no, se para.
- Regresión: `git diff --stat` → pasa si: no aparecen `gba-core`, `ppu.rs`, `ppu_shim.rs` ni `ppu_shim.wgsl`.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice si H13 y H14 se confirman o se descartan, con los números de `D0`, `D1` y las dos aportaciones.

### 11. Parar si

- El control V0r no coincide con el juego: parar, porque el experimento no es válido.
- `forzar_camino_general` no está disponible en la compilación: parar y revisar las features antes de seguir.
- `D0` o `D1` son mayores que 0 (H13 confirmada): parar y presentar la evidencia. Corregir el camino rápido requiere cambiar `ppu.rs`, y eso necesita otra propuesta y tu aprobación.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_camino_general`, cuadro 1000 de `secuencias/intro.txt`. Pruebas `tests/camino_general_tests.rs`: 3 pasan.

| Medida | Píxeles o líneas |
| :--- | ---: |
| Control: camino por defecto (V0r) frente al juego | 0 |
| D0: V0r frente a V0g (camino general forzado, con OBJ) | 0 |
| D1: V1r frente a V1g (camino general forzado, sin OBJ) | 0 |
| Aportación de OBJ con el camino por defecto (V0r frente a V1r) | 6.000 |
| Aportación de OBJ con el camino general (V0g frente a V1g) | 6.000 |
| Líneas con alguna diferencia entre caminos | 0 |

Lectura:
- **H13 descartada.** Los dos caminos dan exactamente los mismos píxeles en las 160 líneas, con OBJ y sin OBJ. La referencia de software es consistente en este cuadro.
- **H14 confirmada.** La aportación de OBJ es de 6.000 píxeles con los dos caminos. La diferencia no viene de la ruta de composición, sino de la composición en sí.

Lo que este Resultado **no** explica: el mecanismo. Los 6.000 píxeles se dividen en 710 dentro de la cobertura de OBJ sola y unos 5.290 fuera, según una salida que no se ha verificado en esta propuesta (`volcados/pixeles-obj-cuadro-1000.txt`, generada por otra versión del ejemplo). Esa salida ya no se usa como base. Tampoco es mezcla de color: en `BLDCNT = 0` la aportación sigue en 6.240 (diagnóstico previo).

Archivos: `examples/ppu_camino_general.rs` (nuevo), `tests/camino_general_tests.rs` (nuevo), `Cargo.toml` (una entrada `[[example]]`), `volcados/camino-general-cuadro-1000.txt` (generado, no versionado). `ppu.rs` solo se leyó en lectura: no cambió. `gba-core`, el shim y el juego no cambiaron.

---

## Revisión humana

- Que el informe separe la diferencia entre caminos de la aportación real de OBJ, y que H13 y H14 queden confirmadas o descartadas con sus números: ✔ | ✘ | ?
