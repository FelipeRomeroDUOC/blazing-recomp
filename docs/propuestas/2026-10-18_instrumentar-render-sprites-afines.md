# Propuesta: Instrumentar temporalmente `render_sprites` (gba-core) para contar en qué etapa se pierden los píxeles de los afines de 7500

Fecha: 2026-10-18
Nivel: 2
Estado: cerrada (obsoleta; superada por la implementacion de la PPU nativa en GPU)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Cambio temporal y reversible en `gba-recomp\crates\gba-core\src\ppu.rs`, detrás de una feature nueva `diag-afin` que no se activa por defecto:

- Contadores por etapa (thread_local o estáticos atómicos) dentro de `render_sprites`, solo para los sprites afines cuyo índice OAM esté en la lista `{0,1,2,3,4}` y solo en el cuadro que se mida.
- Una función pública `diag_afin_contadores()` que devuelve los contadores, usada solo por un ejemplo.
- Un ejemplo nuevo `blazing-recomp\examples\shim_afin_etapas.rs` que activa la feature, corre la intro hasta 7500 y escribe `volcados/afin-etapas-7500.txt`.

Al terminar, los cambios en `ppu.rs` se revierten (se restaura el archivo original desde una copia guardada antes del cambio). La medición queda en el informe; el código no se queda.

No cambia el comportamiento de la PPU cuando la feature está apagada. No cambia `src/` del shim.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_direccion-texel-composicion-afines.md` (verificada): la dirección del texel y la regla de prioridad coinciden con mi medición; H242 quedó abierta. La PPU debería pintar 154 píxeles de los afines y no los pinta.
  - `docs/propuestas/2026-10-18_pixeles-afines-cuadro-7500.md` (verificada): 0 píxeles aportados por la PPU en los 5 afines.
  - Por qué no basta con otro ejemplo: `render_sprites` y `obj_line` son internos de `gba-core`; un ejemplo externo no puede verlos. Reimplementar la función daría otra vez el mismo resultado que mi medición ya hecha (154), sin explicar la pérdida dentro de la PPU.
- Intención: localizar la etapa exacta donde desaparecen los píxeles de cada afín dentro de la PPU real.
- Hipótesis:
  - H250: los píxeles se pierden antes de la escritura: `u`/`v` salen de rango o el rango `min_bx..max_bx` está vacío. Si se confirma, la caja o la matriz que usa la PPU no coinciden con las mías.
  - H251: los píxeles llegan a `color_idx != 0`, pero no pasan la escritura en `out` (prioridad ya ocupada por otro sprite). Si se confirma, otro sprite de prioridad igual o menor tapa estos píxeles en el búfer de sprites.
  - H252: los píxeles se escriben en `out`, pero se pierden después, en la composición (`obj_line` no llega a `compose`, o `obj_span` no se usa). Si se confirma, el fallo está fuera de `render_sprites`.
  - H253: ninguna de las anteriores. Si se confirma, hace falta otra propuesta.
- Restricciones y decisiones previas:
  - Cambio temporal: la feature se apaga y el archivo se restaura al terminar.
  - Solo el cuadro 7500 y solo los afines 0 a 4.
  - Sin cambios en el comportamiento por defecto de `gba-core`.
- Alternativas descartadas:
  - Reimplementar `render_sprites` en un ejemplo → descartada: no puede ver `obj_line` ni los pasos internos de la PPU.
  - Dejar la instrumentación en el código → descartada: se revierte al terminar.

### 3. Ejemplos

- Caso normal: el afín 2 tiene 42 píxeles con texel no transparente dentro de la textura. Los contadores muestran 42 en "texel no transparente" y 0 en "escritos en out" → H251 o H252, según el valor de "escritos en out" en la composición.
- Caso límite: el afín 4 tiene 0 píxeles con `u`/`v` en rango → H250 para ese sprite, y la caja de la PPU no coincide con la mía.

### 4. Pasos

1. Guardar una copia de `gba-recomp\crates\gba-core\src\ppu.rs` y de `Cargo.toml` de `gba-core` en `blazing-recomp\volcados\respaldo-ppu\` (antes de cualquier cambio).
2. Añadir la feature `diag-afin` y los contadores detrás de `#[cfg(feature = "diag-afin")]`.
3. Crear `examples/shim_afin_etapas.rs` (`required-features = ["proto", "diag-afin"]`, o la feature equivalente que exponga `gba-core`).
4. Ejecutar la medición en 7500 con `secuencias\intro.txt`. Salida: `volcados/afin-etapas-7500.txt`, con los contadores por afín y etapa.
5. Restaurar `ppu.rs` y `Cargo.toml` desde la copia. Comprobar con una comparación de hash que son idénticos al original.
6. Decidir H250 a H253 y registrar el Resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian (temporalmente): `gba-recomp\crates\gba-core\src\ppu.rs`, `gba-recomp\crates\gba-core\Cargo.toml`, `blazing-recomp\Cargo.toml` (una entrada de ejemplo).
- Archivos nuevos y que se quedan: `examples/shim_afin_etapas.rs`, `volcados/afin-etapas-7500.txt`, `volcados/respaldo-ppu/` (copias).
- Archivos que no cambian: `src/*` del shim, pruebas, otros ejemplos, propuestas anteriores.
- Riesgo: la instrumentación cambia la compilación del crate `gba-core` (otro proyecto). Si hay otros trabajos abiertos en ese repositorio, pueden verse afectados; hay que comprobar el estado de git antes de empezar.

### 6. Suposiciones

- `gba-core` acepta una feature nueva sin romper su compilación → prototipar: `cargo build --features diag-afin` debe compilar → si es falsa: parar y revertir.
- Los contadores no cambian la salida de la PPU → prototipar: el control (PPU completa frente al emulador) debe dar 0 diferencias con la feature activa → si es falsa: parar y revertir.
- Restaurar desde la copia deja el repositorio igual que antes → prototipar: comparación de hash antes y después → si es falsa: parar y avisar.

### 7. Alcance

- Incluye: contadores por etapa para los afines 0 a 4 en 7500; control de la PPU; restauración verificada; H250 a H253; resultado.
- No incluye: corregir la PPU; implementar afines en el shim; cambios permanentes en `gba-core`; otros cuadros.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la instrumentación no cambia la salida de la PPU
Caso de uso: que la medición no altere lo que mide | Tipo: límite
  Dado la PPU con la feature `diag-afin` activa y la OAM completa
  Cuando se compara con el framebuffer del emulador
  Entonces hay 0 diferencias
Test: aserción en el ejemplo
Estado previo: falla (no hay instrumentación)

Escenario: se restaura `gba-core` tal como estaba
Caso de uso: no dejar cambios en otro repositorio | Tipo: límite
  Dado la copia de respaldo de `ppu.rs` y `Cargo.toml`
  Cuando se restauran y se comparan los hashes
  Entonces los hashes coinciden con los originales
Test: comparación de hash antes de empezar y después de restaurar
Estado previo: falla (no hay copia)

### 9. Cómo lo comprobaré

- Compilación con y sin la feature.
- Ejecución de la medición con `cargo run --release --features proto,diag-afin --example shim_afin_etapas -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 7500`, lanzado como job con salida a fichero.
- Hash de `ppu.rs` y `Cargo.toml` de `gba-core` antes y después de la restauración.

### 10. Terminado cuando

El informe existe, `gba-core` está restaurado (hashes iguales) y el Resultado dice si H250, H251, H252 o H253 se confirman para cada afín.

### 11. Parar si

- El control con la feature activa no da 0 diferencias: parar y restaurar.
- El repositorio `gba-core` tiene cambios no relacionados antes de empezar: parar y pedir instrucciones.
- La restauración no reproduce el hash original: parar y avisar, sin cambiar nada más.

---

## Resultado (lo completa el agente después de editar)

Aprobada por el usuario ("Aprobado"). **Parada antes de editar**, por la condición de parada de la propuesta ("el repositorio gba-core tiene cambios no relacionados antes de empezar").

Estado de `gba-recomp` (`git status`, guardado en `volcados/respaldo-ppu/git-status.txt`):
- Modificados sin commit: `.gitignore`, `crates/gba-core/Cargo.toml`, `apu.rs`, `capi.rs`, `machine.rs`, `mem.rs`, `ppu.rs`, `tests/perf_counters_tests.rs`.
- Sin seguimiento: `gba-recomp-launcher-0.4-x86_64-pc-windows-msvc/`.

Por qué paro: `ppu.rs` y `Cargo.toml` ya tienen cambios que no son de esta medición. Si instrumentara sobre ellos, mezclaría mi código con ese trabajo, y restaurar desde una copia destruiría esos cambios. No he modificado ningún archivo de `gba-recomp`.

Lo único creado: copias de respaldo en `volcados/respaldo-ppu/` (`ppu.rs`, `gba-core-Cargo.toml`, `hashes.txt`, `git-status.txt`, `cargo-leido.txt`, `firmas.txt`). Son copias del estado actual, no del original en `HEAD`.

Opciones para decidir:
1. Que el usuario guarde o confirme el trabajo pendiente de `gba-recomp` (commit o stash). Después se instrumenta en el árbol normal y se restaura con `git checkout`.
2. Hacer la instrumentación en una copia limpia de `HEAD` (por ejemplo, `git worktree` en una carpeta nueva dentro de `volcados`), sin tocar el árbol de trabajo. Requiere que `blazing-recomp` apunte a esa copia solo para esta medición. Es la opción que menos riesgo tiene.

Próximo paso propuesto (sin aprobar): elegir la opción 2 y escribir una propuesta nueva, o esperar a la decisión de la opción 1.

---

## Revisión humana

- Que se acepte la instrumentación temporal de `gba-core` para esta medición: ✔ | ✘ | ?
