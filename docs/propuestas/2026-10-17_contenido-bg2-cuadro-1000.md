# Propuesta: Medir el contenido visible de BG2 en el cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `capa-debajo-del-sprite-cuadro-1000` quedó medido que, al quitar OBJ, los 5.290 píxeles de aportación muestran el fondo 2 (BG2). Falta saber si BG2 tiene contenido visible en el cuadro 1000 y dónde está. El informe de contenido de BG (`ppu_contenido_bg`) midió BG0, BG1 y BG3, pero no BG2.

No se cambia `ppu.rs`, `gba-core`, el shim ni el juego. Solo se mide.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_capa-debajo-del-sprite-cuadro-1000.md` (Resultado): fondo 2 en los 5.290 píxeles.
  - `examples/ppu_contenido_bg.rs`: mide tiles con datos, entradas de mapa, píxeles opacos y colores para BG0, BG1 y BG3. Ya existe en disco, pero no es necesariamente la versión verificada: hay que leerlo antes de tocarlo.
- Intención: saber si BG2 es visible en el cuadro 1000, cuántos píxeles opacos tiene y si los 5.290 píxeles están dentro de su zona visible.
- Hipótesis:
  - H21: BG2 tiene píxeles opacos fuera de los 5.290 en el cuadro 1000. Se confirma si el conteo de opacos fuera de esos píxeles es mayor que 0.
  - H22: los 5.290 píxeles caen dentro de la zona donde BG2 es opaco. Se confirma si todos los 5.290 tienen BG2 opaco en la PPU.
  - H23: BG2 tiene pocos tiles con datos y el contenido visible es escaso. Se confirma si el número de tiles con datos es menor que 32.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia; no se cambia.
  - No sobrescribir `examples/ppu_contenido_bg.rs` sin leerlo antes (ya pasó con otros ejemplos).
  - El trabajo corre en el PC con Windows; el ROM solo en `roms/`.
- Alternativas descartadas:
  - Crear un ejemplo nuevo solo para BG2 → descartada: duplica la lectura de tiles y mapa.
  - Ampliar la reconstrucción de `ppu_capa_ganadora` con BG2 → descartada: su control ya es 0 y mezclar mediciones complicaría el informe verificado.

### 3. Ejemplos

- Caso normal: BG2 tiene 40 tiles con datos, 3.000 píxeles opacos en el cuadro, y los 5.290 píxeles de aportación están dentro de la zona de opacos. El informe lo muestra como `bg2 opacos total: 3000` y `bg2 opacos dentro aportacion: 5290`.
- Caso límite: si BG2 no tiene píxeles opacos, el informe debe decir `bg2 opacos total: 0`, y la hipótesis H21 queda descartada sin más pasos.

### 4. Pasos

1. Leer `examples/ppu_contenido_bg.rs` completo antes de editarlo.
2. Añadir la medición de BG2 al mismo ejemplo, sin cambiar lo que ya mide para BG0, BG1 y BG3.
3. Contar: tiles con datos en BG2, entradas de mapa usadas, píxeles opacos totales, y cuántos de los 5.290 píxeles de aportación son opacos en BG2 en la PPU.
4. Escribir el resultado en `volcados/contenido-bg2-cuadro-1000.txt` (archivo aparte; el informe anterior no cambia).
5. Añadir pruebas en `tests/localizar_pixeles_tests.rs` (o un archivo nuevo) con los criterios de la sección 8.
6. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/ppu_contenido_bg.rs` (se añade BG2), pruebas nuevas, `volcados/contenido-bg2-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `gba-core`, `ppu.rs`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego, la APK, `volcados/capa-ganadora-cuadro-1000.txt`, `volcados/capa-debajo-cuadro-1000.txt`.
- Riesgo: que el cambio en `ppu_contenido_bg.rs` altere BG0, BG1 o BG3. El criterio 2 lo detecta.

### 6. Suposiciones

- Los conteos de BG2 se hacen con la misma lectura de VRAM que usa la PPU para fondos (solo los primeros 64 KB) → prototipar: sí, comparando con la PPU en los 5.290 píxeles → si es falsa: parar.
- Los 5.290 píxeles son los mismos que en la propuesta anterior → prototipar: sí, reutilizando su criterio de selección (V0 distinto de V1 y V2 igual a V3) → si es falsa: parar.

### 7. Alcance

- Incluye: medición de BG2 (tiles, mapa, opacos, dentro de los 5.290); pruebas; resultado.
- No incluye: BG0, BG1 ni BG3 (ya medidos); otros cuadros; cambiar la PPU, el shim o el juego; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: BG0, BG1 y BG3 no cambian
Caso de uso: que la medición nueva no altere lo verificado | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo con BG2 añadido
  Entonces los conteos de BG0, BG1 y BG3 son iguales a los del informe anterior
Test: `tests/localizar_pixeles_tests.rs::test_bg_tiene_tres_capas` (se omite si no está definida `FE_ROM`)
Estado previo: pasa (existe; verificar al ejecutar)

Escenario: BG2 tiene contenido medido
Caso de uso: decidir H21, H22 y H23 | Tipo: normal
  Dado el archivo `volcados/contenido-bg2-cuadro-1000.txt`
  Cuando se leen sus líneas
  Entonces existen `bg2 tiles con datos`, `bg2 opacos total` y `bg2 opacos dentro aportacion`
Test: `tests/localizar_pixeles_tests.rs::test_bg2_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el archivo no existe)

Escenario: los 5.290 píxeles son opacos en BG2 según la PPU
Caso de uso: confirmar H22 con la PPU como referencia | Tipo: normal
  Dado el archivo `volcados/contenido-bg2-cuadro-1000.txt`
  Cuando se lee `bg2 opacos dentro aportacion`
  Entonces su valor es 5290
Test: `tests/localizar_pixeles_tests.rs::test_bg2_cubre_aportacion` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el archivo no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example ppu_contenido_bg -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/contenido-bg2-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test localizar_pixeles_tests` → pasa si: las pruebas de la sección 8 pasan.
- Regresión: `volcados/contenido-bg-cuadro-1000.txt` mantiene los mismos conteos para BG0, BG1 y BG3.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice si H21, H22 y H23 se confirman o se descartan, con sus conteos.

### 11. Parar si

- La medición de BG2 no coincide con la PPU en los 5.290 píxeles: parar, porque el conteo no sería confiable.
- Cambiar `ppu_contenido_bg.rs` altera BG0, BG1 o BG3: parar y corregir antes de seguir.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecutado 2026-10-17, cuadro 1000 (`volcados/contenido-bg2-cuadro-1000.txt`):

- BG2 activo: sí (cnt 0x0e03).
- Tiles con datos (0..1023): 734.
- Tiles referenciados con datos en pantalla: 601.
- Píxeles opacos de BG2 en todo el cuadro: 37.920.
- Opacos dentro de la aportación de 5.290: 5.290 (todos).
- Opacos fuera de la aportación: 32.630.

Hipótesis:
- H21 (opacos fuera de los 5.290): confirmada, 32.630.
- H22 (los 5.290 caen en la zona opaca de BG2): confirmada, 5.290 de 5.290.
- H23 (menos de 32 tiles con datos): descartada, hay 734.

Pruebas: las cinco de `localizar_pixeles_tests` pasan (incluidas `test_bg2_informe_completo` y `test_bg2_cubre_aportacion`).

Limitación del criterio 1 (BG0, BG1 y BG3 sin cambio): el informe anterior `contenido-bg-cuadro-1000.txt` se sobrescribió en esta corrida y no hay copia para comparar línea a línea. Lo que sí se verificó: el código que mide BG0, BG1 y BG3 no se tocó. En `main` se añadió `rom.clone()` y, después de ese bucle, el bloque de BG2 con su propia salida. La comparación exacta queda pendiente si se quiere cerrar ese criterio.

---

## Revisión humana

- Que el contenido de BG2 medido sea suficiente para decidir H21, H22 y H23, y que el resultado se acepte como base para la siguiente propuesta: ✔ | ✘ | ?
