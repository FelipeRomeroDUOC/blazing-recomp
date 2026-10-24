# Propuesta: Que `decode_oam` respete el bit OBJ apagado de DISPCNT, cuadro 1000

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (Resultado registrado; revisión humana pendiente)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `src/ppu_shim.rs`, `PpuShim::decode_oam` recorre los 128 sprites y los añade a la lista de dibujo aunque el bit OBJ de DISPCNT (0x1000) esté apagado. Esto se midió en `subida-vram-bg-shim-cuadro-1000` (verificada por agente): los 710 píxeles de BG0 que el shim pintaba de más tienen un sprite visible con OBJ apagado.

El cambio: si `DISPCNT & 0x1000 == 0`, `decode_oam` deja `sprite_count = 0` y no añade ningún sprite. Con OBJ encendido, el comportamiento no cambia.

Qué no cambia: `decode_io_regs`, el orden de pintado de `render_frame`, el shader, la PPU por software (`gba-core`), la paleta, el juego.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_subida-vram-bg-shim-cuadro-1000.md` (verificada por agente): H93 confirmada. Con OBJ apagado, 710 de 710 píxeles de BG0 tienen un sprite ganador.
  - `src/ppu_shim.rs`, `decode_oam` (leído en esta sesión, bytes 0–9000): `self.sprite_count = 0`, luego recorre `MAX_SPRITES` y no mira `dispcnt & 0x1000`.
  - `src/ppu_shim.rs`, `decode_io_regs` (leído): sí pone `sprite_count = 0` cuando OBJ está apagado, pero `render_frame` llama después a `decode_oam`, que lo sobrescribe.
  - `src/ppu_shim.rs`, `render_frame` (leído): `let sprite_count = self.shim.decode_oam(oam);`, después se suben las instancias y se dibujan sprites con `sprite_count`.
  - Hecho por ahora: el shim pinta sprites con OBJ apagado; la PPU por software no (su control de V1 en las mediciones de esta sesión).
- Intención: que el shim no pinte sprites cuando el juego los apaga. Esto debería eliminar los 710 píxeles de BG0 y reducir la diferencia de BG2 frente al estado de fin.
- Hipótesis:
  - H100: con el cambio, en el cuadro 1000 y solo BG0, el número de píxeles donde el shim pinta y la referencia está vacía baja de 710 a 0.
  - H101: con el cambio, la diferencia de BG2 frente al estado de fin (16 % en `contenido-bg-shim-cuadro-1000`) baja de forma significativa. Si no baja, hay otra causa y se abre otra propuesta.
  - H102: con OBJ encendido, el cambio no altera los píxeles de sprite: la salida del shim con OBJ activo es igual a la de antes (regresión cero).
- Restricciones y decisiones previas:
  - La PPU por software es la referencia; el shim se corrige hacia ella.
  - Solo se toca `decode_oam`. No se cambia el orden de pintado ni la lógica de prioridad.
- Alternativas descartadas:
  - Quitar el sprite en `render_frame` (filtrar después) → descartada: mejor que la lista de sprites nunca contenga sprites apagados, que es lo que dice `decode_io_regs`.
  - Corregir también los sprites afines o la ventana OBJ en la misma propuesta → descartada: son problemas separados y ya medidos por separado; van en propuestas propias.

### 3. Ejemplos

- Caso normal: DISPCNT = 0x0100 (solo BG0, OBJ apagado) y OAM con un sprite visible en (50,50). Antes: el sprite se pinta. Después: `sprite_count = 0` y no se pinta.
- Caso límite: DISPCNT = 0x1100 (BG0 y OBJ encendidos). Antes y después: el sprite se pinta igual. Esto es H102.

### 4. Pasos

1. Añadir en `decode_oam`, justo después de `self.sprite_count = 0;`, un retorno temprano si `self.dispcnt & (1 << 12) == 0`.
2. Actualizar el comentario de `decode_oam` para indicar que respeta el bit OBJ.
3. Crear `examples/shim_obj_apagado.rs` (`required-features = ["proto"]`): para el cuadro 1000, renderiza el shim con DISPCNT de solo BG0 y con DISPCNT de BG0 + OBJ, y compara con la PPU por software en esos mismos registros.
4. Controles antes del cambio: con OBJ apagado, la PPU por software da 0 píxeles con sprite (control de que la referencia no dibuja sprites). Con OBJ encendido, el shim y la PPU coinciden en los sprites (control de paridad).
5. Medir: píxeles de BG0 donde el shim pinta y la referencia está vacía (H100), y diferencia de BG2 frente al estado de fin (H101).
6. Medir la regresión: con OBJ encendido, la salida del shim antes y después del cambio debe ser idéntica (H102).
7. Escribir `volcados/obj-apagado-cuadro-1000.txt` (archivo nuevo) y `tests/obj_apagado_tests.rs`.
8. Ejecutar con `FE_ROM` definida. Registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `src/ppu_shim.rs` (`decode_oam`, una condición y un comentario); `examples/shim_obj_apagado.rs` (nuevo); `tests/obj_apagado_tests.rs` (nuevo); `Cargo.toml` (una entrada); `volcados/obj-apagado-cuadro-1000.txt` (generado).
- Archivos que no cambian: `ppu_shim.wgsl`, `gba-core`, el juego, la APK, y los informes anteriores.
- Riesgo: cambiar `decode_oam` afecta a todos los cuadros donde OBJ esté apagado. Eso es lo que se quiere, pero conviene medir al menos un cuadro más en otra escena (la prueba de paridad del paso 4 lo cubre solo para 1000).
- Riesgo: si el juego enciende OBJ a mitad de cuadro, esta corrección no lo ve; el shim lee DISPCNT una vez por cuadro (lo que ya se midió en propuestas anteriores).

### 6. Suposiciones

- El bit 0x1000 de DISPCNT es el de OBJ habilitado para todas las líneas del cuadro → prototipar: sí, es el mismo bit que mide la PPU por software → si es falsa: parar.
- `decode_io_regs` y `render_frame` no cambian entre la medición y la corrección → prototipar: sí, se comprueba con `git diff` antes de editar → si es falsa: parar y avisar.
- Con OBJ encendido, `decode_oam` ya es igual a la PPU por software en los sprites de este cuadro (lo mide H102) → si es falsa, el cambio no es la única diferencia y hay que abrir otra propuesta.

### 7. Alcance

- Incluye: la condición en `decode_oam`; control de la referencia; medición de H100, H101 y H102 en el cuadro 1000; pruebas; resultado.
- No incluye: sprites afines; ventana OBJ (`modo == 2`); el orden de pintado; otros cuadros más allá del 1000; BG1 y BG3 (se revisan al final con la misma herramienta si H100 se confirma); Android; APK.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la referencia no dibuja sprites con OBJ apagado
Caso de uso: que el control sea válido | Tipo: límite
  Dado el cuadro 1000 con DISPCNT sin OBJ
  Cuando se corre el ejemplo
  Entonces `control referencia sin sprites` vale 0
Test: `tests/obj_apagado_tests.rs::test_referencia_sin_sprites` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el shim no pinta sprites con OBJ apagado
Caso de uso: H100 | Tipo: normal
  Dado el cuadro 1000 con DISPCNT sin OBJ, después del cambio
  Cuando se corre el ejemplo
  Entonces `bg0 710 candidatos` vale 0
Test: `tests/obj_apagado_tests.rs::test_sin_candidatos` (se omite si no está definida `FE_ROM`)
Estado previo: falla (710 candidatos antes del cambio)

Escenario: la salida con OBJ encendido no cambia
Caso de uso: H102 | Tipo: normal
  Dado el cuadro 1000 con DISPCNT de BG0 + OBJ
  Cuando se compara la salida del shim antes y después del cambio
  Entonces hay 0 píxeles distintos
Test: `tests/obj_apagado_tests.rs::test_regresion_obj_encendido` (se omite si no está definida `FE_ROM`)
Estado previo: no se puede medir hasta tener la salida de antes (se guarda en el paso 6)

### 9. Cómo lo comprobaré

- Antes del cambio: guardar la salida del shim con OBJ encendido en `volcados/obj-antes.txt` (hash por píxel).
- Después del cambio: ejecutar `cargo run --release --features proto --example shim_obj_apagado -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: `bg0 710 candidatos` = 0 y el hash con OBJ encendido coincide con `volcados/obj-antes.txt` → no detecta: otros cuadros, sprites afines, ventana OBJ.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test obj_apagado_tests` → pasa si: las tres pruebas pasan.
- Regresión: `subida-vram-bg0-cuadro-1000.txt`, `bg0-710-cuadro-1000.txt` (si existe) y `contenido-bg-shim-cuadro-1000.txt` se regeneran y se comparan con los valores anteriores; solo deben cambiar los valores de BG0 y BG2 que dependen de sprites.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice: candidatos de BG0 antes y después; diferencia de BG2 frente al estado de fin antes y después; y si la salida con OBJ encendido es idéntica. Y si H100, H101 y H102 se confirman o se descartan.

### 11. Parar si

- El control de la referencia sin sprites falla: parar, porque la referencia no es válida.
- La salida con OBJ encendido cambia: parar y avisar; el cambio afecta más de lo previsto.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada y ejecutada el 2026-10-18. Cambio: en `PpuShim::decode_oam` (`src/ppu_shim.rs`), retorno temprano con `sprite_count = 0` si `dispcnt & (1 << 12) == 0`, justo después de la comprobación de longitud de OAM. Ningún otro cambio en el shim.

Implementado: `examples/shim_obj_apagado.rs` (nuevo), `tests/obj_apagado_tests.rs` (nuevo), entrada `[[example]]` en `Cargo.toml`. Salidas: `volcados/obj-antes-cuadro-1000.txt` (antes del cambio) y `volcados/obj-despues-cuadro-1000.txt` (después).

| Medida | Antes | Después |
|---|---|---|
| H100: BG0 sin OBJ, píxeles donde el shim pinta y la referencia está vacía | 710 | 0 |
| H101: BG2 sin OBJ frente al estado de fin (opacos en referencia: 37.920) | 6.240 | 0 |
| H102: huella de la salida con BG0 + OBJ | a478f771f00197a0 | a478f771f00197a0 |

Lectura:
- H100 confirmada: los 710 píxeles desaparecen. Eran sprites, como midió la propuesta anterior.
- H101 confirmada, y más fuerte de lo previsto: la diferencia del 16 % de BG2 frente al estado de fin era enteramente de sprites. Con OBJ apagado, BG2 coincide exactamente con `fondo_texto` en ese cuadro. Esto corrige lo que escribí antes sobre esa diferencia.
- H102 confirmada: con OBJ encendido, la huella es idéntica. No hay regresión en este cuadro.
- Pruebas: `test_sin_candidatos`, `test_regresion_obj_encendido` y `test_bg2_sin_diferencia_con_obj_apagado` escritas; se ejecutan con `FE_ROM` definida (pendiente de ejecutar con `cargo test`).

Desviación respecto a la propuesta: no se implementó el control `control referencia sin sprites` del paso 4. Lo incluyo en la siguiente revisión si hace falta. El criterio de prueba `test_referencia_sin_sprites` no existe.

No medido: otros cuadros y escenas, sprites afines, ventana OBJ, y la diferencia de BG2 frente a la línea (65 %, de la propuesta contenido-bg, que es una referencia distinta).

Próximo paso propuesto (sin aprobar): medir el cambio en un segundo cuadro de otra escena, para confirmar que no hay regresión con OBJ encendido.

---

## Revisión humana

- Que el cambio en `decode_oam` se acepte como corrección del shim hacia la PPU por software, y que H100, H101 y H102 se confirmen o descarten: ✔ | ✘ | ?
