# Propuesta: Capturar el estado de cada línea del cuadro 1000 para reproducirlo con la PPU por software

Fecha: 2026-10-17
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Mientras el juego dibuja el cuadro 1000, se guarda una copia de los campos públicos de `MemMap` justo antes de cada `render_scanline`. Después se redibuja cada línea desde su propia copia con la PPU por software y se compara con el framebuffer del juego. Esto prueba si el dibujo se puede reproducir fuera del juego cuando se conoce el estado de cada línea.

Requiere un cambio mínimo en `gba-core`, solo bajo la feature `perf-video` (no entra en la APK).

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_buscar-estado-faltante-cuadro-1000.md`, resultado: E1 (copia parcial) y E2 (todos los campos públicos) dan 22.185 píxeles distintos cada uno. Los campos públicos no explican la diferencia.
  - `docs/propuestas/2026-10-16_contar-lineas-con-cambio-de-registros.md`: en el cuadro 1000 hay 30 líneas con cambio de registros y 138 filas distintas; 109 caen en líneas sin cambio.
  - `gba-recomp/crates/gba-core/src/ppu.rs`, `render_scanline`: recarga los acumuladores afines (`aff_ref`) desde io al empezar la línea 0 o cuando `aff_dirty` está activo, y los avanza con PB/PD en cada línea. Un estado final no reproduce esos pasos si io cambió durante el cuadro.
  - `gba-recomp/crates/gba-core/src/mem.rs`, `process_video_event`: ya tiene un punto de cálculo por línea, bajo `perf-video`, que cuenta cambios antes de `render_scanline`. Ahí se puede enganchar la captura.
- Intención: obtener un conteo de píxeles distintos al reproducir cada línea desde su propio estado. Éxito: 0 diferencias, o una causa demostrada.
- Hipótesis:
  - H2 (principal): el estado que falta cambia entre líneas. Se confirma si la reproducción por línea da 0.
  - H3: el framebuffer del juego se escribe por algo que no es `render_scanline` (por ejemplo, DMA). Se confirma si la reproducción por línea da distinto de 0 y las diferencias no coinciden con las líneas que dibuja la PPU.
- Restricciones y decisiones previas:
  - La PPU por software no cambia (respuesta humana, 2026-10-16).
  - La captura solo existe con la feature `perf-video`. La APK no la usa (verificado en 2026-10-16 con `cargo tree`).
  - El trabajo corre en el PC con Windows (respuesta humana, 2026-10-16).
- Alternativas descartadas:
  - Capturar solo los registros de io por línea → descartada: no captura `aff_ref`, que sí cambia el dibujo.
  - Guardar el estado completo del bus (incluido audio y cartuchos) en cada línea → descartada: pesa mucho y no hace falta para el dibujo.

### 3. Ejemplos

- Caso normal: en el cuadro 1000, la línea 57 se reproduce desde su captura → antes: no sabemos si la línea coincide con el juego → ahora: 0 píxeles distintos en esa línea.
- Caso límite: una línea con `aff_dirty` activo → antes: el estado final la recargaría desde un io distinto → ahora: la captura de esa línea ya trae el `aff_ref` que usó el juego.

### 4. Pasos

1. Añadir en `gba-recomp/crates/gba-core/src/mem.rs`, solo bajo `perf-video`, un campo público `capturas_linea` (vector de la copia de campos por línea) y un indicador `capturar_linea` que lo activa. En `process_video_event`, justo antes de `render_scanline`, guardar la copia si el indicador está activo.
2. Comprobar que la APK sigue sin `perf-video` con `cargo tree` (igual que en la propuesta de líneas con cambio).
3. Crear `examples/ppu_linea.rs` (con `required-features = ["proto"]`):
   - corre el juego hasta el cuadro 1000, activando la captura solo durante ese cuadro;
   - guarda el framebuffer del juego;
   - reproduce cada línea desde su captura con `render_scanline` en una máquina auxiliar;
   - compara línea a línea y escribe `volcados/replay-cuadro-1000.txt` con el conteo por línea.
4. Añadir la entrada `[[example]]` en `Cargo.toml`.
5. Ejecutar y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `gba-recomp/crates/gba-core/src/mem.rs` (bajo `perf-video`), `examples/ppu_linea.rs` (nuevo), `Cargo.toml` (una entrada), `tests/ppu_linea_tests.rs` (nuevo).
- Archivos que no cambian: `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego y la APK.
- Memoria: 160 copias del estado por cuadro (la VRAM pesa 96 KB), unos 16 MB por cuadro. Solo se usa en el experimento.

### 6. Suposiciones

- El dibujo de cada línea depende solo de los campos públicos y de la copia de esa línea → prototipar: sí, con el paso 5 → si es falsa: H3 → resultado pendiente.
- Copiar `aff_ref` y `aff_dirty` en cada línea basta para los fondos afines → prototipar: sí, con el paso 5 → si es falsa: habrá que copiar también el estado de la sobreescritura de registros → resultado pendiente.

### 7. Alcance

- Incluye: captura por línea del cuadro 1000, reproducción línea a línea, conteo, pruebas.
- No incluye: el desglose por capa (propuesta `comparar-capas`, que se retoma después); cambios en la PPU por software o el shim; corrección de diferencias; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: cada línea reproducida desde su captura coincide con el juego
Caso de uso: confirmar que el dibujo se puede reproducir por línea | Tipo: regresión
  Dado el cuadro 1000 con la captura activa
  Cuando cada línea se redibuja desde su captura con la PPU por software
  Entonces `volcados/replay-cuadro-1000.txt` indica 0 píxeles distintos en las 160 líneas
Test: `tests/ppu_linea_tests.rs::test_replay_por_linea_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (la captura no existe)

Escenario: el informe tiene una fila por línea
Caso de uso: localizar líneas con diferencias | Tipo: normal
  Dado el informe del cuadro 1000
  Cuando se abre `volcados/replay-cuadro-1000.txt`
  Entonces tiene 160 filas, una por línea visible
Test: `tests/ppu_linea_tests.rs::test_informe_tiene_160_lineas` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: la PPU por software y el shim no cambian
Caso de uso: comprobar que la referencia sigue intacta | Tipo: regresión
  Dado el repositorio antes y después del experimento
  Cuando se comparan los archivos
  Entonces `gba-core/src/ppu.rs`, `src/ppu_shim.rs` y `src/ppu_shim.wgsl` no tienen cambios
Test: revisión manual con `git diff --stat`
Estado previo: cumple

### 9. Cómo lo comprobaré

- Captura y reproducción: `cargo run --release --features proto --example ppu_linea -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000 > ..\ppu-linea-output.txt 2>&1` → pasa si: imprime 0 píxeles distintos en total → no detecta: diferencias de color visibles solo en la TV.
- APK sin `perf-video`: `cargo tree -e features` sobre la APK → pasa si: `perf-video` no aparece → no detecta: cambios de tamaño de la APK no relacionados.
- Pruebas: `cargo test --features proto --test ppu_linea_tests` con `FE_ROM` definida → pasa si: todas las pruebas pasan o se omiten con el aviso.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el resultado está en la sección Resultado. Si el conteo es 0, se abre la propuesta del desglose por capa, retomando `comparar-capas`. Si no es 0, se registra qué líneas difieren y se propone el siguiente paso.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.
- El cambio en `gba-core` altera la APK (aparece `perf-video` en su árbol de features): parar y pedir aprobación.
- La reproducción por línea da distinto de 0 y las diferencias no coinciden con líneas que dibuja la PPU: parar y proponer H3.

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno todavía.

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_linea`, cuadro 1000, secuencia `secuencias\intro.txt`:

- Capturas: 160 (una por línea visible).
- Replay línea a línea desde su propia captura: 0 píxeles distintos de 38.400, en 0 líneas.
- Pruebas `tests/ppu_linea_tests.rs`: 2 pasan (`test_replay_por_linea_coincide`, `test_informe_tiene_160_lineas`).
- APK: `cargo tree -e features -i gba-core` muestra solo `default` y `perf-counters` para gba-core; `perf-video` no entra en la APK.

Lectura: H2 queda confirmada. El dibujo del cuadro 1000 se reproduce fuera del juego con la PPU por software, siempre que cada línea use su propio estado. La copia del estado final (propuesta anterior) no basta porque el estado cambia entre líneas. H3 no hace falta: el framebuffer del juego viene de `render_scanline`.

Archivos: `volcados\replay-cuadro-1000.txt` (resumen y 160 filas), `..\ppu-linea-output.txt`, `..\ppu-linea-tests-output.txt`, `..\ppu-linea-features.txt` (salidas; vienen en UTF-16 por la redirección de PowerShell).

Cambios de código: `gba-recomp/crates/gba-core/src/mem.rs` (solo bajo `perf-video`: struct `CapturaLinea`, campos `capturar_linea` y `capturas_linea`, y la captura antes de `render_scanline` en `process_video_event`); `examples/ppu_linea.rs` (nuevo); `Cargo.toml` (una entrada `[[example]]`); `tests/ppu_linea_tests.rs` (nuevo). `ppu.rs`, el shim y el juego no cambiaron.

---

## Revisión humana

- Que la reproducción por línea dé 0 diferencias, o identifique la línea o el mecanismo que falla: ✔ | ✘ | ?
