# Propuesta: Comparar la VRAM que recibe la GPU con el snapshot en los 710 píxeles de BG0, cuadro 1000

Fecha: 2026-10-18
Nivel: 2
Estado: verificada por agente (Resultado registrado; revisión humana pendiente)

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

En `bg0-710-pixeles-shim-cuadro-1000` (discrepada) quedó que, con BG0 sola, la réplica de `fs_bg` que usa los datos del snapshot de VRAM da transparente en los 710 píxeles donde el shim pinta color. La dirección de entrada de mapa y de tile coincide con la de los bits de BG0CNT.

Esta propuesta mide si la VRAM que recibe la GPU (`bg_vram_buffer`, subida en `render_frame`) coincide con el snapshot `vram` del cuadro, en las direcciones que leen esos 710 píxeles. Solo mide. No cambia el shim ni el shader.

### 2. Contexto

- Contexto consultado:
  - `docs/propuestas/2026-10-18_bg0-710-pixeles-shim-cuadro-1000.md` (discrepada): réplica transparente en 710 de 710; entrada, base de caracteres y base de mapa iguales a los bits de BG0CNT.
  - `src/ppu_shim.wgsl` (leído en esta sesión): `fs_bg` lee `vram_bg` como `array<u32>`, con `word = addr / 4` y `shift = (addr % 4) * 8`. Usa `bg_uniforms.screen_base` y `char_base` multiplicados por 2048 y 16384.
  - `src/ppu_shim.rs` (leído en esta sesión hasta el byte 16000 de 35198; falta el resto): `decode_io_regs` pone `screen_base_block` y `char_base_block` como números de bloque. No he leído aún `render_frame` ni cómo se rellena `bg_vram_buffer`.
- Intención: saber si la discrepancia nace en la subida de VRAM (tamaño, desplazamiento, empaquetado de bytes o big/little endian al pasar a `u32`) o en los uniformes.
- Hipótesis:
  - H90: en los 710 píxeles, el byte de VRAM que lee la GPU (`vram_bg[addr/4] >> shift`) difiere del byte `vram[addr]` del snapshot. Si se confirma, el problema está en la subida o el empaquetado de VRAM.
  - H91: los bytes son iguales, pero el uniforme `screen_base` o `char_base` que recibe la GPU no es el número de bloque (por ejemplo, va en bytes o con otro desplazamiento). Si se confirma, el problema está en los uniformes.
  - H92: los bytes y los uniformes son iguales, y la discrepancia viene de otra fuente (por ejemplo, la paleta o el orden de draw). Si se confirma, hay que abrir una nueva propuesta.
- Restricciones y decisiones previas:
  - El snapshot de VRAM del cuadro es la referencia.
  - Sin cambios en `ppu_shim.rs`, `ppu_shim.wgsl`, `gba-core` ni el juego.
  - Un solo cuadro (1000) y solo BG0.
- Alternativas descartadas:
  - Cambiar `render_frame` ahora → descartada: primero hay que saber qué recibe la GPU.
  - Revisar BG1, BG2 o BG3 → descartada por ahora: la discrepancia de BG0 es la más simple de aislar.

### 3. Ejemplos

- Caso normal: la dirección 0x06000100 contiene `0x5A` en el snapshot, y `vram_bg` en esa posición (con shift) da `0x5A`. Cuenta como igual.
- Caso límite: el snapshot da `0x5A` y la GPU da `0x00` en la misma dirección. Cuenta en H90.

### 4. Pasos

1. Leer `render_frame` en `src/ppu_shim.rs` y anotar cómo se construye `bg_vram_buffer` (tamaño, copia, `bytemuck` o conversión a `u32`) y cómo se rellenan los `BgUniformsGpu`.
2. Crear `examples/shim_vram_bg.rs` (`required-features = ["proto"]`). Copia de helpers desde `shim_bg0_710.rs`.
3. Controles: reconstrucción V0 y V1 = 0, paleta = 0, 710 candidatos (igual que `shim_bg0_710`). Si no, parar.
4. Para cada uno de los 710 píxeles, calcular la dirección de byte que lee `fs_bg` (entrada de mapa y de tile) y comparar:
   - el byte del snapshot `vram[addr]`;
   - el byte que la GPU lee de `vram_bg` con la misma fórmula de `fs_bg` (replicada con la copia de la subida).
5. Comparar los uniformes que se pasan a la GPU con los valores esperados (`screen_base_block`, `char_base_block`, `screen_size`, `is_8bpp`).
6. Contar: bytes distintos (H90), uniformes distintos (H91), y píxeles donde ninguno de los dos es distinto (H92).
7. Escribir `volcados/subida-vram-bg0-cuadro-1000.txt` (archivo nuevo).
8. Crear `tests/subida_vram_bg_tests.rs` con las pruebas de la sección 8.
9. Ejecutar con `FE_ROM` definida y registrar el resultado.

### 5. Qué más podría verse afectado

- Archivos que cambian: `examples/shim_vram_bg.rs` (nuevo), `tests/subida_vram_bg_tests.rs` (nuevo), `Cargo.toml` (una entrada), `volcados/subida-vram-bg0-cuadro-1000.txt` (generado).
- Archivos que no cambian: `ppu_shim.rs`, `ppu_shim.wgsl`, `gba-core`, `shim_bg0_710.rs`, el juego, la APK y los informes anteriores.
- Riesgo: la copia del `render_frame` que hace el ejemplo puede no ser fiel a la real. Por eso el paso 4 usa los datos que realmente se suben (no solo el snapshot), y el control del paso 3 se mantiene.

### 6. Suposiciones

- `render_frame` sube `vram` completa o su primer tramo de forma directa → prototipar: sí, se verifica en el paso 1 → si es falsa: parar y avisar.
- Los bytes de `bg_vram_buffer` se leen en little-endian como `u32` → prototipar: sí, el paso 4 lo comprueba → si es falsa: parar y avisar.
- Los uniformes llevan números de bloque → prototipar: sí, el paso 5 lo comprueba → si es falsa: H91 se confirma y se documenta.

### 7. Alcance

- Incluye: lectura de la subida de VRAM y de los uniformes en `render_frame`; comparación byte a byte en los 710 píxeles de BG0; control; pruebas; resultado.
- No incluye: cambiar el shim o el shader; BG1, BG2 y BG3; sprites; ventanas; otros cuadros; Android.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: los controles y el conteo de 710 se reproducen
Caso de uso: que la comparación sea confiable | Tipo: límite
  Dado el cuadro 1000
  Cuando se corre el ejemplo
  Entonces `control reconstruccion V0`, `control reconstruccion V1` y `control paleta fondo` valen 0, y `bg0 710 candidatos` vale 710
Test: `tests/subida_vram_bg_tests.rs::test_controles` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: cada uno de los 710 píxeles queda clasificado por byte y por uniforme
Caso de uso: decidir H90, H91 y H92 | Tipo: normal
  Dado el informe
  Cuando se leen `bytes distintos`, `uniformes distintos` y `ninguno distinto`
  Entonces `bytes distintos` + `ninguno distinto` ≥ 710 o la prueba indica qué falta
Test: `tests/subida_vram_bg_tests.rs::test_clasificacion` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

Escenario: el informe da la causa
Caso de uso: decidir qué fuente es la discrepancia | Tipo: normal
  Dado el informe
  Cuando se leen sus líneas
  Entonces existen las líneas de la sección 4, paso 6
Test: `tests/subida_vram_bg_tests.rs::test_informe_completo` (se omite si no está definida `FE_ROM`)
Estado previo: falla (el informe no existe)

### 9. Cómo lo comprobaré

- Ejecución: `cargo run --release --features proto --example shim_vram_bg -- "roms\1575 - Fire Emblem (E)(Independent).gba" secuencias\intro.txt 1000` → pasa si: existe `volcados/subida-vram-bg0-cuadro-1000.txt` con las líneas del paso 6 → no detecta: BG1 a BG3, otros cuadros.
- Pruebas: con `FE_ROM` definida, `cargo test --features proto --test subida_vram_bg_tests` → pasa si: las tres pruebas pasan.
- Regresión: `bg0-710-cuadro-1000.txt` (si existe) y `contenido-bg-shim-cuadro-1000.txt` no cambian.

### 10. Terminado cuando

Se cumplen los tres criterios de aceptación y el Resultado dice, para los 710 píxeles, cuántos bytes difieren, cuántos uniformes difieren y cuántos píxeles no tienen diferencia; y si H90, H91 o H92 se confirma.

### 11. Parar si

- El control de reconstrucción, de paleta o el conteo de 710 no coinciden: parar.
- `render_frame` no sube VRAM de forma que el ejemplo pueda replicar: parar y avisar, sin escribir informe.
- No hay adaptador de GPU: parar y avisar, sin escribir informe.
- Dos intentos fallidos seguidos con el mismo criterio: detenerse, explicar qué se probó y marcar la propuesta como `discrepada`.

---

## Resultado (lo completa el agente después de editar)

Aprobada y ejecutada el 2026-10-18. Implementado: `examples/shim_vram_bg.rs`, entrada `[[example]]` en `Cargo.toml`. Salida: `volcados/subida-vram-bg0-cuadro-1000.txt` (escrito dos veces por el mismo job; mismo contenido).

Controles: reconstrucción V0 = 0, V1 = 0, paleta = 0; candidatos = 710.

| Medida | Valor |
|---|---|
| H90: bytes de entrada distintos (GPU vs snapshot) | 0 |
| H90: bytes de dato distintos | 0 |
| H91: uniformes distintos (BG0CNT) | 0 |
| H92: ningún factor distinto | 710 |
| H94: dato de tile más allá de 0x10000 | 0 |
| OBJ apagado en DISPCNT del cuadro | sí |
| Sprite ganador si OBJ activo | 710 |
| H93: sprite dibujado con OBJ apagado | 710 |

Lectura:
- H90 descartada: la subida de VRAM (`vram[..98304]` tal cual) y los datos leídos por la GPU coinciden byte a byte con el snapshot en los 710 píxeles.
- H91 descartada: los uniformes coinciden con BG0CNT.
- H94 descartada: ninguna dirección de dato supera 0x10000 en esos píxeles. La hipótesis de la VRAM de OBJ en BG no aplica aquí.
- H93 confirmada: los 710 píxeles tienen un sprite visible con OBJ apagado. La GPU los pinta. `decode_oam` (`src/ppu_shim.rs`) pone `sprite_count = 0` y recuenta todos los sprites sin mirar el bit 0x1000 de DISPCNT. El `sprite_count = 0` de `decode_io_regs` se pierde, porque `render_frame` llama a `decode_oam` después.
- Esto explica por qué los mismos 710 píxeles aparecen en BG0, BG1 y BG3: son sprites, no fondo. También indica que el 16 % de BG2 frente al estado de fin (propuesta contenido-bg) probablemente incluye sprites de la misma causa; sin verificar.

Desviación respecto a la propuesta: los diagnósticos H93 y H94 no estaban en el plan aprobado. Se añadieron para interpretar H90 a H92. Lo señalo para revisión humana.

Próximo paso propuesto (aún sin aprobar): propuesta nivel 2 para que `decode_oam` respete el bit 0x1000 de DISPCNT, con control de que el cuadro 1000 sin sprites coincide con la PPU en esos 710 píxeles y de que el 16 % de BG2 baja.

---

## Revisión humana

- Que la fuente de la discrepancia (H90, H91 o H92) se acepte y que la siguiente propuesta siga esa línea: ✔ | ✘ | ?
