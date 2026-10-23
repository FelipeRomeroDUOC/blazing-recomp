# Propuesta: Contar cuántas veces el juego cambia los registros de vídeo dentro de cada cuadro

Fecha: 2026-10-16
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El informe de comparación muestra, para cada cuadro, cuántas veces el juego escribió en los registros de vídeo, la paleta, la VRAM y la OAM mientras dibujaba ese cuadro. Así se ve si la imagen cambia a mitad de cuadro, que es lo que el shim no reproduce.

### 2. Contexto

- Contexto consultado: `docs/propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md` (criterios y alcance del arnés); `examples/ppu_compare.rs` (salida real: columnas `dispcnt`, `bldcnt`, `bldalpha`, `bldy`, y sprites en OAM); `ppu-compare-informe.csv` (11.110 cuadros de la intro); volcados de los cuadros 298, 303, 500 y 1000 en `volcados/`; `src/ppu_shim.rs` (`decode_io_regs` lee el estado una sola vez por cuadro). `gba-core/src/mem.rs` no se ha leído para esta propuesta; eso se hace en el paso 1.
- Intención: poder decir, para un cuadro con diferencias, si el juego cambió registros a mitad del cuadro. Éxito: el informe muestra escrituras por cuadro y coinciden con los cambios de `BLDCNT` y `BLDY` que ya se observaron (cuadro 302 para `BLDCNT`, cuadro 303 para `BLDY`).
- Restricciones y decisiones previas (máx. 3):
  - La PPU por software se conserva y no se borra (origen: respuesta humana, 2026-10-16) → implica: el contador solo observa; no reemplaza ni cambia la PPU por software → protegida por: "El contador no cambia la imagen del juego".
  - El contador no altera el juego (origen: propuesta de arnés, sección 2, "el harness lee la salida del shim y no cambia la ruta del juego") → implica: no se modifica ningún valor que el juego lee; solo se cuentan las escrituras → protegida por: "El contador no cambia la imagen del juego".
  - La medición de tiempo de la TV usa un flag en tiempo de ejecución (origen: respuesta humana, 2026-10-16) → implica: el contador va detrás de una instrumentación apagada por defecto, igual que `perf_audio_timing` → no testeable (depende del dispositivo; se revisa en la revisión humana).
- Alternativas descartadas:
  - Leer el estado al final de cada cuadro, como hace el shim → descartada porque no responde si hubo cambios a mitad de cuadro.
  - Copiar VRAM y paleta en cada línea desde el arnés → descartada porque el arnés no tiene acceso al momento exacto de cada línea sin tocar `gba-core`.
- Ante imprevistos, prioriza: que la imagen del juego no cambie, sobre tener el contador completo.

### 3. Ejemplos

- Caso normal: secuencia de la intro, cuadros 302 y 303 → antes: no sabemos si el juego cambió registros a mitad de cada cuadro → ahora: el informe muestra al menos una escritura a `BLDCNT` en el cuadro 302 y al menos una a `BLDY` en el cuadro 303.
- Caso límite: cuadro sin escrituras de vídeo → antes: no aplica → ahora: el informe muestra 0 escrituras y no hay error.

### 4. Pasos

1. Leer en `gba-recomp/crates/gba-core/src/mem.rs` el camino de escritura de registros de vídeo, paleta, VRAM y OAM, y buscar dónde se reinicia el cuadro (solo lectura).
2. Prototipo: comprobar que `blazing-recomp` puede activar la instrumentación de `gba-core` (la función `perf-counters` que ya existe) sin cambiar el APK.
3. Añadir `escrituras_video` a `GbaPerfCounters` en `gba-core` y contarlo en `write8`, `write16` y `write32` para las regiones 0x04 a 0x07 (E/S, paleta, VRAM y OAM). El harness lee la diferencia antes y después de cada cuadro.
4. Exponer el contador en `examples/ppu_compare.rs` como columna `escrituras_video` del CSV.
5. Escribir las pruebas de los escenarios de la sección 8.

### 5. Qué más podría verse afectado

- Búsqueda de referencias: no se ha hecho todavía; se hace en el paso 1, antes de editar.
- Archivos que probablemente cambian: `gba-recomp/crates/gba-core/src/mem.rs` (camino de escritura), `blazing-recomp/Cargo.toml` (activar la instrumentación para el arnés), `examples/ppu_compare.rs`, `tests/ppu_compare_tests.rs` (nuevo).
- Archivos que no cambian: `src/pipeline.rs`, `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, el APK y Android.

### 6. Suposiciones

- Las escrituras de registros dentro del cuadro explican las diferencias de imagen (por ejemplo, el grano del cuadro 1000) → prototipar: no → si es falsa: el plan de estado por línea del shim no se justifica y hay que buscar otra causa → resultado del prototipo: pendiente | no detecta: correlación no es causalidad; para confirmarla habría que cambiar el estado y volver a comparar.
- Contar escrituras no cambia la imagen del juego → prototipar: sí → si es falsa: el contador se apaga durante las mediciones de imagen → resultado del prototipo: pendiente | no detecta: diferencias de tiempo que solo aparecen en la TV.
- La instrumentación `perf-counters` de `gba-core` puede activarse desde `blazing-recomp` → prototipar: sí → si es falsa: se agrega un flag propio al arnés → resultado del prototipo: verificada por lectura de `blazing-recomp/Cargo.toml` (línea 12: `features = ["perf-counters"]` sobre `gba-core`); la compilación con el cambio queda pendiente | no detecta: nada que cambie el resultado del arnés.

### 7. Alcance

- Incluye: contador de escrituras por cuadro (solo lectura del juego), columna en el informe, pruebas de los escenarios.
- No incluye: cambios en cómo se dibuja el juego ni en el shim; estado por línea en el shim (eso es una propuesta aparte); cambios en Android y en el APK.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el contador no cambia la imagen del juego
Caso de uso: medir sin alterar lo que se mide | Tipo: regresión
  Dado que la secuencia `secuencias\intro.txt` y el ROM están en sus rutas locales
  Cuando se corren 1.000 cuadros con el contador activo y luego sin él
  Entonces los hashes de los 1.000 cuadros son iguales en ambas corridas
Test: `tests/ppu_compare_tests.rs::test_contador_no_cambia_imagen` (se omite si no está definida la variable `FE_ROM`)
Estado previo: falla (el contador no existe)

Escenario: el cuadro 302 registra escritura a BLDCNT y el 303 a BLDY
Caso de uso: revisar qué cambió justo antes del fade de la intro | Tipo: normal
  Dado el ROM y la secuencia de la intro
  Cuando se corre el harness con el contador activo
  Entonces en el CSV el cuadro 302 tiene `escrituras_video` mayor o igual que 1 y `bldcnt` = `00ff`, y el cuadro 303 tiene `escrituras_video` mayor o igual que 1 y `bldy` = `0001`
Test: `tests/ppu_compare_tests.rs::test_cuadro_302_y_303_registran_escrituras`
Estado previo: falla (el contador no existe)

Escenario: secuencia sin cuadros termina con error claro
Caso de uso: ejecutar el harness con una secuencia vacía | Tipo: error
  Dado una secuencia con solo comentarios
  Cuando se ejecuta `cargo run --example ppu_compare -- <rom> secuencia_vacia.txt`
  Entonces el programa termina con código distinto de 0 y muestra `la secuencia no tiene cuadros`
Test: `tests/ppu_compare_tests.rs::test_secuencia_vacia_falla_con_mensaje`
Estado previo: falla (el harness aún no tiene esa prueba)

### 9. Cómo lo comprobaré

- Contador no cambia la imagen → `cargo test --test ppu_compare_tests test_contador_no_cambia_imagen` con `FE_ROM` apuntando al ROM → pasa si: los hashes de las dos corridas coinciden → no detecta: diferencias que solo aparecen en la TV.
- Cuadros 302 y 303 → `cargo run --release --features proto --example ppu_compare -- <rom> secuencias\intro.txt ppu-compare-informe.csv` y revisar las filas 302 y 303 del CSV → pasa si: las columnas coinciden con lo descrito → no detecta: si el número de escrituras coincide con el hardware real.
- Secuencia vacía → `cargo test --test ppu_compare_tests test_secuencia_vacia_falla_con_mensaje` → pasa si: código distinto de 0 y mensaje exacto → no detecta: errores en otros parámetros.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y los prototipos tienen resultado.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

- Antes: añadir un contador nuevo en `gba-core` con reinicio por cuadro, en `machine.rs` → Ahora: agregar `escrituras_video` al `GbaPerfCounters` que ya existe, y que el harness calcule la diferencia por cuadro; sin cambios en `machine.rs` → Por qué: la función `perf-counters` ya está activa en `blazing-recomp` y el struct ya acumula escrituras; el reinicio por cuadro no hace falta si el harness resta.
- Antes: una sola columna de escrituras de vídeo (E/S, paleta, VRAM y OAM) → Ahora: además, `escrituras_es` con solo las escrituras a registros de E/S (región 0x04) → Por qué: en el primer resultado, la mediana fue de unas 3.500 escrituras por cuadro, sobre todo copias masivas a VRAM y OAM, así que el total no separaba un cambio de registro a mitad de cuadro de una copia normal.

---

## Resultado (lo completa el agente después de editar)

Pendiente de aprobación.

---

## Revisión humana

- Que el contador esté apagado por defecto y no afecte los tiempos de la TV: ✔ | ✘ | ?
