# Propuesta: Composición Jerárquica Bottom-Up Definitiva en GPU (Shader WGSL)

Fecha: 2026-10-31
Estado: verificada por agente
Nivel: 2 (Crítico / Shader WGSL, PPU Nativa y Pipeline)
Subsistemas afectados: `src/ppu_nativa.wgsl`, `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs`, `src/pipeline.rs`, `src/pipeline_tests.rs`

---

## 1. Resumen Ejecutivo

Esta propuesta implanta la **composición jerárquica *bottom-up* definitiva en GPU** en `src/ppu_nativa.wgsl`, erradicando por completo el parpadeo de capas durante la cinemática del dragón de *Fire Emblem: The Blazing Blade*.

### La Causa Raíz Descubierta
En la propuesta anterior, el shader WGSL aún dependía del bucle clásico de 2 objetivos de la GBA (`for (var p: u32 = 0u; p < 4u; p = p + 1u)`). Cuando aparecía el texto (`BG0`) o el cursor parpadeante (`OBJ`), el shader tomaba el texto o cursor como `primera` capa; luego, al evaluar `BG2` (el dragón), lo asignaba como `segunda` capa y ejecutaba inmediatamente `corte = true; break;`.
Esto provocaba que `BG3` (la llamarada de fuego y el cielo rojizo dinámico) **nunca llegara a leerse ni componerse** en los píxeles donde coincidían texto y dragón. Cada vez que una letra se dibujaba o el cursor titilaba, el fondo detrás de esos píxeles alternaba bruscamente entre fuego mezclado y dragón opaco, produciendo un parpadeo persistente.

### La Solución Definitiva
1. **Composición Bottom-Up Pura en GPU (`src/ppu_nativa.wgsl`):**
   Cuando `es_escena_dragon` está activo (`(cfg_comp.dispcnt & 0x0E07u) == 0x0C00u && (cfg_comp.bldcnt != 0u)`), el shader sustituye el bucle de 2 capas por una tubería jerárquica directa de 4 niveles que respeta el recorte de ventanas (`control`):
   - **Nivel 1 (Cielo/Llamarada):** Base `Backdrop`. Si `BG3` está habilitado por ventana (`(control & 8u) != 0u`) y no es transparente (`c3 != 0xFFFFu`), se mezcla directamente con el Backdrop usando `alpha_blend(c3, backdrop, eva_fuego, evb_fuego)` (con pesos nominales de respaldo `16` y `8` si `eva == 0`).
   - **Nivel 2 (Dragón/Montañas):** Si `BG2` está habilitado por ventana (`(control & 4u) != 0u`) y no es transparente (`c2 != 0xFFFFu`), sobreescribe de forma opaca el cielo, preservando el fuego donde el dragón es transparente.
   - **Nivel 3 (Subtítulos y Diálogo):** Si `BG0` está habilitado por ventana (`(control & 1u) != 0u`) y no es transparente (`c0 != 0xFFFFu`), se mezcla suavemente con el fondo acumulado (Dragón/Fuego) si el modo alfa está activo para `BG0`, o se dibuja nítido.
   - **Nivel 4 (Sprites y Cursor OBJ):** Si `OBJ` es visible (`(control & 0x10u) != 0u`), se compone en la cima con alfa si es semitransparente o de forma opaca, sin interrumpir jamás la lectura ni el renderizado de las capas inferiores.
2. **Preservación 100% de la Tubería Estándar GBA:** Fuera de la cinemática del dragón (y en las pinturas estáticas del prólogo donde `bldcnt == 0x0000`), el shader ejecuta el bucle estándar de 2 objetivos de la GBA, garantizando cero regresiones en menús, combates, mapas e ilustraciones.

---

## 2. Contexto

- **Intención:** Superar de raíz la limitación de 2 capas de la GBA en la cinemática del dragón mediante renderizado moderno en GPU que componga todas las capas en orden ascendente (*painters algorithm* con blending acumulativo), manteniendo 60 FPS estables y garantizando fidelidad total en el resto del juego.
- **Restricciones:**
  - *Restricción 1 (Inocuidad en el 100% del juego ordinario y prólogo):* La guarda `(cfg_comp.dispcnt & 0x0E07u) == 0x0C00u && (cfg_comp.bldcnt != 0u)` debe conservarse intacta para que las pinturas fijas de "El Batir" (`bldcnt == 0x0000`), los mapas tácticos (`BG1` activo), las animaciones de batalla y los menús no sufran ninguna alteración. Origen: Integridad visual del catálogo y prevención de falsos positivos en telemetría → Implica cero regresiones.
  - *Restricción 2 (Presupuesto móvil a 60 FPS):* La composición jerárquica no debe añadir pases de renderizado ni lecturas adicionales de memoria; se ejecuta en el mismo fragment shader en una sola pasada. Origen: `AGENTS.md` §1 → Implica coste de tiempo $\approx 0\text{ ms}$.
  - *Restricción 3 (Respeto estricto del sistema de ventanas):* Cada nivel de la tubería bottom-up debe comprobar la máscara de `control` por píxel (`WIN0`, `WIN1`, `OBJWIN`, `WINOUT`) para no pintar capas fuera de sus regiones delimitadas. Origen: Arquitectura PPU GBA → Implica enmascaramiento con `control & (1u << capa)`.
- **Alternativas Descartadas:**
  - *Descartada 1:* Intentar encajar 3 o 4 capas dentro del bucle de 2 objetivos de GBATek. Rechazada porque el bucle está diseñado matemáticamente para detenerse al encontrar la segunda capa, rompiéndose cada vez que entra una tercera (como texto o sprites).
  - *Descartada 2:* Retirar la condición `bldcnt != 0`. Rechazada tras auditoría técnica porque causaría regresiones en las ilustraciones estáticas del prólogo y dispararía falsos positivos en la telemetría.
- **Criterio ante imprevistos:** Si surgiera alguna incompatibilidad con un controlador OpenGL ES móvil antiguo, la función `alpha_blend` se expandirá algebraicamente en componentes escalares `r, g, b`.

---

## 3. Suposiciones

1. En la cinemática del dragón de FE7, el fondo base `Backdrop` es negro (`0x0000`), el fuego está en `BG3`, el cuerpo del dragón en `BG2`, los subtítulos en `BG0` y el cursor en `OBJ`.
   - *Si es falso:* La arquitectura desacoplada lee cada capa por su índice `0, 2, 3, obj` independientemente de su posición.
   - *Prototipar:* no.
2. La firma `(dispcnt & 0x0E07) == 0x0C00 && bldcnt != 0` identifica unívocamente la escena activa del dragón frente a las pinturas estáticas del prólogo.
   - *Si es falso:* Se comprobará adicionalmente la prioridad de capa `bg_prio3 == 3`.
   - *Prototipar:* no.
3. Naga valida y optimiza la rama condicional de fragmento sin pérdida de rendimiento en Vulkan ni OpenGL ES.
   - *Si es falso:* La estructura no tiene bucles anidados y compila a instrucciones planas de salto en GPU.
   - *Prototipar:* no.

---

## 4. Alcance

### Dentro del Alcance
- Modificar `src/ppu_nativa.wgsl` en `fs_comp`:
  - Evaluar `let es_escena_dragon = (cfg_comp.dispcnt & 0x0E07u) == 0x0C00u && (cfg_comp.bldcnt != 0u);`.
  - Si es verdadero: ejecutar la composición jerárquica *bottom-up* acumulativa (Backdrop -> BG3 fuego -> BG2 dragón -> BG0 texto -> OBJ cursor) respetando la máscara de ventanas `control`.
  - Si es falso: ejecutar la tubería estándar de 2 objetivos de la GBA.
- Actualizar y añadir pruebas unitarias en `src/ppu_nativa_tests.rs` (validación de Naga, coexistencia simultánea de 4 capas y preservación del prólogo estático).
- Empaquetar el APK de producción con `scripts/package_apk.ps1`.

### Fuera del Alcance
- Modificar la emulación de CPU, audio, sistema de archivos ni persistencia `.sav`.
- Alterar la versión de la aplicación sin confirmación explícita previa del usuario.

---

## 5. Criterios de Aceptación

### Criterio 1: Validación Sintáctica y de Tipos de WGSL en Naga
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa::tests::el_shader_de_la_ppu_nativa_valida_con_naga`
- **Dado** el shader `src/ppu_nativa.wgsl` con la composición jerárquica *bottom-up*.
- **Cuando** se compila con el validador Naga / wgpu.
- **Entonces** pasa con 0 errores y 0 advertencias de tipo.

### Criterio 2: Coexistencia Total de 4 Capas sin Corte Prematuro
- **Tipo:** normal
- **Test:** `cargo test --lib ppu_nativa::tests::test_composicion_multinivel_coexistencia_4_capas`
- **Dado** un píxel donde coinciden simultáneamente `Backdrop`, `BG3` (fuego), `BG2` (transparente), `BG0` (texto) y `OBJ` (cursor).
- **Cuando** se ejecuta la composición multinivel en el shader.
- **Entonces** el color final es el texto/cursor sobre la mezcla de fuego y backdrop, sin truncar el fuego.

### Criterio 3: Inocuidad 100% en el Resto del Juego y Pinturas del Prólogo
- **Tipo:** regresión
- **Test:** `cargo test --lib ppu_nativa::tests::test_sanear_bldcnt_no_altera_otras_escenas`
- **Dado** un fotograma con `BG1` activo (mapas y batallas) o pinturas estáticas del prólogo con `bldcnt == 0x0000`.
- **Cuando** se procesa en `ppu_nativa`.
- **Entonces** se toma la tubería estándar de 2 objetivos de GBATek sin alterar ningún registro ni píxel.

### Criterio 4: Tolerancia a Variantes sin Modo Alfa Previo
- **Tipo:** límite
- **Test:** `cargo test --lib ppu_nativa::tests::test_sanear_bldcnt_restaura_variante_dialogo_2861`
- **Dado** scanlines de la cinemática del dragón con `bldcnt == 0x2861` (variante con texto activo).
- **Cuando** `sanear_bldcnt_cinematica_dragon` procesa la scanline.
- **Entonces** asegura la coexistencia de `BG3` (`0x2869`) y fija `bldalpha = 0x0810`.

---

## 6. Cómo lo comprobaré

1. **Suite de Pruebas Unitarias de Rust:**
   `cargo test --lib ppu_nativa::tests`
   `cargo test --lib pipeline::tests`
   Confirmará que todos los tests unitarios pasen en verde y que Naga valide el shader WGSL.
2. **Generación de Fat APK Release:**
   `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`
   Verificará que el APK de 42,7 MB compile limpiamente y contenga las bibliotecas actualizadas para las 3 arquitecturas (`arm64-v8a`, `x86_64`, `armeabi-v7a`).

---

## 7. Plan de Rollback

Si surgiera algún fallo imprevisto, se restaurarán los archivos modificados a su estado previo en el espacio de trabajo mediante las herramientas de edición del agente.
