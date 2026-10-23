# Propuesta: Encontrar qué estado falta para reproducir el cuadro 1000 con la PPU por software

Fecha: 2026-10-17
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se averigua qué estado usa `render_scanline` para dibujar el cuadro 1000 y que no entra en la copia de io, paleta, VRAM y OAM. Para eso se hacen dos experimentos de solo lectura sobre el juego: una copia completa del bus y, si hace falta, una búsqueda del campo mínimo que falta. No se cambia la PPU por software, el shim ni el juego.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_comparar-capas-del-cuadro-fijo.md`, prototipo 2: la copia con io, paleta, VRAM y OAM da 22.185 píxeles distintos de 38.400 (esperado 0). Criterio no cumplido el 2026-10-17; la propuesta queda sin avanzar hasta resolver esto.
  - `docs/propuestas/2026-10-16_contar-lineas-con-cambio-de-registros.md`: en el cuadro 1000 hay 30 líneas con cambio de registros y 138 filas distintas; 109 de esas filas caen en líneas sin ningún cambio.
  - `gba-recomp/crates/gba-core/src/ppu.rs`, función `render_scanline` (leída parcialmente el 2026-10-17, primeros 9.000 bytes de 86.829): además de io, paleta, VRAM y OAM, usa campos de `MemMap` que la copia no tocaba:
    - `mem.aff_ref[0..4]`: acumuladores de los fondos afines.
    - `mem.aff_dirty[0..2]`: indicadores que fuerzan a recargar esos acumuladores desde BG2X/BG2Y/BG3X/BG3Y (io 0x28, 0x2C, 0x38, 0x3C).
    - Al inicio de cada línea 0, o cuando el indicador está activo, los acumuladores se recargan desde io. En cada línea después se avanzan con PB y PD (io 0x22, 0x26, 0x32, 0x36).
  - Lo que falta leer de `ppu.rs` (`compose`, `render_sprites`, ventanas y efectos de color) se hace en el paso 1.
- Intención: encontrar la causa de los 22.185 píxeles para que la copia sirva al desglose por capa. Éxito: un experimento con 0 diferencias, o una causa demostrada con su siguiente paso.
- Hipótesis, por orden de prueba:
  - H1: falta en la copia un campo de `MemMap` que el renderizador usa (por ejemplo `aff_ref` o `aff_dirty`). Se confirma si la copia completa da 0 diferencias.
  - H2: el renderizador depende de estado que cambia entre líneas, aunque los registros de io no cambien en esas líneas (por ejemplo, un acumulador afín que se recarga mientras `aff_dirty` está activo). Se confirma si la copia completa da distinto de 0 y una copia por línea daría 0. Esa segunda parte no se prueba aquí.
- Restricciones y decisiones previas:
  - La PPU por software se conserva y no se borra (respuesta humana, 2026-10-16). Por eso no se cambia para este experimento.
  - El juego no cambia; solo se leen estados y se usan copias (propuesta del arnés, sección 2).
  - El trabajo corre en el PC con Windows, no en Android (respuesta humana, 2026-10-16).
- Alternativas descartadas:
  - Añadir campos a la copia uno a uno según se me ocurran → descartada como primer paso: no sabemos cuáles faltan. La copia completa responde primero si el problema está dentro de `MemMap`.
  - Hacer una captura de todo el estado en cada línea → descartada en esta propuesta: exige cambiar `gba-core` (un hook por línea) y merece su propia propuesta si H1 queda descartada.
- Ante imprevistos, prioriza: no tocar la PPU por software ni el shim, sobre llegar a la causa.

### 3. Ejemplos

- Caso normal: copia completa del bus al final del cuadro 1000 → antes: 22.185 píxeles distintos con la copia parcial → ahora: el número de la copia completa queda escrito en `volcados/estado-cuadro-1000.txt`. Si da 0, H1 queda confirmada.
- Caso límite: la copia completa da 0 y se ha copiado el bus entero → antes: no sabemos qué campo faltaba → ahora: una bisección por grupos de campos deja un campo mínimo, o la lista de campos que hacen falta.

### 4. Pasos

1. Leer el resto de `gba-recomp/crates/gba-core/src/ppu.rs` (`compose`, `render_sprites`, ventanas, efectos de color) y anotar cada campo de `MemMap` que lee. Solo lectura. Resultado: lista en la sección Resultado.
2. Cambio respecto a lo aprobado: `MemMap` no tiene `Clone` (comprobado el 2026-10-17 en `mem.rs`), y añadirlo arrastraría tipos internos (APU, HLE de audio, cartuchos) que tampoco lo tienen. No se añade `Clone`. E2 copia campo a campo los campos públicos, y así no se toca `gba-core`.
3. Crear `examples/ppu_estado.rs` (nuevo, con `required-features = ["proto"]`, igual que `ppu_capas`):
   - corre el juego con la secuencia `intro.txt` hasta el cuadro indicado;
   - guarda el framebuffer del juego;
   - copia el bus completo (`copia.bus = m.bus.clone()`), renderiza las líneas 0 a 159 con `render_scanline` y cuenta los píxeles distintos (E2);
   - escribe el número en `volcados/estado-cuadro-1000.txt`.
4. Añadir la entrada `[[example]]` correspondiente en `Cargo.toml`.
5. Ejecutar E2 y registrar el resultado.
6. Si E2 da 0: bisección por grupos de campos de `MemMap` (copiar la mitad del bus y comprobar) hasta dejar el campo mínimo que hace falta. Máximo 8 ejecuciones.
7. Si E2 da distinto de 0: parar. No se investiga H2 aquí; se escribe una propuesta aparte para una captura por línea.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_estado.rs` (nuevo), `Cargo.toml` (una entrada `[[example]]`), `tests/ppu_estado_tests.rs` (nuevo). Solo si hace falta, `gba-recomp/crates/gba-core/src/mem.rs` (derive `Clone`).
- Archivos que no cambian: `gba-core/src/ppu.rs` (ni su lógica), `src/ppu_shim.rs`, `src/ppu_shim.wgsl`, `src/pipeline.rs`, el juego y Android.
- La APK no cambia: el experimento es solo de ejemplo con la feature `proto`.

### 6. Suposiciones

- Todo el estado que usa `render_scanline` está en campos públicos de `MemMap` → prototipar: sí, con E2, porque Rust impide a `ppu.rs` leer campos privados → si es falsa: E2 no cubre todo el estado y se amplía la copia con el campo que falte → resultado pendiente.
- El estado que usa `render_scanline` vive dentro de `MemMap`, no en `Machine` (CPU, temporizadores) → prototipar: sí, con E2 → si es falsa: E2 no es válido porque la copia del bus no basta; se para y se anota → resultado pendiente.
- Al final del cuadro 1000, `aff_ref` y `aff_dirty` valen lo mismo que el juego usó al empezar cada línea → si es falsa: H1 queda sin valor y se pasa a H2 → resultado pendiente.

### 7. Alcance

- Incluye: lectura de `render_scanline` y sus funciones auxiliares; copia completa del bus; bisección si E2 da 0; informe en `volcados/`; pruebas.
- No incluye: captura por línea; cambios en `ppu.rs` o en el shim; corrección de las diferencias (eso va en una fase posterior); Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la copia completa del bus se renderiza y su resultado queda registrado
Caso de uso: decidir si el estado que falta está dentro de `MemMap` | Tipo: normal
  Dado el estado del cuadro 1000 de la intro
  Cuando se renderiza la copia completa del bus con `render_scanline` en las líneas 0 a 159
  Entonces `volcados/estado-cuadro-1000.txt` contiene una línea con el número de píxeles distintos de 38.400
Test: `tests/ppu_estado_tests.rs::test_informe_estado_tiene_conteo` (se omite si no está definida la variable `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: si la copia completa da 0, se identifica el campo mínimo que falta
Caso de uso: saber qué estado añadir a la copia de desglose por capa | Tipo: normal
  Dado que el informe anterior da 0
  Cuando se ejecuta la bisección por grupos de campos
  Entonces el informe lista un campo de `MemMap` (o una lista de campos) que, copiado solo, hace que el resultado sea 0
Test: `tests/ppu_estado_tests.rs::test_biseccion_deja_un_campo_minimo` (se omite si no está definida la variable `FE_ROM`)
Estado previo: falla (la bisección no existe)

Escenario: la PPU por software no cambia durante el experimento
Caso de uso: comprobar que la referencia sigue intacta | Tipo: regresión
  Dado el repositorio antes y después del experimento
  Cuando se comparan los archivos
  Entonces `gba-core/src/ppu.rs`, `src/ppu_shim.rs` y `src/ppu_shim.wgsl` no tienen cambios
Test: revisión manual con `git diff --stat` (el repositorio no tiene prueba automática para esto)
Estado previo: cumple (no se ha cambiado nada todavía)

### 9. Cómo lo comprobaré

- E2 (copia completa): `cargo run --release --features proto --example ppu_estado -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000 > ..\ppu-estado-output.txt 2>&1` → pasa si: imprime un número de píxeles distintos y escribe `volcados\estado-cuadro-1000.txt` → no detecta: estado que vive fuera de `MemMap`.
- Pruebas: `$env:FE_ROM="roms\1575 - Fire Emblem (E)(Independent).gba"` y luego `cargo test --features proto --test ppu_estado_tests` → pasa si: todas las pruebas pasan o se omiten con el aviso de `FE_ROM` → no detecta: diferencias de color que solo se notan en la TV.
- Regresión: `git diff --stat` → pasa si: no aparecen `ppu.rs`, `ppu_shim.rs` ni `ppu_shim.wgsl` → no detecta: cambios no versionados.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el resultado de E2 está escrito en la sección Resultado. Si E2 da 0, también está escrito el campo mínimo que falta. Si E2 da distinto de 0, la propuesta termina con la propuesta de captura por línea ya escrita.

### 11. Parar si

- 2 intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.
- E2 da distinto de 0: parar y proponer la captura por línea en una propuesta aparte. No se continúa con H2 dentro de esta.
- Para completar el experimento hay que cambiar `ppu.rs` o el shim: parar y pedir aprobación.

### Correcciones previas a presentar

- Al aprobar, la propuesta decía que `MemMap` se podía clonar. No tiene `Clone`. Se corrige en el paso 2.
- Hipótesis H1 con mayor probabilidad de ser falsa: `render_scanline` recarga `aff_ref` desde io al empezar la línea 0 (y cuando `aff_dirty` está activo). La copia de E1 ya hace esa recarga, así que añadir `aff_ref` y `aff_dirty` solos probablemente no cambie el resultado. Si E2 da distinto de 0, la causa más probable es H2 (cambios de io durante el cuadro que la recarga desde io al inicio no reproduce).
- En `2026-10-17_comparar-capas-del-cuadro-fijo.md`, la sección 6 decía que la copia (io, paleta, VRAM y OAM) reproduce el juego, y que el estado interno no detectado era "estado interno de la PPU que no está en io, paleta, VRAM ni OAM". Esa lista estaba incompleta: `render_scanline` usa `aff_ref` y `aff_dirty`, que no se copiaron. Esta propuesta lo corrige y no cambia el criterio de aceptación de aquella.

### Cambios tras prototipos (solo si el plan cambió)

Ninguno todavía. Cambio de plan antes de ejecutar (2026-10-17): el paso 2 usa copia de campos públicos en vez de `clone()` (ver Paso 2).

---

## Resultado (lo completa el agente después de editar)

Ejecutado el 2026-10-17 con `examples/ppu_estado`, cuadro 1000, secuencia `secuencias\intro.txt`:

- E1 (copia parcial: E/S, paleta, VRAM y OAM): 22.185 píxeles distintos de 38.400.
- E2 (copia de todos los campos públicos de `MemMap`): 22.185 píxeles distintos de 38.400.

Lectura: H1 queda descartada. Copiar `aff_ref`, `aff_dirty` y el resto de campos públicos no cambia el resultado. Como E2 da distinto de 0, se para aquí (paso 7) y no se hace la bisección. La causa más probable es H2: el estado que falta cambia entre líneas, y una copia del estado final no lo reproduce. Eso necesita una captura por línea, que se propone en una propuesta aparte.

Archivos: `volcados\estado-cuadro-1000.txt` (informe), `..\ppu-estado-output.txt` (salida del ejemplo; viene en UTF-16 por la redirección de PowerShell).

Nota: el código cambió solo en `examples/ppu_estado.rs`, `Cargo.toml` (una entrada `[[example]]`) y `tests/ppu_estado_tests.rs`. `ppu.rs`, el shim y el juego no cambiaron.

---

## Revisión humana

- Que el experimento E2 identifique el campo que falta, o descarte H1 con un número claro: ✔ | ✘ | ?
