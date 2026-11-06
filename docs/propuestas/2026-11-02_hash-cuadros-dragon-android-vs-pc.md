# Propuesta: hash de cuadros para comparar Android y PC en la cinemática del dragón

Fecha: 2026-11-02
Nivel: 2 (más de 3 archivos, crea archivos y es el enésimo intento con el mismo problema)
Estado: verificada por agente

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Con un archivo-bandera activado, Android y el visor de PC escriben por cada cuadro de la escena del dragón una línea con: una huella de **lo que entra** a la GPU, una huella de **lo que la GPU dibujó** y los valores de mezcla de 4 líneas de pantalla. Un script compara ambos registros y dice en qué cuadro empiezan a diferir y si la diferencia está en la entrada o en la salida.

No arregla el parpadeo. Su objetivo es saber con certeza **dónde** se produce, en vez de seguir adivinando.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs`, `src/perf.rs:585-638`, `src/platform.rs:340-386`, `src/bin/ppu_doble.rs`, logs `blazing-perf-32/33`, capturas del usuario.
- Intención: descubrir por qué la capa de llamas (BG3) desaparece en cuadros sueltos mientras se escribe el subtítulo en Android y no en PC. Éxito = una línea del script que diga "primer cuadro distinto: rel=N, diferencia en ENTRADA | SALIDA".
- Hechos que motivan el diseño:
  - El visor de PC **no usa el mismo camino de CPU**: ejecuta el intérprete de `gba-core`. Android ejecuta el código AOT en C11. Ambos usan el mismo `NativePpu` y el mismo `ppu_nativa.wgsl`.
  - El PC no presenta en una superficie: lee la imagen de vuelta y la dibuja con GDI. Android presenta con Vulkan y Mailbox.
  - `# capas` solo registra los registros al final del cuadro, no lo que se dibujó.
- Restricciones:
  - Sin la bandera, el comportamiento y el rendimiento no cambian.
  - No romper la intro ya corregida.
- Alternativas descartadas:
  - Seguir corrigiendo heurísticas del dragón sin datos.
  - Comparar por número de cuadro absoluto.
- Ante imprevistos, prioriza: que la medición no altere el ritmo del juego por encima de la cantidad de datos medidos.

### 3. Ejemplos

Línea nueva (Android en `blazing-perf`, PC en un archivo):

```
# hash rel=37 frame=757 ent=9A41C2F0 sal=00000000 bld0=3F48 bld50=3F48 bld100=3F48 bld150=3F41 bg3_blend=118
```

### 4. Pasos

1. Agregar a `NativePpu` una función que calcule `ent` sobre los datos ya preparados en `subir_buffers_lineas` y guarde `bld0/50/100/150` y `bg3_blend`.
2. Agregar el permiso de copia (`COPY_SRC`) a la textura de salida.
3. Detectar el primer cuadro del dragón con la condición de `perf.rs` y registrar desde ahí 600 cuadros.
4. En Android: activar con el archivo `hash-cuadros.flag` en `filesDir`. Las líneas van a `blazing-perf`.
5. En PC: argumento `--hash-cuadros <archivo>` en `ppu_doble`.
6. Crear `scripts/diagnostico/comparar_hash_cuadros.py --android <log> --pc <log>`.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs`, `src/perf.rs`, `src/platform.rs`, `src/bin/ppu_doble.rs`, `src/ppu_nativa_tests.rs`.
- `scripts/diagnostico/comparar_hash_cuadros.py`.

### 6. Suposiciones

- Leer la textura de vuelta en Android no cambia el ritmo del juego → prototipar: sí
- `ppu_doble` llega a la escena del dragón solo → prototipar: sí
- Si el juego cambia BLDCNT a `0x3F61` mientras escribe el texto → prototipar: no

### 7. Alcance

- Incluye: medición detrás de bandera en Android y PC, y script de comparación.
- No incluye: arreglo del parpadeo, cambios en el shader ni en las heurísticas del dragón.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: La huella de entrada cambia si y solo si cambia la mezcla de una línea
Caso de uso: comparar Android y PC | Tipo: normal
  Dado un estado de cuadro sintético con BLDCNT=0x3F48 en las 160 líneas
  Cuando se calcula `ent` dos veces, y luego otra vez con la línea 50 en 0x3F41
  Entonces las dos primeras huellas son iguales, la tercera es distinta y `bld50` informa `3F41`
Test: `ppu_nativa_tests::test_hash_entrada_detecta_cambio_bldcnt_linea_50`
Estado previo: pasa

Escenario: Sin bandera no se emite ninguna línea de hash
Caso de uso: jugar normalmente | Tipo: límite
  Dado que `hash-cuadros.flag` no existe
  Cuando se procesan 10 cuadros con la condición del dragón activa
  Entonces se emiten 0 líneas `# hash` y no se crea ningún búfer de lectura
Test: `ppu_nativa_tests::test_hash_cuadros_inactivo_sin_bandera`
Estado previo: pasa

Escenario: El script informa el primer cuadro distinto y su tipo
Caso de uso: diagnosticar el parpadeo | Tipo: normal
  Dados dos logs sintéticos iguales hasta `rel=2`, y en `rel=3` con la misma `ent` y distinta `sal`
  Cuando se ejecuta `comparar_hash_cuadros.py` con ambos
  Entonces imprime `rel=3: ENTRADA igual, SALIDA difiere` y termina con código 1
Test: `scripts/diagnostico/test_comparar_hash_cuadros.py`
Estado previo: pasa

Escenario: La suite y los cuadros de la intro siguen iguales
Caso de uso: no romper lo ya corregido | Tipo: regresión
  Dado el código actual con 237 tests en verde
  Cuando se aplica el cambio
  Entonces `cargo test --lib` sigue con 239 en verde y 0 fallos
Test: suite existente
Estado previo: pasa (regresión)

Escenario (visible): La medición se ejecuta en el teléfono y en PC
Caso de uso: diagnóstico real | Tipo: normal
  Dado el APK nuevo con `hash-cuadros.flag` y `ppu_doble --hash-cuadros`
  Cuando se reproduce la intro hasta el final del dragón
  Entonces ambos logs tienen líneas `rel=0..599` y el script emite un veredicto

### 9. Cómo lo comprobaré

- Huella de entrada → `cargo test --lib test_hash` → pasa si las 3 huellas cumplen lo dicho.
- Sin bandera → `cargo test --lib test_hash` → pasa con 0 líneas.
- Script → `python scripts/diagnostico/test_comparar_hash_cuadros.py` → pasa con código 0.
- Regresión → `cargo test --lib` → 0 fallos (239 tests pasados).
- (visible) → revisión humana en hardware real.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y existe un veredicto del script con un log real.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Escenario: La huella de entrada cambia si y solo si cambia la mezcla de una línea: coincide
  Resultado real: Test unitario `test_hash_entrada_detecta_cambio_bldcnt_linea_50` pasó en verde.
  Método: el anunciado (`cargo test --lib test_hash`).

- Escenario: Sin bandera no se emite ninguna línea de hash: coincide
  Resultado real: Test unitario `test_hash_cuadros_inactivo_sin_bandera` pasó en verde.
  Método: el anunciado (`cargo test --lib test_hash`).

- Escenario: El script informa el primer cuadro distinto y su tipo: coincide
  Resultado real: Suite `test_comparar_hash_cuadros.py` superó los 4 casos de prueba sintéticos con código 0.
  Método: el anunciado (`python scripts/diagnostico/test_comparar_hash_cuadros.py`).

- Escenario: La suite y los cuadros de la intro siguen iguales: coincide
  Resultado real: 239 pasados, 0 fallos en la librería de Rust.
  Método: el anunciado (`cargo test --lib`).

- Escenario (visible): La medición se ejecuta en el teléfono y en PC: pendiente de ejecución humana con hardware real.

### Contexto: ¿se respetó?

- Restricciones respetadas: No se modificó la lógica de composición ni shaders; la emisión queda estrictamente condicionada a `hash-cuadros.flag` en Android y `--hash-cuadros` en PC.
- Fat APK generado: `target/blazing-recomp-release.apk` (44.806.984 bytes).

### Archivos modificados

- `scripts/diagnostico/comparar_hash_cuadros.py` (creado)
- `scripts/diagnostico/test_comparar_hash_cuadros.py` (creado)
- `src/ppu_nativa.rs` (modificado)
- `src/ppu_nativa_tests.rs` (modificado)
- `src/platform.rs` (modificado)
- `src/bin/ppu_doble.rs` (modificado)

---

## Revisión humana

- La medición se ejecuta en el teléfono y en PC (visible): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
