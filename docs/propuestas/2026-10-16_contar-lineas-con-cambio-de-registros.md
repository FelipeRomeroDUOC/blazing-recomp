# Propuesta: Contar las líneas de pantalla cuyos registros de vídeo cambian respecto a la línea anterior

Fecha: 2026-10-16
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El informe de comparación indica, para cada cuadro, en cuántas líneas de la pantalla el juego cambió sus registros de vídeo o su paleta respecto a la línea anterior. Así se ve si la imagen cambia dentro del cuadro, que es lo que el shim no reproduce.

### 2. Contexto

- Contexto consultado: `docs/propuestas/2026-10-16_contar-escrituras-de-registros-por-cuadro.md` (resultado: el contador de escrituras no separa cambios a mitad de cuadro de copias normales); `examples/ppu_compare.rs` (salida real del último informe); `gba-recomp/crates/gba-core/src/mem.rs` (funciones `write8`, `write16`, `write32` y el contador `GbaPerfCounters`, leídos en la sesión; el punto donde se llama `render_scanline` en HBlank se verificó en la sesión anterior, línea ~709, y se vuelve a verificar en el paso 1).
- Intención: decidir si el grano de la intro (cuadro 1000) viene de cambios entre líneas. Éxito: el informe muestra, por cuadro, un número de líneas con cambio, y ese número permite decir si el shim tiene que reproducir estado por línea o no.
- Restricciones y decisiones previas (máx. 3):
  - La PPU por software se conserva y no se borra (origen: respuesta humana, 2026-10-16) → implica: el contador solo observa; no cambia cómo se dibuja la línea → protegida por: "El conteo no cambia la imagen del juego".
  - El contador queda detrás de `perf-counters`, como el resto de la instrumentación (origen: `gba-recomp/crates/gba-core/Cargo.toml`, feature `perf-counters`) → implica: el contador existe en el APK porque `blazing-recomp` activa esa feature en todas sus compilaciones; hay que medir el costo antes de dejarlo → no testeable (depende del dispositivo; se revisa en la revisión humana).
  - El trabajo es en el PC con Windows, no en Android (origen: respuesta humana, 2026-10-16) → implica: el harness no se compila para Android → no testeable directamente; se verifica al correrlo en el PC.
- Alternativas descartadas:
  - Contar escrituras totales o de E/S → descartada: el resultado de la propuesta anterior mostró que mezclan copias masivas con cambios reales, y los accesos de 16 bits se cuentan dos veces.
  - Copiar la VRAM o los registros en cada línea desde el arnés → descartada porque el arnés no tiene acceso al momento de cada línea sin tocar `gba-core`.
- Ante imprevistos, prioriza: que la imagen del juego no cambie, sobre tener un número exacto de cambios.

### 3. Ejemplos

- Caso normal: cuadro 1000 de la intro → antes: no sabemos si la imagen cambia entre líneas → ahora: el informe muestra cuántas líneas (de 0 a 159) tienen registros distintos de la línea anterior.
- Caso límite: cuadro en el que el juego no cambia nada durante el dibujo → antes: no aplica → ahora: el informe muestra 0 y no hay error.

### 4. Pasos

1. Confirmar en `gba-recomp/crates/gba-core/src/mem.rs` el punto donde se llama `render_scanline` en HBlank y en qué orden se dibujan las líneas (solo lectura).
2. Prototipo: comprobar que el contador de líneas funciona en una sola línea de prueba, sin cambiar la imagen.
3. Añadir `lineas_con_cambio` a `GbaPerfCounters`. En el punto de la línea 1, guardar una copia de los registros de E/S (0x00 a 0x5F) y de la paleta (1 KB). En cada línea siguiente, comparar con la copia anterior; si cambió algo, sumar 1 y actualizar la copia. Todo dentro de `#[cfg(feature = "perf-counters")]`.
4. Exponer `lineas_con_cambio` en `examples/ppu_compare.rs` como columna del CSV, con la diferencia antes y después de cada cuadro.
5. Medir el costo en el APK comparando el tiempo de `ejec` con y sin la feature, usando el flag de telemetría existente.
6. Escribir las pruebas de los escenarios de la sección 8.

### 5. Qué más podría verse afectado

- Búsqueda de referencias: se hizo solo sobre `mem.rs` (funciones de escritura y el struct). La búsqueda completa de `render_scanline` y de sus llamadas se hace en el paso 1, antes de editar.
- Archivos que probablemente cambian: `gba-recomp/crates/gba-core/src/mem.rs` (HBlank y struct), `blazing-recomp/examples/ppu_compare.rs`, `tests/ppu_compare_tests.rs` (nuevo).
- Archivos que no cambian: `src/pipeline.rs`, `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, la lógica de dibujo de `gba-core` y Android.
- El contador `escrituras_video` y `escrituras_es` que se agregaron antes quedan en el código. Se deciden después si se quitan (propuesta aparte).

### 6. Suposiciones

- Si el grano del cuadro 1000 viene de cambios entre líneas, el cuadro 1000 tendrá más de 0 líneas con cambio → prototipar: sí → si es falsa (0 líneas): la causa no es un cambio de registros por línea y el plan de estado por línea del shim no se justifica → resultado del prototipo: pendiente | no detecta: que el cambio sea de un registro que no se compara (por ejemplo, el de un sprite).
- Los registros de E/S y la paleta cubren los cambios que afectan a la imagen de fondos → prototipar: no → si es falsa: habrá que sumar la OAM y la VRAM al conteo → resultado: no probado | no detecta: cambios de VRAM que el juego hace a mitad de línea.
- Comparar 160 líneas por cuadro con una copia de 1 KB de paleta y 96 bytes de E/S cuesta poco frente a dibujar la línea → prototipar: sí → si es falsa: el contador se deja apagado en el APK y solo se usa en el PC → resultado del prototipo: pendiente | no detecta: costo en la TV, que se mide en el paso 5.

### 7. Alcance

- Incluye: contador de líneas con cambio por cuadro (solo observa), columna en el informe, medición de costo en el APK, pruebas de los escenarios.
- No incluye: cambios en cómo se dibuja el juego ni en el shim; estado por línea en el shim (eso es una propuesta aparte); eliminar los contadores de escrituras ya agregados.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el conteo no cambia la imagen del juego
Caso de uso: medir sin alterar lo que se mide | Tipo: regresión
  Dado que la secuencia `secuencias\intro.txt` y el ROM están en sus rutas locales
  Cuando se corren 1.000 cuadros con el conteo activo y luego sin él
  Entonces los hashes de los 1.000 cuadros son iguales en ambas corridas
Test: `tests/ppu_compare_tests.rs::test_conteo_de_lineas_no_cambia_imagen` (se omite si no está definida la variable `FE_ROM`)
Estado previo: falla (el conteo no existe)

Escenario: cada cuadro tiene un número de líneas con cambio entre 0 y 159
Caso de uso: revisar la intro completa | Tipo: normal
  Dado el ROM y la secuencia de la intro
  Cuando se corre el harness
  Entonces en el CSV la columna `lineas_con_cambio` existe y en los 11.110 cuadros su valor es un entero entre 0 y 159
Test: `tests/ppu_compare_tests.rs::test_lineas_con_cambio_en_rango`
Estado previo: falla (la columna no existe)

Escenario: un cuadro sin cambios de registros registra 0 líneas con cambio
Caso de uso: comprobar que el conteo no inventa cambios | Tipo: límite
  Dado un cuadro en el que el juego no escribe en los registros de vídeo ni en la paleta durante el dibujo
  Cuando se corre el harness
  Entonces el valor de `lineas_con_cambio` en ese cuadro es 0
Test: `tests/ppu_compare_tests.rs::test_cuadro_sin_cambios_registra_cero`
Estado previo: falla (el conteo no existe)

### 9. Cómo lo comprobaré

- Conteo no cambia la imagen → `cargo test --test ppu_compare_tests test_conteo_de_lineas_no_cambia_imagen` con `FE_ROM` apuntando al ROM → pasa si: los hashes de las dos corridas coinciden → no detecta: diferencias que solo aparecen en la TV.
- Rango de la columna → `cargo run --release --features proto --example ppu_compare -- <rom> secuencias\intro.txt ppu-compare-informe.csv` y `cargo test --test ppu_compare_tests test_lineas_con_cambio_en_rango` → pasa si: todos los valores están entre 0 y 159 → no detecta: si el número coincide con lo que dibuja el hardware real.
- Cuadro sin cambios → `cargo test --test ppu_compare_tests test_cuadro_sin_cambios_registra_cero` → pasa si: el valor es 0 → no detecta: cambios de registros que el conteo no mira (por ejemplo, la OAM).
- Costo en la TV → revisión humana: comparar `ejec` en `blazing-perf.txt` con la feature activa y apagada → no detecta: diferencias de otros dispositivos.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación, los prototipos tienen resultado y el costo en el APK está medido.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: contar en `render_scanline` (cambio de registros por línea dentro de la función de dibujo) -> Ahora: contar en `process_video_event`, justo antes de `render_scanline`, omitiendo la línea 0 -> Por qué: el punto de llamada en HBlank ya tiene el estado de la línea, y la línea 0 se compararía contra el cuadro anterior.
- Antes: el conteo corre con la feature perf-counters, que el APK activa -> Ahora: feature propia perf-video (activa solo con el harness, feature proto); los contadores de escrituras y el de líneas quedan bajo perf-video -> Por qué: decisión humana (2026-10-17): el APK no debe pagar el costo de medición que solo se usa en el PC.
- Resultado parcial (2026-10-17): el conteo no cambia la imagen (9.731 cuadros con diferencias y 162.306.583 píxeles, igual que antes); lineas_con_cambio va de 0 a 84, dentro del rango esperado; cuadro 1000 y 500 con 30 líneas con cambio; cuadros 303 y 304 con 100 % de diferencias y 0 líneas con cambio, porque el fade cambia BLDY una vez por cuadro y el conteo por línea no lo registra.

---

## Resultado (lo completa el agente después de editar)

Pendiente de aprobación.

---

## Revisión humana

- Que el costo del conteo en la TV sea aceptable: ✔ | ✘ | ?
