# Propuesta: Localizar los 710 píxeles de BG0 que el shim pinta sin contenido en la referencia, cuadro 1000

Fecha: 2026-10-18
Nivel: 2
Estado: discrepada (parada por criterio de la réplica; ver Resultado)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `contenido-bg-shim-frente-a-fondo-texto-cuadro-1000` (verificada por agente) quedó que, con solo BG0 activa, el shim pinta 710 píxeles opacos donde `fondo_texto` no tiene contenido (0 opacos de referencia). Lo mismo aparece en BG1 y BG3 (710 cada una). No está claro si es la misma causa en las tres capas.

Esta propuesta localiza esos 710 píxeles de BG0 y compara, para cada uno, la entrada de mapa, el tile y el índice de color que usa el shim (replicando `fs_bg`) con los que usa `fondo_texto`.

No se cambia `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl` ni el juego. Es solo medición.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_contenido-bg-shim-frente-a-fondo-texto-cuadro-1000.md` (verificada por agente): BG0, BG1 y BG3 con 0 opacos de referencia y 710 opacos en el shim; BG2 con 16 % de diferencias frente al estado de fin.
  - `src/ppu_shim.wgsl`, `fs_bg`: entrada de mapa `(screen_base + sbb_offset) * 2048 + (local_ty * 32 + local_tx) * 2`; tile con `char_base * 16384`; coordenadas `px = screen_pos.x + scroll.x` con `screen_pos` en el centro del píxel (x + 0.5); color 0 descartado.
  - `examples/shim_contenido_bg.rs`: ya renderiza el shim con una sola capa activa y compara con `fondo_texto`.
- Intención: saber si los 710 píxeles salen de una región concreta del mapa, y si el shim lee una entrada, un tile o una base distinta de la que lee `fondo_texto`.
- Hipótesis:
  - H80: en al menos 90 % de los 710 (639 píxeles), el shim y `fondo_texto` leen la misma entrada de mapa, y `fondo_texto` da color 0 (transparente). Si se confirma, el shim pinta un tile que `fondo_texto` ve como transparente: problema de lectura del tile o de la paleta.
  - H81: en al menos 90 % de los 710, la entrada de mapa del shim cae en un bloque (SBB) o una fila distinta de la de `fondo_texto`. Si se confirma, el problema es de selección de bloque de mapa o del offset de SBB.
  - H82: en al menos 90 % de los 710, la base de caracteres (`char_base`) o de mapa (`screen_base`) que usa el shim es distinta de la de `fondo_texto`. Si se confirma, el problema es en la decodificación de BGnCNT.
- Restricciones y decisiones previas:
  - La PPU por software es la referencia. El shim es lo que se mide.
  - El ROM solo en `roms/`. No sobrescribir ejemplos existentes; el resultado va a un archivo nuevo.
  - Se usa la misma tolerancia ±1 y la misma conversión de canal que en las propuestas anteriores.
- Alternativas descartadas:
  - Cambiar `fs_bg` ahora → descartada: primero hay que saber qué lee distinto el shim.
  - Medir solo BG1 o BG3 → descartada por ahora: BG0 es la capa con el caso más simple (una sola capa activa); si se explica, las otras dos se revisan con la misma herramienta.

### 3. Ejemplos

- Caso normal: un píxel donde el shim muestra un color C. La entrada de mapa del shim es `E1`, y la de `fondo_texto` es `E1`, pero el tile de `fondo_texto` da índice 0 en ese píxel. Cuenta en H80.
- Caso límite: un píxel donde el shim usa `E1` y `fondo_texto` usa `E2` (otro bloque de mapa). Cuenta en H81, no en H80.

### 4. Pasos

1. Crear `examples/shim_bg0_710.rs` (con `required-features = ["proto"]`), copiando por código de `shim_contenido_bg.rs`: `Gpu`, `fondo_texto`, `rgb8`, `cerca`, `leer_secuencia`.
2. Control previo: reconstrucción V0 y V1 sobre toda la pantalla debe dar 0, control de paleta del fondo debe dar 0, y la cuenta de píxeles con BG0 sola (shim opaco donde la referencia es transparente) debe dar 710. Si no, parar.
3. Renderizar el shim con solo BG0 activa (sin OBJ) y obtener sus píxeles.
4. Listar los 710 píxeles (shim opaco, referencia transparente).
5. Para cada uno, calcular con la fórmula de `fs_bg` (replicada en Rust, con el mismo `screen_size`, `sbb_offset`, `char_base` y `screen_base`): la dirección de entrada de mapa, el tile, el índice de color y el color.
6. Calcular lo mismo con la fórmula de `fondo_texto`.
7. Contar: misma entrada de mapa (H80 y H81), misma base de caracteres y de mapa (H82), y si el color del shim coincide con el color de algún índice del tile.
8. Escribir `volcados/bg0-710-cuadro-1000.txt` (archivo nuevo) con las líneas de la sección 8.
9. Crear `tests/bg0_710_tests.rs` con las pruebas de la sección 8.
10. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_bg0_710.rs` (nuevo), `tests/bg0_710_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/bg0-710-cuadro-1000.txt` (generado, no versionado).
- Archivos que no cambian: `ppu.rs`, `gba-core`, `ppu_shim.rs`, `ppu_shim.wgsl`, `shim_contenido_bg.rs`, el juego, la APK, y todos los informes anteriores.
- Riesgo: la réplica en Rust de `fs_bg` puede no coincidir con el shader (por ejemplo, en el centro del píxel). El control del paso 2 solo mide el total de 710; por eso también se verifica que la réplica reproduce el color del shim en esos 710 (si no, parar).

### 6. Suposiciones

- La réplica de `fs_bg` en Rust es fiel al shader actual (leído en esta sesión) → prototipar: sí, y se comprueba que reproduce el color del shim en los 710 → si es falsa: parar y avisar.
- La cuenta de 710 se reproduce con la misma configuración del cuadro 1000 → prototipar: sí, es el control del paso 2 → si es falsa: parar.
- Los tres casos (BG0, BG1 y BG3) comparten causa → prototipar: no se asume; se mide solo BG0 en esta propuesta.

### 7. Alcance

- Incluye: localizar y clasificar los 710 píxeles de BG0; comparar entrada, tile, base y color con `fondo_texto`; control; pruebas; resultado.
- No incluye: cambiar el shim o el shader; BG1 y BG3; BG2 (16 % de diferencias); el estado por línea; sprites; ventanas; otros cuadros; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: la reconstrucción coincide con la PPU y la cuenta de 710 se reproduce
Caso de uso: que las comparaciones sean confiables | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0`, `control reconstruccion V1` y `control paleta fondo` valen 0, y `bg0 shim opaco sobre transparente` vale 710
Test: `tests/bg0_710_tests.rs::test_reconstruccion_coincide` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: cada uno de los 710 píxeles queda clasificado
Caso de uso: decidir H80, H81 y H82 | Tipo: normal
  Dado el informe
  Cuando se leen `bg0 710 misma entrada`, `bg0 710 entrada distinta`, `bg0 710 misma base` y `bg0 710 base distinta`
  Entonces `bg0 710 misma entrada` + `bg0 710 entrada distinta` = 710
Test: `tests/bg0_710_tests.rs::test_710_clasificados` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da la causa
Caso de uso: decidir qué lee distinto el shim | Tipo: normal
  Dado el informe
  Cuando se leen sus líneas
  Entonces existen `bg0 710 color coincide tile`, `bg0 710 replica fs_bg` y todas las líneas de la sección 8
Test: `tests/bg0_710_tests.rs::test_causa_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_bg0_710 -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/bg0-710-cuadro-1000.txt` con todas las líneas de la sección 8 → no detecta: BG1, BG3, otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test bg0_710_tests` → pasa si: las tres pruebas pasan.
- Regresión: `contenido-bg-shim-cuadro-1000.txt`, `sprite-debajo-cuadro-1000.txt` y `origen-x-cuadro-1000.txt` no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice cuántos de los 710 píxeles tienen la misma entrada de mapa, la misma base de caracteres y de mapa, y si el color del shim coincide con un índice de tile; y si H80, H81 y H82 se confirman o se descartan.

### 11. Parar si

- El control de reconstrucción, de paleta o la cuenta de 710 no coinciden: parar, porque la comparación no sería confiable.
- La réplica de `fs_bg` no reproduce el color del shim en los 710: parar y avisar, porque el modelo no es fiel.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada el 2026-10-18. Implementado: `examples/shim_bg0_710.rs`, entrada `[[example]]` en `Cargo.toml`. No se creó `tests/bg0_710_tests.rs` porque el informe no se escribió (ver abajo).

Ejecución: `shim_bg0_710` con `roms\1575 - Fire Emblem (E)(Independent).gba`, `secuencias\intro.txt`, cuadro 1000. Salida en `bg0-710-output.txt` (proceso con código 1).

- Controles: reconstrucción V0 = 0, V1 = 0, paleta de fondo = 0. Pasan.
- Candidatos (shim opaco, referencia transparente): 710. Coincide con lo esperado.
- Réplica de `fs_bg` (con `PpuShim::decode_io_regs`): reproduce el color del shim en **0 de 710**. Da transparente en **710 de 710**.
- Entrada de mapa, base de caracteres y base de mapa: la réplica coincide con los bits de BG0CNT en los 710 píxeles (no hay entrada distinta).

Criterio de parada aplicado: la réplica no reproduce el color del shim, así que no se escribió el informe de causa (`volcados/bg0-710-cuadro-1000.txt`). H80, H81 y H82 quedan sin evaluar.

Lectura provisional (no confirmada): en esas direcciones de mapa y tile, la réplica lee transparente sobre la misma VRAM que usa la referencia, mientras el shim pinta color. La diferencia está entre los datos de VRAM que se pasan a la GPU (subida de `bg_vram_buffer` en `render_frame`, o empaquetado de bytes) y el snapshot de VRAM del cuadro, no en la aritmética de dirección. Hay que leer `render_frame` para confirmar. Hay también un supuesto sin verificar: que los uniformes `screen_base` y `char_base` lleven números de bloque (como `BgLayerConfig`), y no direcciones en bytes.

Próximo paso propuesto (aún sin aprobar): leer la subida de VRAM en `render_frame` y comparar lo que recibe la GPU con el snapshot en esos 710 píxeles.

---

## Revisión humana

- Que la causa de los 710 píxeles de BG0 se acepte como error de lectura del shim, y que H80, H81 y H82 se confirmen o descarten: ✔ | ✘ | ?
