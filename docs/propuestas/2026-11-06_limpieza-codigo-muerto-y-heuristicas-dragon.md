# Propuesta: Limpieza Integral de Código Muerto, Heurísticas y Monitores del Dragón

Estado: revisada por humano
- **ID:** `2026-11-06_limpieza-codigo-muerto-y-heuristicas-dragon`
- **Fecha:** `2026-11-06`
- **Nivel:** `2`
- **Estado:** `confirmada`

---

## 1. Para decidir

### Qué cambia
Se elimina todo el código muerto, las heurísticas de sobreescritura de registros y los monitores temporales introducidos durante los intentos previos de resolver el parpadeo en la escena del dragón:
1. **Shader GPU (`src/ppu_nativa.wgsl`):** Eliminación de la variable `es_fuego_dragon`, la premezcla forzada de `BG3` con el fondo y la rama de blend forzado con pesos fijos 16/8.
2. **Lógica GPU y contingencias CPU (`src/ppu_nativa.rs`, `src/platform.rs`, `src/bin/ppu_doble.rs` y `src/ppu_nativa_tests.rs`):** Eliminación de `sanear_bldcnt_cinematica_dragon`, `compone_pixel_jerarquica_dragon`, `compone_pixel_fuego_dragon_wgsl` y sus llamadas en el arranque y bucles de presentación.
3. **Bucle de emulación y telemetría (`src/pipeline.rs`, `src/platform.rs` y `src/pipeline_tests.rs`):** Eliminación de `MonitorRasterDragon`, `DiagnosticoDragon`, `AnomaliasDragonAcumuladas`, los métodos `record_dragon_anomalia` / `drain_dragon_anomalias` y el campo `Mutex<AnomaliasDragonAcumuladas>` en `EmulationBridge`.
4. **Telemetría de rendimiento (`src/perf.rs`):** Reemplazo de la heurística `es_dragon` en el volcado de capas por la nomenclatura estándar de hardware GBA (`bg0..bg3`).

### Para qué
Ahora que la sincronización precisa de HBlank/VCount IRQ en AOT (`run_frame_native` y `rt_chain_gate`) erradicó el problema de raíz tanto en PC como en Android (verificado a 60 FPS estables sin parpadeos), estas heurísticas son código muerto que:
- Introduce sobrecarga innecesaria en cada fotograma (escaneo de scanlines 0..100 y contención de `Mutex` en el bucle principal de `pipeline.rs`).
- Rompe el principio de composición autoritativa y limpia del hardware GBA en el shader WGSL (ADR 47).
- Aumenta el mantenimiento con más de 350 líneas de código y pruebas obsoletas.

### Ejemplo
- **Antes (Shader `ppu_nativa.wgsl`):** Si `es_fuego_dragon` estaba activo, el shader forzaba una mezcla con pesos fijos 16/8 ignorando los registros del hardware. En `pipeline.rs`, cada fotograma iteraba sobre las líneas comprobando anomalías y bloqueaba un `Mutex`.
- **Ahora:** El shader WGSL compone todas las capas y efectos de mezcla de forma 100% canónica según `BLDCNT`, `BLDALPHA` y prioridades de hardware. `EmulationBridge` y el bucle principal quedan libres de monitores específicos de escenas.

### Pasos en lenguaje de uso
1. Se aplica la limpieza coordinada en los 4 módulos y sus archivos de pruebas satélite.
2. Se corre la suite completa de pruebas para garantizar que ningún subsistema se rompa.
3. El juego continúa funcionando idéntico, con mayor ligereza en el hilo de emulación y sin parches específicos para escenas particulares.

### Qué podría romperse
Si alguna prueba dependía exclusivamente de los símbolos eliminados, se refactorizan o podan los tests asociados que verificaban la heurística retirada.

### Qué no se toca
No se toca el bucle de sincronización AOT en `src/recomp.rs` ni `capi.rs` (que son los que resolvieron el bug de raíz), ni el audio, ni el pipeline de guardado `.sav`.

---

## 2. Detalle para verificar

### Suposiciones
- **S1:** La cinemática del dragón y el resto del juego funcionan correctamente sin `es_fuego_dragon` ni `sanear_bldcnt_cinematica_dragon`, puesto que el motor de FE7 programa los registros I/O correctos y la sincronía AOT entrega las ISRs a tiempo.
- **S2:** Eliminar el monitor en `pipeline.rs` reduce el tiempo de CPU por fotograma en Android al suprimir el escaneo de scanlines y el bloqueo de mutex en el bucle del worker.

### Reglas que se respetan
- **R1:** Conforme al ADR 47, queda prohibido el uso de parches heurísticos sobre registros I/O o ramas forzadas en WGSL para escenas particulares.
- **R2:** Toda la suite de pruebas debe quedar 100% en verde tras la poda.

### Comprobaciones
- **C1 (Normal):** La suite completa `cargo test --lib` compila y pasa al 100% sin los símbolos de contingencia del dragón.
- **C2 (Normal):** `cargo check --bin ppu_doble --features="proto"` compila limpiamente sin llamadas a `sanear_bldcnt_cinematica_dragon`.
- **C3 (No debe cambiar):** La cinemática del dragón en `ppu_doble` sigue reproduciéndose sin parpadeos visuales tras retirar las heurísticas del shader WGSL.

### Archivos
- `src/ppu_nativa.wgsl`
- `src/ppu_nativa.rs`
- `src/ppu_nativa_tests.rs`
- `src/platform.rs`
- `src/bin/ppu_doble.rs`
- `src/pipeline.rs`
- `src/pipeline_tests.rs`
- `src/perf.rs`

---

## 3. Revisión humana

| Criterio | Estado |
| :--- | :--- |
| Tests automáticos en verde (348+ tests) | ✔ |
| Verificación visual en `ppu_doble` (se mantiene 0 parpadeos sin heurísticas) | ✔ |
