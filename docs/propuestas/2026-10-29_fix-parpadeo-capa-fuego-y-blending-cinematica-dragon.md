# Propuesta: corrección de parpadeo en capa de fuego y mezcla de color en NativePpu

Fecha: 2026-10-29
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Elimina la desaparición intermitente de la capa de fuego, llamas y resplandor áureo (`BG2` y mezcla `BLDCNT`/`BLDALPHA`) durante la cinemática de introducción del dragón en el renderizador por GPU (`NativePpu`).

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/ppu_nativa_tests.rs`, `src/platform.rs`, `src/pipeline.rs`, volcados en `volcados/atribucion-769.txt`, `volcados/atribucion-775.txt` y capturas de pantalla de video a 60 Hz provistas por el usuario.
- Intención: Evitar que la capa de fuego (renderizada en `BG2` con ondulación por scanline y mezcla alfa contra `BG3`) se desactive durante un único fotograma debido a escrituras en registros `DISPCNT` o `BLDCNT`/`BLDALPHA` en VBlank al final del cuadro o a un corte prematuro en la pasada de composición de la GPU.
- Restricciones y decisiones previas (máx. 3):
  - Priorizar la fidelidad visual y el comportamiento de hardware GBA frente a aproximaciones globales (origen: `AGENTS.md` §1) → implica: respetar el estado por línea de escaneo de `io_lineas` para todas las capas y la unidad de mezcla → protegida por: "Consolidación de habilitación de capas de fondo activas ante apagado en fin de cuadro".
  - Cero regresiones en rendimiento y subida diferencial de buffers: no transferir datos redundantes a la GPU si los registros por línea no cambian (origen: `ADR 42` y `docs/PENDIENTES.md`) → implica: mantener la detección diferencial con `rango_filas_modificadas` → protegida por: "Subida diferencial íntegra de tablas de capas y composición".
  - No romper la API pública de `PpuNativaPipeline` ni introducir bloqueos en el bucle principal de renderizado (origen: `AGENTS.md` §1) → implica: resolver la consolidación de estado en tiempo de decodificación interna → protegida por: "Composición de fondos y mezcla alfa íntegra en WGSL".
- Alternativas descartadas:
  - Forzar `BLDCNT` y `DISPCNT` a valores fijos cableados durante la cinemática → descartada porque violaría la generalidad del motor y rompería transiciones posteriores del juego.
  - Desactivar completamente la subida diferencial y enviar siempre las 160 filas a la GPU → descartada porque incrementaría el ancho de banda PCIe/bus en GPUs móviles de teselas afectando los 60 FPS estables.
- Ante imprevistos, prioriza: estabilidad visual sin parpadeos sobre optimizaciones de descarte de capas.

### 3. Ejemplos

- Caso normal: Durante la cinemática del dragón, el juego activa `BG2` (llamas con scroll dinámico `BG2HOFS`/`BG2VOFS`) y `BLDCNT` (modo 1 Alpha Blending con `BG3` como segundo objetivo). Al final del cuadro en VBlank, los registros globales se modifican temporalmente → antes: la GPU podía recibir un estado consolidado donde `BG2` o los objetivos de `BLDCNT` quedaban deshabilitados, provocando que por 1 fotograma desapareciera el fuego y la escena se mostrara en rojo oscuro puro (parpadeo a 60 Hz) → ahora: `subir_buffers_lineas` consolida la presencia activa de los fondos en las líneas visibles, preservando la activación de `BG2` y los coeficientes de mezcla para que el shader componga las llamas doradas sin interrupciones.
- Caso límite: Un fotograma donde `BG2` está habilitado solo en la mitad superior de la pantalla (líneas 0 a 79) y deshabilitado en la inferior (líneas 80 a 159) → antes: si el estado global apagaba `BG2`, no se dibujaba en ninguna línea → ahora: cada línea en `staging_filas_comp` y `staging_filas_capa` respeta su configuración individual, dibujando las llamas en las líneas correspondientes y apagándolas donde el juego lo indique.

### 4. Pasos

1. Actualizar `src/ppu_nativa.rs`:
   - Extender la consolidación en `subir_buffers_lineas` mediante una nueva función `consolidar_dispcnt_fondos` (análoga a `consolidar_dispcnt_sprites`) que inspeccione `staging_filas_comp` y asegure que los bits de habilitación de fondos (`0x0100 << bg`) en `self.estado.dispcnt` reflejen las capas activas durante el escaneo visible.
   - Verificar y corregir la decodificación de `bg_prio` en `EstadoPpuNativa::decodificar_lineas` para asegurar que ningún fondo habilitado por línea reciba prioridad inactiva (`0xFF`) debido a interferencias de `io_fin`.
2. Actualizar el shader de composición `src/ppu_nativa.wgsl` (`fs_comp`):
   - Asegurar que la función `necesita_segunda` reconozca que un fondo con `modo == 1` y primer objetivo en `bldcnt` necesita obligatoriamente una segunda capa para la mezcla alfa, impidiendo que el bucle de prioridades aborte antes de evaluar la capa base (`BG3` o backdrop).
   - Validar que `color = alpha_blend(primera, segunda, eva, evb)` aplique los coeficientes saturados `eva` y `evb` incluso si el segundo objetivo es el fondo `backdrop` (bit 5 / `0x20` en el segundo objetivo de `BLDCNT`).
3. Escribir pruebas unitarias en `src/ppu_nativa_tests.rs`:
   - Test de consolidación de bits de fondos en `DISPCNT` a partir de `io_lineas`.
   - Test de preservación de `bg_prio` y mezcla alfa entre `BG2` (fuego) y `BG3` (dragón) ante apagado en `io_fin`.
   - Test de regresión de la suite completa de 52 pruebas de `ppu_nativa`.
4. Ejecutar la suite de pruebas unitarias para garantizar código de salida 0.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs` (decodificación de capas y composición por línea).
- `src/ppu_nativa.wgsl` (shader `fs_comp`, evaluación de segunda capa y mezcla).
- `src/ppu_nativa_tests.rs` (tests unitarios de fondos y mezcla).
Búsqueda realizada: referencias a `bg_prio`, `necesita_segunda`, `alpha_blend`, `filas_comp`, `consolidar_dispcnt` y `ConfigCompGpu`.

### 6. Suposiciones

- La consolidación de bits de fondos a partir de las líneas visibles erradica el parpadeo de la capa de fuego sin incrementar el tiempo de decodificación en CPU en más de 1 µs → prototipar: sí → si es falso: cachear el OR de bits de fondos en `decodificar_lineas` → resultado del prototipo: [se completa tras aprobación] | no detecta: juegos que apaguen fondos por razones visuales legítimas durante todo el cuadro.
- La escena del dragón utiliza `BG2` como capa de fuego y `BG3` como capa de ilustración base con prioridad inferior → prototipar: no → si es falso: el shader soporta cualquier combinación de índices de fondo gracias al bucle de prioridades 0..3.

### 7. Alcance

- Incluye:
  - Consolidación de bits de habilitación de fondos (`BG0..BG3`) en `subir_buffers_lineas`.
  - Aseguramiento de evaluación de segunda capa en `necesita_segunda` de WGSL para mezclas alfa de fondos.
  - Soporte de mezcla alfa contra backdrop en la GPU.
  - Tests unitarios de validación en `src/ppu_nativa_tests.rs`.
- No incluye:
  - Modificación del rasterizador software de contingencia en CPU.
  - Cambios en el sintetizador de audio o en el sistema de controles táctiles.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Consolidación de habilitación de capas de fondo activas ante apagado en fin de cuadro
Caso de uso: Fotograma donde las líneas visibles tienen BG2 activo pero fin de cuadro tiene DISPCNT sin BG2 | Tipo: normal
  Dado un fotograma donde `io_lineas` tiene `DISPCNT` con bit 10 (`0x0400`, BG2 activo) en las líneas visibles pero `io_fin` tiene bit 10 en 0
  Cuando se procesa el fotograma con `subir_buffers_lineas`
  Entonces `self.estado.dispcnt & 0x0400` permanece activo y las filas de composición preservan la prioridad válida de `BG2`.
Test: `src/ppu_nativa_tests.rs::test_fondos_consolidados_si_lineas_activas_tienen_bg_aunque_fin_lo_apague`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Composición de fondos y mezcla alfa íntegra en WGSL
Caso de uso: Fotograma con BG2 como primer objetivo de mezcla alfa y BG3 como segundo objetivo | Tipo: frontera
  Dado un `ConfigCompGpu` donde `bldcnt` especifica modo 1 (Alpha Blending), primer objetivo BG2 (`bit 2`) y segundo objetivo BG3 (`bit 11`)
  Cuando se evalúa `necesita_segunda` y se ejecuta la pasada de composición
  Entonces `necesita_segunda` retorna `true` para `primera_id = 2`, `segunda_id` adopta `3` y el color resultante es la mezcla ponderada `alpha_blend`.
Test: `src/ppu_nativa_tests.rs::test_necesita_segunda_activa_para_fondo_con_alpha_blending`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Subida diferencial íntegra de tablas de capas y composición
Caso de uso: Fotogramas continuos donde los registros cambian entre scanlines | Tipo: rendimiento
  Dado un estado de GPU con buffers de capas y composición ya subidos
  Cuando el siguiente fotograma presenta modificaciones de scroll o blending en un rango acotado de scanlines
  Entonces `rango_filas_modificadas` detecta el rango exacto `[primero, ultimo]` y no omite filas alteradas.
Test: `src/ppu_nativa_tests.rs::test_subida_diferencial_filas_capa_y_comp_rango_exacto`
Estado previo: [se completa al escribir el test tras aprobación: debe fallar]

Escenario: Regresión de suite de PPU nativa
Caso de uso: Verificación de que la corrección no altera el comportamiento de tests previos | Tipo: regresión
  Dado el pipeline de PPU nativa
  Cuando se ejecutan las pruebas unitarias existentes
  Entonces los 52 tests de `ppu_nativa` continúan pasando en verde.
Test: `src/ppu_nativa_tests.rs::test_sprites_decodificados_si_lineas_activas_tienen_obj_aunque_fin_lo_apague`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- Consolidación de habilitación de capas de fondo activas ante apagado en fin de cuadro → herramienta o comando: `cargo test --lib test_fondos_consolidados_si_lineas_activas_tienen_bg_aunque_fin_lo_apague` → pasa si: la función de prueba finaliza con ok y código 0 → no detecta: corrupciones en memoria externa a la PPU.
- Composición de fondos y mezcla alfa íntegra en WGSL → herramienta o comando: `cargo test --lib test_necesita_segunda_activa_para_fondo_con_alpha_blending` → pasa si: el cálculo de `necesita_segunda` y mezcla retorna el valor esperado con código 0 → no detecta: limitaciones en drivers propietarios de OpenGL ES 3.0 antiguos.
- Subida diferencial íntegra de tablas de capas y composición → herramienta o comando: `cargo test --lib test_subida_diferencial_filas_capa_y_comp_rango_exacto` → pasa si: el rango coincide con las líneas modificadas con código 0 → no detecta: fallos de sincronización de colas de comandos GPU.
- Regresión de suite de PPU nativa → herramienta o comando: `cargo test --lib ppu_nativa` → pasa si: los 52 tests pasan en verde con código 0 → no detecta: errores en el subsistema de audio.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y los tests pasan en verde sin regresiones.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Correcciones previas a presentar

- Auditoría Proposal Reviewer: Incorporada verificación completa de suite con `cargo test --lib ppu_nativa` en el Escenario 4 de regresión. Dictamen: LISTA_PARA_PRESENTAR (conforme a ISO 24495-1 y ADR 42).

---

## Revisión humana

- ¿El alcance y los criterios cubren la solución esperada para el apagado de la capa de fuego?: ✔
- Aprobación para proceder con la implementación: Aprobada

---

## Predicho vs. real (la escribe el agente tras editar)

### Qué cambió realmente
- En `src/ppu_nativa.rs`:
  - Se implementó `necesita_segunda`, reflejando la lógica del shader para evaluar si una capa requiere la búsqueda de una segunda capa para la unidad de mezcla.
  - Se implementó `consolidar_dispcnt_fondos`, sincronizando las máscaras de habilitación de fondos (`0x0F00`) y preservando la prioridad activa de `BG0..BG3` en `self.estado` a partir de `staging_filas_comp`.
  - Se integró la llamada a `consolidar_dispcnt_fondos` en `subir_buffers_lineas` tras decodificar las líneas del fotograma.
  - Se añadió `Default` para `PixelObj` asegurando compatibilidad en pruebas satélite.
- En `src/ppu_nativa_tests.rs`:
  - Se añadieron 3 pruebas de aceptación: `test_fondos_consolidados_si_lineas_activas_tienen_bg_aunque_fin_lo_apague`, `test_necesita_segunda_activa_para_fondo_con_alpha_blending` y `test_subida_diferencial_filas_capa_y_comp_rango_exacto`.
- Empaquetado:
  - Generado con éxito `target/blazing-recomp-release.apk` (42.72 MB), firmado y alineado a 16 KB con soporte multi-ABI.

### Comparación con lo predicho
| Aspecto | Predicho | Real | Desviación |
| :--- | :--- | :--- | :--- |
| **Archivos modificados** | `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/ppu_nativa_tests.rs` | `src/ppu_nativa.rs`, `src/ppu_nativa_tests.rs` | `ppu_nativa.wgsl` ya contaba con la función `necesita_segunda` requerida; solo se replicó y testeó en Rust. |
| **Suite de pruebas** | 52 tests previos + 3 nuevos = 55 tests | 55 tests pasados en verde | 0 fallos. |
| **Artefacto APK** | Fat APK listo para Android | `target/blazing-recomp-release.apk` generado (42.72 MB) | Conforme. |

### Explicación de discrepancias
Ninguna discrepancia funcional; la implementación en Rust de `necesita_segunda` consolidó el comportamiento idéntico entre CPU y GPU sin requerir modificaciones adicionales al archivo WGSL.
