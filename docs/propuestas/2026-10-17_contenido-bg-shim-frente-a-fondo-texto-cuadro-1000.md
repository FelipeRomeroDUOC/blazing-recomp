# Propuesta: Comparar el contenido de BG0–BG3 que muestra el shim con el de fondo_texto, cuadro 1000

Fecha: 2026-10-17
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `orden-de-pintado-del-shim-cuadro-1000` (verificada por agente) quedó que los 2.523 píxeles "ninguno" tienen un sprite ganador semitransparente como único sprite, y que el color X debe venir de un fondo o del fondo vacío tal como lo muestra el shim. Pero las hipótesis que usan los colores de `fondo_texto` (que coinciden con la PPU) dieron 0. Eso sugiere que el shim muestra, en algunos píxeles, colores de fondo distintos de los de la PPU.

Esta propuesta mide, por cada capa de fondo BG0 a BG3, el contenido que el shim muestra cuando esa capa es la única activa, y lo compara con `fondo_texto`. Se miden dos referencias:

- **Referencia de fin:** `fondo_texto` con los registros y la VRAM del final del cuadro. Es lo mismo que el shim lee.
- **Referencia por línea:** `fondo_texto` con los registros y la VRAM de cada línea, como lo ve la PPU.

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-17_orden-de-pintado-del-shim-cuadro-1000.md` (verificada por agente): el color X sale de un fondo; H50 a H52 dieron 0; el modelo del orden de pintado difiere del shim en 21.070 píxeles.
  - `docs/propuestas/2026-10-17_contenido-bg2-cuadro-1000.md` (verificada por agente): BG2 activo, 37.920 opacos en el cuadro; `fondo_texto` coincide con la PPU (control 0).
  - `src/ppu_shim.rs`, `decode_io_regs`: el shim lee DISPCNT, BGnCNT, HOFS y VOFS una vez por cuadro (estado de fin). No lee el estado por línea.
  - `src/ppu_shim.wgsl`, `fs_bg`: la base de mapa, la base de caracteres, 4bpp u 8bpp, el voltear, el scroll y los modos de tamaño (`screen_size`) se calculan igual que en `fondo_texto`, según la lectura del código. Esto no está medido.
  - Dato histórico de una sesión anterior: el shim con solo fondos (V3) difiere de la PPU en 37.690 de 38.400 píxeles. Sin separar por capa ni por referencia.
- Intención: saber si el shim muestra el mismo contenido que `fondo_texto` para cada capa, y si la diferencia viene del estado de fin (un solo juego de registros para todo el cuadro) o de otra causa.
- Hipótesis:
  - H70: para cada capa BG_k, el shim coincide con `fondo_texto` con referencia de fin en al menos el 99 % de los píxeles donde la referencia es opaca (±1). Si se confirma, el contenido del shim es correcto para el estado de fin, y la diferencia viene de los registros por línea.
  - H71: para cada capa BG_k, el shim coincide con la referencia por línea en menos del 99 % de los píxeles opacos. Si se confirma, el shim no reproduce los cambios de registros por línea.
  - H72: el shim muestra píxeles opacos donde la referencia es transparente, en más del 0,5 % de los píxeles de la capa. Si se confirma, hay un error de muestreo o de máscara en `fs_bg`, no solo de estado.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. El shim es lo que se mide.
  - El ROM solo en `roms/`. No sobrescribir ejemplos existentes; el resultado va a un archivo nuevo.
  - Se usa la misma tolerancia ±1 y la misma conversión de canal que en las propuestas anteriores.
  - Para medir cada capa sola, se pone en DISPCNT solo el bit de esa capa, sin OBJ ni ventanas. Esto se hace en una copia del registro; el juego no cambia.
- Alternativas descartadas:
  - Cambiar `fs_bg` para corregir el muestreo → descartada: primero hay que saber dónde está la diferencia.
  - Medir con la PPU solamente → descartada: el objetivo es el shim.

### 3. Ejemplos

- Caso normal: BG2 solo activo. En un píxel la referencia de fin da color C y el shim da C ±1: cuenta como coincidencia en H70.
- Caso límite: un píxel donde la referencia por línea da color C1 y la de fin da C2 (distintos), y el shim da C2. Cuenta en H70 y no en H71; así se ve si la diferencia es del estado.

### 4. Pasos

1. Crear `examples/shim_contenido_bg.rs` (con `required-features = ["proto"]`), copiando por código de `shim_sprite_debajo.rs`: `Gpu`, `fondo_texto`, `componer`, `rgb8`, `cerca` y la lectura de secuencia.
2. Control previo: reconstrucción V0 y V1 sobre toda la pantalla debe dar 0, y el control de paleta del fondo debe dar 0. Si no, parar.
3. Para cada capa BG_k, k en 0..3, con BG_k como única capa activa, sin OBJ ni ventanas: renderizar el shim con el estado de fin (`Gpu::renderiza`) y obtener su color por píxel.
4. Referencia de fin: `fondo_texto` con el io y la VRAM de fin para cada línea.
5. Referencia por línea: `fondo_texto` con el io y la VRAM de cada captura.
6. Para cada capa: contar píxeles opacos en la referencia; diferencias del shim frente a la referencia de fin (H70); diferencias frente a la referencia por línea (H71); píxeles donde el shim es opaco y la referencia es transparente (H72).
7. Contar cuántas líneas tienen registros de fondo distintos entre el estado de fin y el estado por línea (dato de contexto).
8. Escribir `volcados/contenido-bg-shim-cuadro-1000.txt` (archivo nuevo) con las líneas de la sección 8.
9. Crear `tests/contenido_bg_shim_tests.rs` con las pruebas de la sección 8.
10. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_contenido_bg.rs` (nuevo), `tests/contenido_bg_shim_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/contenido-bg-shim-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, `shim_sprite_debajo.rs`, el juego, la APK, y todos los informes anteriores.
- Riesgo: si el shim cambia de estado durante el cuadro (por ejemplo, con cambios de scroll por línea), la diferencia H71 mezclará efectos de estado y de muestreo. Por eso se reporta también el número de líneas con registros distintos (paso 7).

### 6. Suposiciones

- Con una sola capa activa, el shim pinta solo esa capa sobre el fondo vacío → prototipar: sí, con la referencia de fondo vacío en los píxeles transparentes → si es falsa: parar.
- `fondo_texto` coincide con la PPU en el estado por línea (control 0 en V0 y V1 con la composición completa) → prototipar: sí, el control del paso 2 → si es falsa: parar.
- Poner DISPCNT en la copia del registro es equivalente a un juego con esa configuración para el shim → prototipar: sí, el shim solo lee DISPCNT y los registros de capa → si es falsa: se reporta como limitación.

### 7. Alcance

- Incluye: contenido de BG0 a BG3 en el shim, frente a la referencia de fin y por línea; control; pruebas; resultado.
- No incluye: cambiar el shim o el shader; corregir el muestreo; sprites; ventanas; mezcla de la PPU; el residuo del 1,33 %; otros cuadros; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la reconstrucción coincide con la PPU
Caso de uso: que las comparaciones de fondo sean confiables | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0`, `control reconstruccion V1` y `control paleta fondo` valen 0
Test: `tests/contenido_bg_shim_tests.rs::test_reconstruccion_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: cada capa tiene su informe de coincidencia
Caso de uso: decidir H70, H71 y H72 | Tipo: normal
  Dado el informe
  Cuando se leen sus líneas
  Entonces existen `BG0 opacos`, `BG0 shim vs fin`, `BG0 shim vs linea`, `BG0 shim opaco sobre transparente` y las mismas líneas para BG1, BG2 y BG3
Test: `tests/contenido_bg_shim_tests.rs::test_capas_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da el número de líneas con registros distintos
Caso de uso: separar el efecto del estado por línea | Tipo: normal
  Dado el informe
  Cuando se lee `lineas con registros de fondo distintos`
  Entonces existe la línea y su valor está entre 0 y 160
Test: `tests/contenido_bg_shim_tests.rs::test_lineas_distintas_rango` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_contenido_bg -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/contenido-bg-shim-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test contenido_bg_shim_tests` → pasa si: las tres pruebas pasan.
- Regresión: `contenido-bg2-cuadro-1000.txt`, `sprite-debajo-cuadro-1000.txt` y `origen-x-cuadro-1000.txt` no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice, para cada capa, cuántos píxeles opacos coinciden con la referencia de fin y con la referencia por línea, cuántos píxeles son opacos en el shim donde la referencia es transparente, y si H70, H71 y H72 se confirman o se descartan.

### 11. Parar si

- El control de reconstrucción o de paleta no da 0: parar, porque la comparación no sería confiable.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Ejecución (2026-10-17): `shim_contenido_bg` escribió `volcados/contenido-bg-shim-cuadro-1000.txt`. El job terminó con código 1 por avisos de cargo; el archivo de salida confirma el resultado. Pruebas `contenido_bg_shim_tests`: 3 pasan, 0 fallan.

Controles: reconstrucción V0 = 0, V1 = 0; control paleta fondo = 0. Pasan.

Dato de contexto: 129 de 160 líneas tienen registros de fondo o DISPCNT distintos del estado de fin.

Resultados por capa (opaco del shim = color distinto del fondo vacío):
- BG0: opacos (fin) 0; opacos (línea) 0; shim vs fin 0; shim vs línea 0; shim opaco sobre transparente 710.
- BG1: opacos 0; opacos (línea) 0; shim vs fin 0; shim vs línea 0; shim opaco sobre transparente 710.
- BG2: opacos (fin) 37.920; opacos (línea) 37.920; shim vs fin 6.240 (16 %); shim vs línea 24.602 (65 %); shim opaco sobre transparente 0.
- BG3: opacos 0; shim vs fin 0; shim vs línea 0; shim opaco sobre transparente 710.

Hipótesis:
- H70 descartada: BG2 coincide con la referencia de fin en 84 % de los píxeles opacos, no 99 %.
- H71 confirmada: BG2 coincide con la referencia por línea solo en 35 %. El shim sigue el estado de fin del cuadro, no los registros por línea.
- H72 confirmada en BG0, BG1 y BG3 (710 de 38.400 píxeles, 1,8 %), pero con una salvedad: esas capas no tienen contenido en este cuadro (0 opacos de referencia) y el mismo número aparece en las tres. Probablemente no es un error de muestreo de cada capa. Queda sin explicar.
- H72 no se cumple en BG2 (0).

Lectura:
- La mayor parte de la diferencia de BG2 viene de que el shim usa un solo estado de registros por cuadro, y la PPU cambia registros por línea en 129 líneas.
- Aún queda un 16 % de diferencias frente al estado de fin. Eso no se explica por el estado por línea y necesita otra medición (muestreo o máscara en `fs_bg`).
- Las 710 diferencias en BG0, BG1 y BG3 sin contenido son una anomalía a revisar antes de sacar conclusiones de H72.

---

## Revisión humana

- Que la diferencia de contenido de fondo entre el shim y la PPU se acepte como explicación del color X, y que H70, H71 y H72 se confirmen o descarten: ✔ | ✘ | ?
