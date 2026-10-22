# Propuesta: Fase 1.3 — Soporte de Sprites 8bpp (256 colores) en GPU PpuShim

Fecha: 2026-10-16
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## 1. Contexto y Diagnóstico Empírico

En la prueba comparativa A/B realizada entre la build con Shims GPU y la build con software forzado (`FORZAR_PPU_SOFTWARE = true`):
1. **En software puro (CPU):** El usuario confirmó que *"todo se vio correcto"* (logo de Intelligent Systems nítido, sin columnas partidas ni franjas).
2. **En la telemetría del logo (`t=3s a 5s`):**
   ```text
   # ppu_hw gpu=59 sw=0 modo0=0 win=53 blend=59 spr_total=128 spr_8bpp=18 spr_afin=1
   # ppu_hw gpu=59 sw=0 modo0=0 win=59 blend=59 spr_total=128 spr_8bpp=19 spr_afin=1
   ```
   Se detectaron **19 sprites activos en modo 8bpp** (256 colores).

### Causa Raíz
En [`src/ppu_shim.rs`](../../src/ppu_shim.rs), la función `unpack_obj_tiles_4bpp` trata **toda la VRAM de sprites (32 KB)** ciegamente como si fueran tiles de 4bpp:
- Cada byte de VRAM se divide en dos nibbles (`byte & 0x0F` y `byte >> 4`), asumiendo que contiene dos píxeles de 16 colores.
- Cuando un sprite opera en modo **8bpp**, cada byte de la VRAM representa **1 píxel completo con un índice de 0 a 255**.
- Al ser procesado por el desempaquetador actual, el valor de 8 bits se divide a la mitad: los colores se truncan a 0..15 y la imagen se duplica horizontalmente a la mitad de ancho.
- En el shader [`src/ppu_shim.wgsl`](../../src/ppu_shim.wgsl), la rama `if ((in.flags & 16u) != 0u)` ya existe para sumar el offset de 256 colores, pero recibe píxeles corruptos desde el atlas.

---

## 2. Objetivos del Cambio

1. **Soporte de Tiles OBJ 8bpp:** Permitir que los sprites con el bit `attr0 & 0x2000` activo decodifiquen sus píxeles en el rango completo de 0..255.
2. **Direccionamiento de Tiles en 8bpp:**
   - En GBA, en modo 8bpp los tiles ocupan 64 bytes (el doble de un tile de 4bpp).
   - En mapeo 1D (`mapping_1d`), cada tile de 8x8 consume 2 índices de tile de la numeración 4bpp: `tile_offset = (base_tile & !1) + (ty * tw + tx) * 2`.
   - En mapeo 2D, cada fila consume 2 índices horizontales.
3. **Cero impacto en rendimiento:** Mantener el presupuesto de 60 FPS sin alocaciones dinámicas.

---

## 3. Alternativas Técnicas Evaluadas

| Opción | Descripción | Pros | Contras | Decisión |
| :--- | :--- | :--- | :--- | :--- |
| **A. Desempaquetado dual (Atlas 4bpp + Atlas 8bpp)** | Generar dos texturas de 64 KB en CPU cada frame. | No altera el shader actual. | Duplica el trabajo de CPU por frame y el ancho de banda de subida a GPU. | Descartada |
| **B. VRAM OBJ directa en GPU Texture/Buffer + Decodificación en Shader** | Subir los 32 KB de VRAM cruda directamente a la GPU; el fragment shader decodifica si lee 4bpp (nibble) u 8bpp (byte) según `in.flags`. | Elimina el bucle de desempaquetado en CPU (ahorra ~1.5 ms de CPU); la GPU hace la lectura exacta. | Requiere modificar el muestreo en WGSL. | **Recomendada** |
| **C. Desempaquetado inteligente por sprite en CPU** | La CPU recorre la OAM y solo desempaqueta en el atlas los tiles con su formato respectivo. | Menor cambio en el shader. | Complejidad por solapamiento si un mismo tile es compartido en diferentes modos. | Descartada |

### Enfoque Arquitectónico Elegido (Opción B)
1. En [`src/ppu_shim.rs`](../../src/ppu_shim.rs):
   - Subir la VRAM de sprites (32 KB crudos de `0x10000..0x18000`) directamente a una textura/búfer GPU `R8Uint` o `Texture2D`.
   - En el fragment shader de [`src/ppu_shim.wgsl`](../../src/ppu_shim.wgsl):
     - Si `is_8bpp == false` (4bpp): leer el byte de VRAM en `tile_offset * 32 + row * 4 + col / 2` y extraer el nibble alto/bajo.
     - Si `is_8bpp == true` (8bpp): leer el byte directo en `(tile_offset & !1) * 32 + (ty * tw + tx) * 64 + row * 8 + col`.
2. Con esto:
   - **Se elimina por completo el bucle `unpack_obj_tiles_4bpp` de la CPU.**
   - La CPU ahorra ~1.5 ms por frame.
   - Pasa a decodificación exacta idéntica al hardware de GBA.

---

## 4. Criterios de Aceptación (Dado / Cuando / Entonces)

- **Criterio 1: Desempaquetado correcto de píxeles 8bpp**
  - **Dado** un sprite configurado con `attr0` bit 13 activo (8bpp) y bytes de VRAM con valores `[0x00, 0x42, 0xFF, 0x10]`,
  - **Cuando** el pipeline renderiza el sprite,
  - **Entonces** el fragment shader extrae exactamente los índices `0` (transparente), `66`, `255` y `16`, indexando la paleta OBJ en `256 + índice`.

- **Criterio 2: Logo de Intelligent Systems sin columnas rotas**
  - **Dado** la escena del logo en `t=3s a 5s` donde hay 19 sprites de 8bpp,
  - **Cuando** corre en la GPU,
  - **Entonces** las letras y el logo se muestran continuos, sólidos y con su paleta original de 256 colores.

- **Criterio 3: Integridad de Sprites 4bpp**
  - **Dado** los sprites regulares del juego (cursores, unidades en el mapa) que usan 4bpp,
  - **Cuando** se renderizan junto a sprites de 8bpp,
  - **Entonces** conservan su apariencia y subpaletas idénticas a la ejecución por software.

---

## 5. Estrategia de Tests Previos (`test-generation`)

Antes de modificar el código de producción o el shader, agregaremos a [`src/ppu_shim_tests.rs`](../../src/ppu_shim_tests.rs):
1. `test_decodificacion_sprite_8bpp_indices_completos`: Test que valida que un sprite de 8bpp no trunca sus valores a nibbles de 4 bits y que el cálculo del `tile_offset` avanza de 64 en 64 bytes.
2. `test_ppu_shim_shader_validates`: Asegurar que el shader con la decodificación directa de VRAM compile y valide con `naga`.

---

## 6. Fuera de Alcance

- Las ventanas de hardware (`WIN0`/`WIN1`) se abordarán en la siguiente fase (Fase 1.4).
- La mezcla alfa y efectos de brillo/oscuridad (`BLDCNT`) se abordarán en la Fase 1.5.
