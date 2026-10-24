# Propuesta: Localizar por qué la PPU no pinta los 154 píxeles afines de 7500 (dirección del texel o composición)

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (lectura ejecutada; H240 y H241 descartadas, H242 abierta)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Solo lectura. No se modifica código ni se ejecuta ningún ejemplo nuevo. Se leen:

- `gba-recomp\crates\gba-core\src\ppu.rs`, función `render_sprites`, desde el cálculo de coordenadas de textura del camino afín (bloque "Texture coordinates", alrededor de las líneas 495 a 550) hasta el escritura de `ObjPixel`.
- `gba-recomp\crates\gba-core\src\ppu.rs`, composición (`compose`, `compose_straight`, `straight_obj`, `straight_span`), solo en lo que afecte a píxeles de sprite con `priority != 0xFF`.
- `blazing-recomp\examples\shim_afin_tiles.rs`, función `texel()`, para comparar la dirección.

Resultado: un informe en el propio Resultado de esta propuesta, con líneas citadas y una comparación tabla a tabla. Si hace falta medir, se propone aparte.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_fondos-tapan-afines-cuadro-7500.md` (discrepada): hay 154 píxeles no tapados por fondo, con texel no transparente y dentro de la textura, pero la PPU aporta 0.
  - `docs/propuestas/2026-10-18_pixeles-afines-cuadro-7500.md` (verificada): la PPU aporta 0 píxeles de los 5 afines.
  - `docs/propuestas/2026-10-18_tiles-afines-cuadro-7500.md` (verificada): los tiles no están vacíos.
  - `ppu.rs`, `render_sprites` (leído parcialmente): el camino afín calcula `u`, `v` con la matriz y usa `w`, `h` sin doblar; el camino normal usa otra dirección de tile.
- Intención: decidir entre dos causas, sin implementar nada:
  - (1) mi `texel()` usa la dirección de sprites normales, y la PPU usa otra para afines;
  - (2) la PPU calcula bien el texel, pero la composición descarta el píxel.
- Hipótesis:
  - H240: la dirección de tile del camino afín en la PPU difiere de `texel()`. Si se confirma, mi cálculo de los 154 píxeles no es el que pinta la PPU, y hay que corregir la medición (no la PPU).
  - H241: la dirección coincide, y la composición (por ejemplo, la vía recta o el reparto de capas) descarta los píxeles de sprite de estos afines. Si se confirma, el fallo está en la composición de la PPU, y el shim tendría que replicar esa regla.
  - H242: ninguna de las dos. Si se confirma, hay otra condición en `render_sprites` (por ejemplo, un `continue` previo) que no he identificado.
- Restricciones y decisiones previas:
  - Solo lectura: sin cambios en `gba-core`, `src/` ni ejemplos.
  - Se citan líneas exactas de `ppu.rs` para cada afirmación.
- Alternativas descartadas:
  - Añadir prints en la PPU para depurar → descartada en esta fase: primero leer el código, porque cambiar `gba-core` requiere otra propuesta.

### 3. Ejemplos

- Caso normal: en `render_sprites`, para el camino afín, el texel se calcula con `tile = tile_base + tv*row_tiles + tu*paso` (igual que el normal) → H240 descartada para ese punto.
- Caso límite: en la composición, un píxel con `priority` 1 y fondo opaco de prioridad 0 se descarta (o se acepta) según el reparto de capas → confirma o descarta H241.

### 4. Pasos

1. Leer `ppu.rs` desde el bloque "Texture coordinates" de `render_sprites` hasta el final del bucle de píxeles de sprite. Anotar la dirección de tile y cualquier condición que descarte píxeles.
2. Leer la composición (`compose`, `straight_obj`, `straight_span`) buscando cómo trata `ObjPixel` con prioridad 1 frente a fondos de prioridad 0 y 1, y si hay condiciones de ventana o de OBJ semitransparente.
3. Comparar la dirección de la PPU con `texel()` de `shim_afin_tiles.rs`, línea por línea.
4. Decidir H240, H241 y H242 con citas de línea.
5. Registrar el Resultado y, si H241 o H242 se confirman, proponer por separado qué medir.

### 5. Qué más podría verse afectado

- Archivos que cambian: solo esta propuesta (sección Resultado).
- Archivos que no cambian: todo el código, los ejemplos, los informes.
- Riesgo: el fichero de `gba-core` es grande (unas 89 KB); se lee por partes con límites de línea. Si la lectura falla, se registra en el Resultado.

### 6. Suposiciones

- `gba-recomp\crates\gba-core\src\ppu.rs` es la versión que usa `blazing-recomp` como dependencia → prototipar: comprobar `Cargo.toml` de `blazing-recomp` (ruta de `gba-core`) → si es falsa: parar y preguntar.
- La versión de `ppu.rs` no cambió desde las mediciones del cuadro 7500 → prototipar: la fecha de modificación (2026-10-17) es anterior a las medidas de hoy → si es falsa: parar.

### 7. Alcance

- Incluye: lectura de la dirección de texel del camino afín; lectura de la composición de sprites; comparación con `texel()`; H240 a H242; resultado.
- No incluye: cambios de código; nuevas mediciones; implementar afines en el shim.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: se compara la dirección de tile de la PPU con la del medidor
Caso de uso: decidir H240 | Tipo: normal
  Dado `render_sprites` en `ppu.rs`
  Cuando se lee el camino afín
  Entonces el Resultado cita las líneas y dice si la dirección coincide con `texel()`
Test: no se escribe prueba nueva
Estado previo: falla (no está decidido)

Escenario: se determina si la composición descarta los píxeles
Caso de uso: decidir H241 | Tipo: normal
  Dado la composición en `ppu.rs`
  Cuando se lee cómo trata prioridad 1 frente a fondos de prioridad 0
  Entonces el Resultado cita las líneas y dice si descarta o no
Test: no se escribe prueba nueva
Estado previo: falla

### 9. Cómo lo comprobaré

- Revisión de las líneas citadas.
- Comprobación de que la lista de archivos modificados solo contiene esta propuesta.

### 10. Terminado cuando

El Resultado dice si H240, H241 o H242 se confirman o descartan, con citas de línea.

### 11. Parar si

- No se encuentra la ruta de `gba-core`: parar y pedirla.
- La lectura de `ppu.rs` falla dos veces seguidas: parar y registrarlo en el Resultado.

---

## Resultado (lo completa el agente después de editar)

Aprobada por el usuario ("Aprobado"). Solo lectura; sin cambios en el repositorio. Leído: `gba-recomp\crates\gba-core\src\ppu.rs` líneas 415 a 512 (camino afin de `render_sprites`) y 512 a 575 (escritura de `ObjPixel`), 785 a 860 (`straight_obj`).

**Dirección del texel (H240):** descartada. En la PPU, para afines:
- `tile = tile_base + tv*row_tiles + tu*tile_step` (líneas 520-521), con `row_tiles` igual que en el caso normal (1D: `(w/8)*(2 si 8bpp)`; 2D: 32).
- `addr = 0x10000 + tile*32 + pv*4 + pu/2` en 4 bpp, o `0x10000 + (tile/2)*64 + pv*8 + pu` en 8 bpp (líneas 526-531).
- Esa es la misma fórmula que `texel()` de `shim_afin_tiles.rs`. Mi lectura de tiles era correcta.

**Prioridad frente a fondos (H241):** descartada como causa de la pérdida. En `straight_obj` (líneas 807-815), un sprite con `op <= bp` gana, y solo un fondo habilitado, opaco y con prioridad estrictamente menor lo tapa. Es la misma regla que uso en `shim_afin_tapado.rs`.

**Escritura de píxeles (líneas 546-562):** un píxel de sprite se guarda si su color de índice no es 0 y si gana por prioridad (`priority < out.priority`, o el punto aún está vacío). No hay un descarte adicional visible por modo afin. Ojo: el modo 2 (ventana de OBJ) sí se desvía, pero los 5 afines son modo 0.

**H242 (abierta):** la PPU debería pintar esos píxeles según el código leído y no los pinta. La causa no está en la dirección del texel ni en la regla de prioridad. Puede estar en una condición que no leí (algo entre las líneas 415 y 512 que no he visto completo, o en cómo se llena `obj_line` antes de `compose`), o en mi medición de la caja.

Próximo paso propuesto (sin aprobar): rastrear `render_sprites` para esos 5 sprites en el cuadro 7500 (contar en cada etapa cuántos texeles pasan: línea cubierta, rango de `min_bx..max_bx`, `u,v` en rango, `color_idx != 0`, y si escriben en `out`). Eso requiere una copia de la función en un ejemplo, o una instrumentación de `gba-core` en una propuesta aparte. Requiere propuesta y aprobación.

---

## Revisión humana

- Que la corrección (medición o composición de la PPU) se decida con este resultado: ✔ | ✘ | ?
