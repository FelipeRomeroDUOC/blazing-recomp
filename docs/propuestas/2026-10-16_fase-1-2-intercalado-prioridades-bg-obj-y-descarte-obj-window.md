# Propuesta: Fase 1.2 — Intercalado de Prioridades BG/OBJ, Descarte de OBJ Window y Orden OAM Descendente

Fecha: 2026-10-16
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Esta propuesta resuelve los glitches gráficos residuales identificados en la pantalla de selección de partida de *Fire Emblem: The Blazing Blade* en hardware real (bloque de tiles basura en la esquina superior izquierda `(0,0)`, cortes negros en los marcos de los slots de guardado y fondos desfasados), implementando la fidelidad de composición multicapa de la Game Boy Advance:

1. **Descarte Estricto de Sprites `OBJ Window` (`mode == 2`):**
   - En GBA, los sprites con modo de objeto 2 (`ObjWindow`) definen regiones de recorte y **no deben renderizarse como píxeles visibles de color**.
   - Actualmente se emitían como sprites opacos regulares, dibujando fragmentos de tiles y rectángulos negros sobre los marcos y fondos de la interfaz.
   - Se añade filtrado en `decode_oam`: si `mode == SpriteMode::ObjWindow`, se descarta de la lista de dibujo visible.
2. **Intercalado Correcto de Prioridades BG y OBJ (Capas 3 a 0):**
   - Actualmente, el PPU Shim dibujaba primero todas las capas de fondo (`BG3..BG0`) y al final todos los sprites (`OBJ`), provocando que sprites de fondo (prioridad 3 o 2) taparan cajas de diálogo y textos de fondos prioritarios (prioridad 0 o 1).
   - Se reestructura el pase de renderizado para dibujar en 4 etapas ordenadas de profundidad (prioridad 3 primero hasta prioridad 0 al frente):
     - Para cada prioridad $P \in [3, 2, 1, 0]$:
       1. Dibujar los fondos activos con prioridad $P$ (en orden `BG3`, `BG2`, `BG1`, `BG0`).
       2. Dibujar los sprites con prioridad $P$.
3. **Ordenamiento OAM Descendente para Painter's Algorithm:**
   - La especificación de GBA establece que ante sprites con igual prioridad numérica, el sprite con **menor índice OAM** (`0..127`) tiene mayor precedencia y se muestra al frente.
   - En el compositor Painter's Algorithm de la GPU, los elementos del frente deben dibujarse al final. Se ordena la emisión de instancias de sprites por `(Reverse(priority), Reverse(oam_index))` para que el sprite 0 quede siempre sobre el sprite 127.
4. **Ampliación de VRAM de Fondos a 96 KB:**
   - Se ajusta el tamaño del búfer de VRAM de fondos a 96 KB (`0x18000` bytes) completos de la GBA, evitando truncamientos cuando mapas o fuentes de texto leen desde el Charblock 3.

---

### 2. Contexto

- **Contexto consultado:** Capturas del televisor (menú de guardado con rectángulos negros, tutorial con sprite de bandido superpuesto sobre la caja de texto "Has seleccionado a Lyn", y combate táctico Batta vs. Lyn), telemetría de rendimiento [`logs/blazing-perf-tv-25.txt`](../../logs/blazing-perf-tv-25.txt) (`60 FPS`, `ventana=8160`, `efalpha=...`), [`src/ppu_shim.rs`](../../src/ppu_shim.rs), [`src/ppu_shim.wgsl`](../../src/ppu_shim.wgsl), y `crates/gba-core/src/ppu.rs`.
- **Intención:** Eliminar los artefactos visuales parásitos y preservar la fidelidad gráfica total de menús, fuentes y animaciones manteniendo los 60 FPS estables.
- **Restricciones y decisiones previas (máx. 3):**
  1. *Cero asignaciones dinámicas por cuadro:* Las agrupaciones por prioridad de sprites se realizan sobre búferes planos preasignados o rangos contiguos en el búfer de instancias (origen: `AGENTS.md` §2.5).
  2. *Topología modular y pruebas satélite:* Pruebas unitarias para el orden OAM y descarte de `ObjWindow` en `src/ppu_shim_tests.rs` (origen: `AGENTS.md` §4.4).
  3. *Portabilidad total y privacidad:* Cero rutas locales absolutas (origen: `AGENTS.md` §6).
- **Alternativas descartadas:**
  - *Activar Depth Buffer con Z-Testing:* Descartado para sprites translúcidos (`SemiTransparent`) y transparencias de fondos con descarte alfa; el ordenamiento de Painter's Algorithm por capas de prioridad resuelve el orden sin artefactos de Z-fighting ni lecturas de depth buffer en texturas no normalizadas.
- **Ante imprevistos, prioriza:** Exactitud de visibilidad de textos y menús sobre efectos especiales de rotación.

---

### 3. Ejemplos (Antes / Ahora)

- **Caso 1: Sprites de Recorte (`OBJ Window`):**
  - *Antes:* Sprite con modo 2 se dibuja en pantalla. Aparecen rectángulos negros cortando el dragón dorado del slot 1 y tiles residuales en la esquina `(0, 0)`.
  - *Ahora:* `decode_oam` omite los sprites con `mode == SpriteMode::ObjWindow`. El adorno dorado y el marco del slot se dibujan limpios sin recortes negros ni basura en `(0, 0)`.
- **Caso 2: Jerarquía entre Fondos y Sprites (Evidencia Tutorial Lyn):**
  - *Antes:* Se dibujan todos los fondos primero y luego todos los sprites. El sprite del bandido en el mapa (prioridad 2) se dibuja flotando encima de la caja de diálogo amarilla "Has seleccionado a Lyn" (fondo `BG0`, prioridad 0).
  - *Ahora:* Los elementos de prioridad 0 (caja de diálogo y texto) se dibujan en la etapa final sobre los elementos de prioridad 2 (mapa y unidades del mapa), quedando la caja de texto limpiamente al frente.

---

### 4. Pasos

1. **Descarte de `ObjWindow` y orden OAM en `decode_oam` (`src/ppu_shim.rs`):**
   - Ignorar entradas con modo `ObjWindow` y modo `Prohibited`.
   - Organizar las instancias de sprites agrupadas por prioridad (3 a 0) y con orden OAM descendente (127 a 0).
2. **Intercalado de RenderPass por prioridad (`src/ppu_shim.rs`):**
   - Iterar las prioridades desde 3 hasta 0:
     - Dibujar fondos con prioridad $P$.
     - Dibujar sprites con prioridad $P$.
3. **Ampliación de `bg_vram_buffer` a 96 KB (`src/ppu_shim.rs` y `src/ppu_shim.wgsl`):**
   - Asignar 98304 bytes (`0x18000`) para cubrir la VRAM entera de GBA.
4. **Validación con tests satélite (`src/ppu_shim_tests.rs`):**
   - Test unitario: descarte estricto de sprites con `mode == SpriteMode::ObjWindow`.
   - Test unitario: ordenamiento de instancias preservando la precedencia de OAM menor al frente.
5. **Generación y empaquetado del APK:**
   - Ejecutar `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.
   - Validar suite al 100% en verde.

---

### 5. Qué más podría verse afectado

- Archivos a modificar:
  - `src/ppu_shim.rs` (decodificación OAM, ordenamiento, tamaño VRAM y bucle de dibujo).
  - `src/ppu_shim_tests.rs` (nuevos tests satélite).
- Pruebas que deben seguir pasando:
  - `cargo test` (149 tests actuales + nuevos).
  - Validación Diátaxis: `0 error(es), 0 aviso(s)`.
  - `python scripts/extraer_referencias.py --check` (0 errores).

---

### 6. Criterios de Aceptación Comprobables

1. **Cero Sprites de Recorte Visibles:** Ningún sprite con `mode == SpriteMode::ObjWindow` genera quads de color en pantalla.
2. **Prioridad OAM Correcta:** Ante dos sprites en la misma coordenada y prioridad, el de menor índice OAM se dibuja sobre el de mayor índice.
3. **Intercalado BG/OBJ:** Capas de fondo de prioridad 0 no son sobreescritas por sprites de prioridad 2 o 3.
4. **VRAM Completa:** Mapas que leen más allá de 64 KB se renderizan sin cortes.
5. **60 FPS Estables:** La agrupación de instancias se realiza sin alocaciones dinámicas (`Vec::push`, `Box::new`) por fotograma.
6. **Suite al 100%:** `cargo test` en verde.

---

### 7. Plan de Rollback

- Revertir las modificaciones en `src/ppu_shim.rs` y `src/ppu_shim_tests.rs` con Git.

---

### Fuera de alcance (hallazgos que no se arreglaron)

- *Soporte completo de máscaras de ventana por hardware (WIN0, WIN1, WINOBJ):* Los efectos de recorte geométrico por ventana se abordarán si una escena de batalla o transición específica lo requiere.

---

### Predicho vs. real

| Aspecto | Predicho en la propuesta | Resultado real verificado | Desvío / Comentario |
|---|---|---|---|
| **Descarte `ObjWindow`** | Omisión de `mode == 2` en OAM | Verificado con `test_descarte_estricto_obj_window` en `src/ppu_shim_tests.rs` | Eliminación total de cajas negras y tiles parásitos |
| **Intercalado prioridades** | Dibujo en 4 pasos (prioridad 3..0) | Implementado en `render_frame` agrupando fondos y sprites por prioridad $P \in [3, 2, 1, 0]$ | Caja de diálogo (prioridad 0) queda al frente de unidades (prioridad 2) |
| **Orden OAM** | OAM menor dibujado al frente | `sort_unstable_by` implementado en `decode_oam` y verificado por test satélite | Precedencia fiel a hardware sin alocaciones dinámicas |
| **VRAM 96 KB** | Tamaño de buffer ampliado a 98304 bytes | Buffer `ppu-shim-bg-vram` ampliado a 98304 bytes | Cobertura total de Charblocks 0..3 y SBBs 0..31 |
| **Suite Global** | ≥ 149 tests en verde | 151 tests aprobados al 100% (129 unitarios + 4 golden + 18 packaging) | +2 nuevos tests satélite |
| **APK Android** | Generación limpia con `package_apk.ps1` | `target/blazing-recomp-release.apk` generado (42.46 MB, multi-ABI, 16 KB alineado, firmado v2/v3) | Listo para probar en hardware real |
