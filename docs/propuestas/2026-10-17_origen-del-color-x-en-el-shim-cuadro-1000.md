# Propuesta: Medir de dónde sale el color X del shim en los 5.290 píxeles de aportación del cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se sabe que el color X del shim no coincide con ningún fondo de texto (BG0–BG3) en los 5.290 píxeles (`identificar-color-x-del-shim-cuadro-1000`, verificada por agente). Falta saber de dónde sale X. Esta propuesta mide tres candidatos concretos, todos sacados del código del shim:

- **Candidato A: mezcla de alfa del GPU.** En `src/ppu_shim.wgsl`, `fs_sprite` pone `color.a = 0.5` en sprites semitransparentes, y el pipeline usa `wgpu::BlendState::ALPHA_BLENDING`. Eso mezcla el sprite con lo que hay debajo al 50 %. La PPU, en cambio, aplica los coeficientes EVA/EVB de BLDALPHA (`ppu.rs`, `alpha_blend`). Si X es esa mezcla, el error viene de la mezcla, no de la paleta.
- **Candidato B: color de fondo vacío (backdrop).** X es el color de `palette[0]`, es decir, lo que queda cuando el shim no dibuja nada en ese píxel.
- **Candidato C: otra entrada de paleta.** X es un color `palette[k]` con `k` distinto del índice que la PPU usa para el sprite (`256 + palette_idx*16 + raw` en 4bpp).

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_identificar-color-x-del-shim-cuadro-1000.md` (verificada por agente): X sin fondo en los 5.290; prioridad sin empates.
  - `docs/propuestas/2026-10-17_capa-debajo-del-sprite-cuadro-1000.md` (verificada por agente): la capa debajo del sprite en V1 es BG2, opaca en los 5.290.
  - `docs/propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md` (revisada por humano): el sprite gana en V0 en los 5.290.
  - `src/ppu_shim.wgsl`: `fs_sprite` (`color.a = 0.5` si `flags & 4`), `fs_bg`, `palette_data` (índices 256..511 para OBJ).
  - `src/ppu_shim.rs`: `unpack_palette` convierte cada canal de 5 bits con `/31` a `f32`; `sprite_pipeline` y `bg_pipeline` usan `ALPHA_BLENDING`.
- Intención: para cada uno de los 5.290 píxeles, saber si X sale de A, B o C.
- Hipótesis:
  - H40: en al menos 4.761 píxeles (90 %), X es la mezcla 50/50 del sprite ganador con BG2, con tolerancia ±1 por canal. Si se confirma, el origen es el alfa del GPU.
  - H41: en al menos 4.761 píxeles, X es igual al color de `palette[0]` (backdrop) con tolerancia ±1 por canal. Si se confirma, el shim no dibuja el sprite en esos píxeles.
  - H42: en al menos 4.761 píxeles, X es igual a `palette[k]` con `k` distinto del índice esperado del sprite. Si se confirma, la paleta del sprite se indexa mal en el shim.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. El shim es lo que se mide.
  - El ROM solo en `roms/`. No sobrescribir ejemplos existentes; el resultado va a un archivo nuevo.
  - La conversión de paleta es la misma que en el arnés (`(v*255+15)/31`), para que las comparaciones sean justas.
- Alternativas descartadas:
  - Cambiar `ppu_shim.wgsl` para probar la mezcla correcta → descartada: primero hay que saber si el candidato A es el origen.
  - Medir solo con la PPU → descartada: X es un valor del shim; hay que leerlo del shim.

### 3. Ejemplos

- Caso normal: un píxel donde el sprite ganador es semitransparente, con color S, y BG2 es B. Si X = (S+B)/2 por canal (±1), el píxel cuenta en H40.
- Caso límite: un píxel donde X es igual a `palette[0]`. Cuenta en H41 aunque el sprite sea opaco, y se reporta aparte.

### 4. Pasos

1. Crear `examples/shim_origen_x.rs` (con `required-features = ["proto"]`), copiando por código lo necesario de `examples/shim_capa_x.rs`: `Gpu`, `fondo_texto`, `sprites` (con `semi` y los índices de paleta), `separa`, `junta` y `componer`.
2. Control previo: reconstrucción V0 y V1 sobre toda la pantalla debe dar 0. Si no, parar.
3. Para los 5.290 píxeles, leer X del shim (estado de fin, como en `shim-fondo`).
4. Por píxel, calcular: el sprite ganador (color, si es semitransparente, índice de paleta esperado), BG2 (capa debajo), y `palette[0]`.
5. Contar H40, H41 y H42 por separado, y los píxeles que no entran en ninguno.
6. Para H42, buscar `k` en 0..511 con `palette[k]` igual a X y listar cuántos casos hay por `k`.
7. Escribir `volcados/origen-x-cuadro-1000.txt` (archivo nuevo) con las líneas de la sección 8.
8. Crear `tests/origen_x_tests.rs` con las pruebas de la sección 8.
9. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_origen_x.rs` (nuevo), `tests/origen_x_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/origen-x-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, `shim_capa_x.rs`, `ppu_capa_ganadora.rs`, el juego, la APK, y todos los informes anteriores.
- Riesgo: si X no coincide con A, B ni C, el resultado es "origen no identificado"; no se descarta nada, y habrá que medir la escritura de la paleta en el GPU.

### 6. Suposiciones

- El shader y el pipeline son los del código actual (`ppu_shim.wgsl` modificado el 2026-10-16) → prototipar: sí, leyendo el archivo antes de correr → si es falsa: parar.
- La paleta que sube el shim es la misma que lee la PPU (`unpack_palette` de los mismos bytes) → prototipar: sí, comparando `palette[0]` del shim con la PPU en el control → si es falsa: parar.
- La mezcla 50/50 del GPU se calcula en el espacio de valores de `Rgba8Unorm` (sin conversión sRGB) → prototipar: sí, con la tolerancia ±1 → si es falsa: H40 se evalúa con tolerancia mayor y se marca como "dudosa".

### 7. Alcance

- Incluye: medir A, B y C en los 5.290 píxeles; control; pruebas; resultado.
- No incluye: cambiar el shim; corregir el shader; otros cuadros; ventanas, mezcla de la PPU o el residuo del 1,33 %; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la reconstrucción coincide con la PPU
Caso de uso: que las comparaciones con X sean confiables | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0` y `control reconstruccion V1` valen 0
Test: `tests/origen_x_tests.rs::test_reconstruccion_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: cada píxel se asigna a un solo candidato o a ninguno
Caso de uso: decidir H40, H41 y H42 | Tipo: normal
  Dado el informe
  Cuando se suman los conteos de H40, H41, H42 y "ninguno"
  Entonces la suma es igual a `aportacion total`
Test: `tests/origen_x_tests.rs::test_origen_suma_total` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da los conteos de cada candidato
Caso de uso: decidir qué origen tiene X | Tipo: normal
  Dado el informe
  Cuando se leen sus líneas
  Entonces existen `origen A mezcla`, `origen B backdrop`, `origen C otra paleta` y `origen ninguno`
Test: `tests/origen_x_tests.rs::test_origen_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_origen_x -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/origen-x-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test origen_x_tests` → pasa si: las tres pruebas pasan.
- Regresión: `identificar-x-cuadro-1000.txt`, `capa-ganadora-cuadro-1000.txt` y `shim-fondo-cuadro-1000.txt` no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice cuántos de los 5.290 píxeles caen en A, B, C o ninguno, y si H40, H41 y H42 se confirman o se descartan.

### 11. Parar si

- El control de la reconstrucción no da 0 en V0 y V1: parar, porque la comparación no sería confiable.
- El `palette[0]` del shim no coincide con la PPU: parar y avisar.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecución (2026-10-17): `shim_origen_x` escribió `volcados/origen-x-cuadro-1000.txt`. El job terminó con código 1 por avisos de cargo; el archivo de salida confirma el resultado. Pruebas `origen_x_tests`: 3 pasan, 0 fallan.

Controles: reconstrucción V0 = 0, V1 = 0; control paleta fondo = 0. Pasan.

Conteos (aportación total 5.290):
- origen A mezcla: 2.767 (52 %). Todos son con sprite semitransparente (A con sprite semi: 2.767).
- origen B backdrop: 0.
- origen C otra paleta: 0.
- origen ninguno: 2.523.
- igual sprite: 0.
- suma: 5.290.

Hipótesis:
- H40 descartada: 2.767 de 5.290 (52 %) son mezcla 50/50 con BG2. Se pedía al menos 4.761 (90 %). La mezcla explica una parte, no todo.
- H41 descartada: ningún píxel es igual al fondo vacío.
- H42 descartada: ningún píxel es igual a otra entrada de la paleta.

Lectura:
- El color X no es un color de la paleta en ningún píxel del grupo "ninguno" (2.523). Es un color mezclado.
- El candidato A usa BG2 como capa debajo. Los 2.523 píxeles pueden tener otra capa debajo (BG0, BG1 o BG3), o mezclar dos veces. No está medido.
- Limitación: el control de paleta cubre solo `palette[0]`. Para C se usó la paleta final del shim.

Siguiente paso: medir qué capa hay debajo de los 2.523 píxeles de "ninguno" y si su sprite es semitransparente. Requiere propuesta nueva.

---

## Revisión humana

- Que el origen de X (A, B, C o ninguno) se acepte como explicación del error del shim en estos píxeles: ✔ | ✘ | ?
