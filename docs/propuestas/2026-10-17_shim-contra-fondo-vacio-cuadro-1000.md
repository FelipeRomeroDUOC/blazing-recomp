# Propuesta: Qué color muestra el shim en los 5.290 píxeles de aportación del cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `shim-aportacion-cuadro-1000` quedó medido que el shim, con estado de fin o por línea, no coincide con la PPU en ninguno de los 5.290 píxeles de aportación, y que da el mismo color con OBJ que sin OBJ (H24, H25 y H26). Falta saber qué color muestra el shim en esos píxeles. Esta propuesta lo compara con dos referencias de la PPU: V3 (nada: fondo vacío) y V2 (solo OBJ).

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_shim-contra-aportacion-cuadro-1000.md` (verificada por agente): shim con 0 coincidencias en la aportación; control de cuadro completo 37,8 % (fin) y 85,6 % (línea); determinismo 0.
  - `docs/propuestas/2026-10-17_capa-ganadora-por-pixel-obj-cuadro-1000.md` (revisada por humano): en los 5.290, la PPU V2 y V3 coinciden, y el color del sprite ganador es igual al fondo vacío (H15).
  - `examples/shim_aportacion.rs`: ya renderiza el shim con V0 y V1, en estado de fin y por línea, y compara con la PPU V0 y V1 con tolerancia ±1.
- Intención: saber si el shim muestra el fondo vacío (V3) en los 5.290 píxeles, o un color que no es ni V0, ni V1, ni V3.
- Hipótesis:
  - H27: con estado de fin, el shim coincide con la PPU V3 en al menos el 90 % de los 5.290. Se confirma si el conteo es al menos 4.761.
  - H28: con estado por línea, el shim coincide con la PPU V3 en al menos el 90 % de los 5.290. Se confirma si el conteo es al menos 4.761.
  - H29: el shim muestra un color que no coincide con V0, V1 ni V3 en más del 50 % de los 5.290. Se confirma si el conteo de "otro" es mayor que 2.645.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia; el shim es lo que se mide.
  - El trabajo corre en el PC con Windows, con GPU. Si no hay adaptador de GPU, el ejemplo para.
  - El ROM solo en `roms/`. No sobrescribir `shim_aportacion.rs` ni `volcados/shim-aportacion-cuadro-1000.txt`.
- Alternativas descartadas:
  - Reemplazar `shim_aportacion.rs` → descartada: el informe anterior ya está verificado y sus pruebas dependen de él.
  - Medir el color del shim con un muestreo de píxeles sueltos → descartada: la comparación por máscara completa es más directa y reutiliza el control de arnés.

### 3. Ejemplos

- Caso normal: un píxel de la aportación con PPU V3 = `3062` y shim fin = `3062`. El informe cuenta ese píxel en "shim fin igual PPU V3".
- Caso límite: si el shim muestra un color que no coincide con V0, V1 ni V3, el píxel cuenta en "shim fin categoria otro" y el ejemplo no lo descarta.

### 4. Pasos

1. Extender `examples/shim_aportacion.rs` para renderizar el shim en V2 y V3, en estado de fin y por línea. No cambia lo que ya escribe.
2. Calcular la PPU V2 y V3 como referencias (ya están en el cálculo de la máscara).
3. Contar, para los 5.290 píxeles: shim fin igual PPU V3, shim linea igual PPU V3, y el resto como "otro" (usando las categorías de V0, V1 y V3).
4. Escribir el resultado en `volcados/shim-fondo-cuadro-1000.txt` (archivo nuevo; el informe anterior no cambia).
5. Crear `tests/shim_fondo_tests.rs` con las pruebas de la sección 8.
6. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_aportacion.rs` (se añaden renderizados V2 y V3 y un archivo de salida nuevo), `tests/shim_fondo_tests.rs` (nuevo), `volcados/shim-fondo-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu_frente_a_frente.rs`, `volcados/frente-a-frente.txt`, `volcados/shim-aportacion-cuadro-1000.txt`, `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, el juego, la APK.
- Riesgo: que la extensión cambie el informe anterior. El criterio 1 de la sección 8 lo detecta.

### 6. Suposiciones

- El arnés de comparación (tolerancia ±1 y control de cuadro completo) sigue siendo válido para V2 y V3 → prototipar: sí, con el mismo control de cuadro completo en V3 → si es falsa: parar.
- La PPU V3 es el fondo vacío en esos píxeles (H15 lo indica) → prototipar: sí, con el conteo de V3 igual a la PPU V2 en la máscara → si es falsa: se reporta sin concluir sobre el fondo.

### 7. Alcance

- Incluye: comparación del shim con V3 y V2 en los 5.290 píxeles, en estado de fin y por línea; categorías; pruebas; resultado.
- No incluye: otros cuadros; cambios al shim; ventanas, mezcla o el residuo del 1,33 %; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: el informe anterior no cambia
Caso de uso: que la extensión no altere lo verificado | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo con la extensión
  Entonces `volcados/shim-aportacion-cuadro-1000.txt` tiene los mismos conteos que antes
Test: `tests/shim_aportacion_tests.rs::test_shim_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: pasa (verificado el 2026-10-17)

Escenario: el informe da el color del shim frente a V3
Caso de uso: decidir H27, H28 y H29 | Tipo: normal
  Dado el archivo `volcados/shim-fondo-cuadro-1000.txt`
  Cuando se leen sus líneas
  Entonces existen `shim fin igual PPU V3`, `shim linea igual PPU V3` y `shim fin categoria otro`
Test: `tests/shim_fondo_tests.rs::test_shim_fondo_informe` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el archivo no existe)

Escenario: las categorías del shim suman el total
Caso de uso: que ningún píxel quede sin clasificar | Tipo: normal
  Dado el archivo `volcados/shim-fondo-cuadro-1000.txt`
  Cuando se suman las categorías del shim de fin de cuadro
  Entonces la suma es igual a `aportacion total`
Test: `tests/shim_fondo_tests.rs::test_shim_fondo_suma_total` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el archivo no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_aportacion -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existen `volcados/shim-aportacion-cuadro-1000.txt` y `volcados/shim-fondo-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test shim_fondo_tests` y `--test shim_aportacion_tests` → pasa si: todas las pruebas pasan.
- Regresión: `volcados/shim-aportacion-cuadro-1000.txt` mantiene sus conteos.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice si H27, H28 y H29 se confirman o se descartan, con sus conteos.

### 11. Parar si

- El control de cuadro completo en V3 es bajo (menos de 30 % de coincidencia): parar, porque el arnés no sería confiable para esa variante.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecutado 2026-10-17, cuadro 1000 (`volcados/shim-fondo-cuadro-1000.txt`):

Control de arnés: el shim con estado de fin coincide con la PPU V3 en 37.690 de 38.400 píxeles de la pantalla (98,2 %). El arnés funciona. Con estado por línea, el mismo número.

Aportación (5.290 píxeles):
- Shim fin igual a PPU V3: 0. Shim línea igual a PPU V3: 0.
- Shim fin igual a PPU V2 (solo OBJ, comparando cada uno con su propia referencia): 5.290. Shim línea igual a PPU V2: 5.290.
- Categorías del shim de fin: igual V0: 0; solo V1: 0; solo V3: 0; otro: 5.290.

Hipótesis:
- H27 (fin igual a V3 en al menos 90 %): descartada, 0 de 5.290.
- H28 (línea igual a V3 en al menos 90 %): descartada, 0 de 5.290.
- H29 (otro en más de la mitad): confirmada, 5.290 de 5.290.

Lectura: en estos píxeles el shim muestra un color propio (lo llamo X) que no es el fondo vacío ni el color de BG2 de la PPU. Con solo OBJ (V2), el shim sí da el fondo vacío, igual que la PPU. Con los fondos encendidos (V0 y V1), el shim da X en ambos casos, así que ignora el sprite en estos píxeles. Por H15, la PPU con OBJ muestra el color del fondo vacío porque el sprite lo tiene; por eso el shim (que muestra X) no coincide con la PPU.

Limitación: la igualdad "PPU V0 = PPU V3" en estos píxeles viene de H15 (color del sprite igual al del fondo vacío), no de una medición directa en este informe. Lo que falta es identificar X: qué capa de fondo lo produce y por qué el shim no dibuja el sprite ahí.

Pruebas: las dos de `shim_fondo_tests` y las tres de `shim_aportacion_tests` pasan.

---

## Revisión humana

- Que el color que muestra el shim en los 5.290 píxeles se acepte como base para decidir la corrección, y que H27, H28 y H29 se confirmen o descarten: ✔ | ✘ | ?
