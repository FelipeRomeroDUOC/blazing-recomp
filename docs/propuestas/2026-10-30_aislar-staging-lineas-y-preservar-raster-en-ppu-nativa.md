# Propuesta: aislar-staging-lineas-y-preservar-raster-en-ppu-nativa

Fecha: 2026-10-30
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La llamarada y texturas de fuego (`BG3`) sobre el dragón (`BG2`) en la cinemática de introducción se renderizan de forma continua y estable en Android sin desaparecer ni parpadear mientras avanzan los subtítulos de texto.

### 2. Contexto

- Contexto consultado: `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/pipeline.rs`, `src/runner.rs`, `.agents/telemetry/FINDINGS_CACHE.md`, `volcados/atribucion-769.txt`, `volcados/atribucion-775.txt`.
- Intención: Evitar que la modulación raster por línea (`BLDCNT`, `BLDALPHA`, scrolls) decodificada desde `io_lineas` sea destruida por la propagación y consolidación con el estado residual de fin de cuadro (`self.estado`), permitiendo que el shader WGSL evalúe continuamente la segunda capa de fuego (`BG3`).
- Restricciones y decisiones previas (máx. 3):
  - No alterar la decodificación pura de `io_lineas` en `decodificar_lineas` (origen: `ADR 42 / ppu_nativa.rs`) → implica: mantener la correspondencia 1:1 de registros por scanline → protegida por: Preservar configuración raster por línea sin sobrescritura de fin de cuadro
  - Preservar el fallback estático cuando `io_lineas` es `None` (origen: `src/ppu_nativa.rs:1873`) → implica: solo aplicar `consolidar_dispcnt_fondos`, `propagar_fondos_a_staging` y `consolidar_dispcnt_sprites` cuando no haya registros por línea → protegida por: Fallback de consolidación cuando no hay registros por línea
  - Prohibido debilitar asserts o alterar las matemáticas de blend en el shader WGSL (origen: `AGENTS.md §3.3`) → implica: la corrección reside en aislar el staging de Rust sin modificar las fórmulas de mezcla en WGSL → protegida por: no testeable
- Alternativas descartadas (solo si las hubo):
  - Interrumpir forzadamente funciones C AOT mediante longjmp/preemption en `rt_tick` → descartada porque los 144.875 bloques C compilados mantienen estado en registros locales del host y corromperían la pila de ejecución.
  - Forzar `BLDCNT = 0x2844` de forma global en todo el fotograma → descartada porque arruina la composición de menús, cajas de texto y el fundido a negro posterior a la scanline 100.
- Ante imprevistos, prioriza: fidelidad scanline a scanline sobre simplificación de búferes globales.

### 3. Ejemplos

- Caso normal: Escena del dragón con `io_lineas` presente donde la scanline 50 tiene `BLDCNT = 0x2844` (fuego activo) e `io_fin` tiene `0x0144` → antes: `staging_filas_comp[50]` se sobrescribía con `0x0144` y `staging_filas_comp[50].dispcnt |= 0x8000`, anulando el blend de BG3 y apagando el fuego → ahora: `staging_filas_comp[50]` mantiene intacto `BLDCNT = 0x2844`, `DISPCNT = 0x1F00` y sus prioridades nativas, renderizando la llamarada de forma continua.
- Caso límite: Fotograma de inicio de cinemática sin `io_lineas` (`None`) → antes: aplicaba consolidación y propagación global → ahora: sigue aplicando consolidación y propagación global como contingencia exclusiva.

### 4. Pasos

1. En `src/ppu_nativa.rs` (`subir_buffers_lineas`), mover las llamadas a `consolidar_dispcnt_fondos` y `propagar_fondos_a_staging` al bloque `None` del `match io_lineas`, garantizando que cuando `io_lineas` sea `Some`, las 160 filas de `staging_filas_capa` y `staging_filas_comp` permanezcan 100% aisladas e idénticas a la decodificación autoritativa de hardware.
2. En `src/ppu_nativa.rs` (`subir_buffers_lineas`), condicionar las invocaciones de `consolidar_dispcnt_sprites` para que solo se ejecuten cuando `io_lineas.is_none()`, evitando que `OBJWIN` (`0x8000`) sea forzado indiscriminadamente en todas las scanlines y cancele los efectos de color (`control & 0x20 == 0`).
3. Añadir tests unitarios satélite en `src/ppu_nativa_tests.rs` que verifiquen el aislamiento del staging con `io_lineas` heterogéneas (scanlines con blend activo y fin de cuadro con blend inactivo) y el fallback cuando `io_lineas` es `None`.

### 5. Qué más podría verse afectado

- `src/ppu_nativa.rs`: método `subir_buffers_lineas` (único punto de carga a GPU).
- `src/ppu_nativa_tests.rs`: tests de aislamiento de staging.

### 6. Suposiciones

- El canal de paso de `io_lineas` desde `EmulationWorker` hacia `PpuNativaPipeline` en Android entrega el slice completo de 160 líneas en cada fotograma renderizado → prototipar: no → si es falso: habría que revisar el single-slot mailbox en `pipeline.rs`.
- Las 100 primeras scanlines en `volcados/atribucion-769.txt` y en tiempo real bajo AOT contienen `BLDCNT` con bit 2 activo y bits 6-7 en modo alfa durante la animación del fuego → prototipar: no → si es falso: requeriría forzar sincronización raster en `recomp.rs`.

### 7. Alcance

- Incluye: Aislamiento estricto de `staging_filas_capa` y `staging_filas_comp` frente a `self.estado` cuando `io_lineas` está presente; protección contra propagación de `OBJWIN` sobre scanlines sin ventanas; pruebas unitarias de aislamiento en `src/ppu_nativa_tests.rs`.
- No incluye: Modificaciones a los shaders WGSL (`fs_comp`), re-generación de código C11 ni alteración de rutinas de audio o mandos.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Preservar configuración raster por línea sin sobrescritura de fin de cuadro
Caso de uso: Renderizado de la cinemática del dragón con modulación HBlank activa | Tipo: normal
  Dado un buffer `io_lineas` donde la scanline 50 tiene `BLDCNT = 0x2844` y `DISPCNT = 0x1F00`, y `self.estado` tiene `io_fin` con `BLDCNT = 0x0144` y `DISPCNT = 0x9F00`
  Cuando se llama a `subir_buffers_lineas` con `Some(&io_lineas)`
  Entonces `staging_filas_comp[50].bldcnt` es igual a `0x2844`, `staging_filas_comp[50].dispcnt` conserva `0x1F00` (bit 15 en 0), y `staging_filas_capa[50].capas[3].enabled` es igual a 1
Test: `test_staging_lineas_aislado_de_estado_residual`
Estado previo: [se completa al escribir el test tras aprobación]

Escenario: Fallback de consolidación cuando no hay registros por línea
Caso de uso: Renderizado de fotogramas estáticos o contingencia donde no hay registros capturados | Tipo: límite
  Dado que `io_lineas` es `None` y `self.estado` tiene un fondo activo consolidado en `dispcnt`
  Cuando se llama a `subir_buffers_lineas` con `None`
  Entonces `staging_filas_capa` y `staging_filas_comp` se pueblan con la configuración consolidada de `self.estado`
Test: `test_staging_fallback_cuando_io_lineas_es_none`
Estado previo: [se completa al escribir el test tras aprobación]

Escenario: Regresión suite de renderizado PPU nativa
Caso de uso: Verificación de no rotura de fondos, mosaico, ventanas y sprites | Tipo: regresión
  Dado el conjunto completo de 61 tests unitarios de `src/ppu_nativa.rs`
  Cuando se ejecuta la suite con `cargo test --lib ppu_nativa`
  Entonces los 61 tests pasan exitosamente sin fallos
Test: `cargo test --lib ppu_nativa`
Estado previo: pasa (61 pasados, 0 fallados)

Escenario: Estabilidad visual del fuego en Android sin parpadeos (visible)
Caso de uso: Partida real en el dispositivo Android durante la introducción del dragón | Tipo: normal
  Dado el juego ejecutándose a 60 FPS en Android en la cinemática del dragón
  Cuando los subtítulos de texto inferior se van desplegando carácter a carácter
  Entonces la textura naranja de fuego (`BG3`) se mantiene visible y animada de forma continua sobre el dragón (`BG2`) sin desaparecer en ningún fotograma
Test: revisión humana en dispositivo Android
Estado previo: [se verifica con nuevo APK compilado]

### 9. Cómo lo comprobaré

- Preservar configuración raster por línea sin sobrescritura de fin de cuadro → herramienta o comando: `cargo test --lib test_staging_lineas_aislado_de_estado_residual` → pasa si: exit code 0 → no detecta: diferencias visuales específicas de shaders de GPU.
- Fallback de consolidación cuando no hay registros por línea → herramienta o comando: `cargo test --lib test_staging_fallback_cuando_io_lineas_es_none` → pasa si: exit code 0 → no detecta: fallos en rasterizadores externos.
- Regresión suite de renderizado PPU nativa → herramienta o comando: `cargo test --lib ppu_nativa` → pasa si: 61 pasados, 0 fallados → no detecta: problemas fuera del módulo `ppu_nativa`.
- Estabilidad visual del fuego en Android sin parpadeos (visible) → revisión humana: la persona verifica en el APK instalado que la llamarada naranja sobre el dragón se mantiene visible y animada sin parpadeos mientras el texto inferior avanza → no detecta: métricas de memoria internas.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y el APK compilado es validado en el dispositivo físico.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Se corrigió la premisa errónea de la propuesta anterior que intentaba abortar bloques C11 en `rt_tick`, reconociendo que los bloques C11 exportan funciones `void` que no pueden interrumpirse limpiamente sin desbordar o corromper el host.
- Se identificó la causa exacta de la discrepancia de la propuesta anterior: `propagar_fondos_a_staging` y `consolidar_dispcnt_sprites` estaban pisando incondicionalmente el búfer `staging_filas_comp` y `staging_filas_capa` con el estado `self.estado` (de fin de cuadro), anulando la decodificación de `io_lineas` y forzando `OBJWIN` sobre scanlines sin ventana.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Preservar configuración raster por línea sin sobrescritura de fin de cuadro: coincide
  Resultado real: `test_staging_lineas_aislado_de_estado_residual` pasó exitosamente (staging_comps[50].bldcnt = 0x2844, dispcnt = 0x1F00 sin OBJWIN, BG3 enabled, necesita_segunda = true).
  Método: el anunciado (`cargo test --lib test_staging_lineas_aislado_de_estado_residual`).
- Fallback de consolidación cuando no hay registros por línea: coincide
  Resultado real: `test_staging_fallback_cuando_io_lineas_es_none` pasó exitosamente (recibe configuración consolidada de estado en las 160 filas).
  Método: el anunciado (`cargo test --lib test_staging_fallback_cuando_io_lineas_es_none`).
- Regresión suite de renderizado PPU nativa: coincide
  Resultado real: 63 tests pasados, 0 fallados, 0 ignorados (exit code 0).
  Método: el anunciado (`cargo test --lib ppu_nativa`).
- Estabilidad visual del fuego en Android sin parpadeos (visible): pendiente de validación física
  Resultado real: APK compilado listo para prueba humana en dispositivo.
  Método: el anunciado (revisión humana en dispositivo Android).

### Contexto: ¿se respetó?

- Restricción no alterar decodificación pura de io_lineas → respetada (decodificar_lineas se mantiene 1:1 intacta).
- Restricción preservar fallback estático cuando io_lineas es None → respetada (consolidar_dispcnt_fondos, propagar_fondos_a_staging y consolidar_dispcnt_sprites activos bajo None).
- Restricción no debilitar asserts ni alterar matemáticas WGSL → respetada (sin cambios en shaders WGSL, fórmulas intactas).
- ¿Se tocó algo que alguna restricción prohibía? no
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: sí
- ¿Reapareció alguna alternativa descartada? no

### Archivos modificados

- `src/ppu_nativa.rs`: incorporación de `preparar_staging_filas` en `EstadoPpuNativa` y ajuste de `subir_buffers_lineas`.
- `src/ppu_nativa_tests.rs`: adición de tests de aceptación `test_staging_lineas_aislado_de_estado_residual` y `test_staging_fallback_cuando_io_lineas_es_none`.
¿Coincide con lo anunciado? sí

### Discrepancias

ninguna detectada en tests unitarios ni suite de renderizado PPU.

### Para la próxima vez

Encapsular la preparación de staging en métodos puros de estructura (`preparar_staging_filas`) desde el primer momento para evitar que heurísticas de fallback se mezclen con el camino autoritativo por línea.

### Fuera de alcance (hallazgos que no se arreglaron)

ninguno

---

## Discrepancia (solo si el estado es "discrepada")

<!-- Esta sección se llena solo si la propuesta no se cumple -->

---

## Revisión humana

- Estabilidad visual del fuego en Android sin parpadeos (visible): ✔ | ✘ | ?
- Restricción de no debilitar asserts o alterar matemáticas de blend (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano:
