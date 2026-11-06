# Propuesta: persistir-anomalias-dragon-en-blazing-perf

Fecha: 2026-10-31
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Conecta el monitor de telemetría del dragón desde `src/pipeline.rs` hacia el bucle de eventos en `src/platform.rs`, emitiendo a 1 Hz en el archivo `blazing-perf.txt` una línea de comentario `# dragon_anomalias` con el recuento de cuadros anómalos y los registros `BLDCNT` capturados, permitiendo diagnosticar el parpadeo directamente desde el archivo de texto sin requerir `adb logcat`.

### 2. Contexto

- Contexto consultado: `src/platform.rs`, `src/pipeline.rs`, `src/perf.rs`, `.agents/telemetry/FINDINGS_CACHE.md`.
- Intención: Permitir al usuario observar los datos numéricos exactos de las anomalías de raster/blending del dragón directamente en `blazing-perf.txt` al probar el juego en el teléfono.
- Restricciones y decisiones previas (máx. 3):
  - Cero sobrecarga de rendimiento a 60 FPS (origen: `AGENTS.md §1`) → implica: no tocar el esquema tabular de métricas (`telemetria.rs`); usar emisión condicional con `state.append_perf_line` (idéntico a `# hotspots` y `# regiones`) solo si `count > 0` → protegida por: Drenado atómico de anomalías sin impacto cuando no hay incidencias
  - Cero contención en el hilo de emulación (origen: `AGENTS.md §1` y `ADR 42`) → implica: drenado atómico/lock-free en `EmulationBridge` → protegida por: Drenado correcto de anomalías acumuladas desde EmulationBridge
  - Preservar compatibilidad de formato con analizadores existentes de `blazing-perf.txt` → implica: emitir la línea con prefijo de comentario `#` → protegida por: no testeable
- Alternativas descartadas (solo si las hubo):
  - Modificar el esquema tabular de 28 columnas de `telemetria.rs` → descartada porque rompe los scripts de telemetría existentes (`analizar_telemetria.py`) y el contrato de versión.
  - Escribir en disco desde el worker de emulación en cada anomalía → descartada porque causa tirones (frame drops) en el hilo de emulación a 60 FPS.
- Ante imprevistos, prioriza: estabilidad de la tasa de cuadros y simplicidad sobre trazabilidad continua.

### 3. Ejemplos

- Caso normal: 60 fotogramas con modulación alfa normal de fuego en scanlines 0..99 → antes: no aparece nada en `blazing-perf.txt` → ahora: no se emite ninguna línea adicional, manteniendo el archivo limpio.
- Caso anómalo: 1 o más fotogramas pierden el blending de fuego durante el segundo → antes: solo salía a Logcat vía `log::warn!` → ahora: en `blazing-perf.txt` se escribe inmediatamente tras el bloque del segundo:
  `# dragon_anomalias count=3 bldcnt_l50=0x0144 bldcnt_fin=0x0144 blend_lines=0`

### 4. Pasos

1. En `src/pipeline.rs`, definir en `EmulationBridge` un acumulador de anomalías (conteo y último `DiagnosticoDragon`) accesible de forma atómica o mediante mutex ligero, junto con el método `drain_dragon_anomalias(&self) -> AnomaliasDragonAcumuladas`.
2. En `src/pipeline.rs`, en `EmulationWorker::run`, tras registrar el fotograma con `MonitorRasterDragon`, si se detecta anomalía, actualizar el acumulador en el puente.
3. En `src/platform.rs`, tras la llamada periódica `state.perf.tick_at(...)` y `state.write_perf_report(...)`, consultar `worker.bridge().drain_dragon_anomalias()`; si `count > 0`, formatear la línea `# dragon_anomalias ...` y registrarla mediante `state.append_perf_line(&msg)`.
4. Añadir tests unitarios en `src/pipeline_tests.rs` para verificar el registro y drenado atómico de las anomalías en el puente.

### 5. Qué más podría verse afectado

- `src/pipeline.rs`: estructura `EmulationBridge` y worker.
- `src/platform.rs`: bucle de eventos en `event_handler`.
- `src/pipeline_tests.rs`: tests de integración del puente.

### 6. Suposiciones

- El archivo `blazing-perf.txt` ignora las líneas que empiezan por `#` al parsear métricas tabulares → prototipar: no → si es falso: los scripts de análisis fallarían (ya ignoran `# ppu_hw`, `# regiones` y `# hotspots`).
- `AppState::append_perf_line` escribe y vacía (`flush`) la línea correctamente si el archivo de rendimiento está activo → prototipar: no → si es falso: la línea podría quedar en búfer.

### 7. Alcance

- Incluye: paso de estadísticas de anomalías del dragón desde `EmulationBridge` a `blazing-perf.txt` y tests unitarios en `pipeline_tests.rs`.
- No incluye: cambios en el renderizado de la PPU ni en el contrato de métricas de `telemetria.rs`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Drenado correcto de anomalías acumuladas desde EmulationBridge
Caso de uso: El worker registra una anomalía y el hilo principal la recupera en el informe de 1 segundo | Tipo: normal
Dado un `EmulationBridge` inicializado
Cuando el hilo de emulación registra una anomalía con `bldcnt_l50 = 0x0144` y `blend_lines = 0`
Entonces `bridge.drain_dragon_anomalias()` retorna `count == 1`, los registros asociados y deja el acumulador en 0 para el siguiente segundo
Test: `test_drenado_acumulador_anomalias_dragon`

Escenario: Drenado atómico de anomalías sin impacto cuando no hay incidencias
Caso de uso: Ejecución de fotogramas normales sin parpadeo | Tipo: límite
Dado un `EmulationBridge` en funcionamiento continuo sin anomalías
Cuando se llama a `bridge.drain_dragon_anomalias()`
Entonces retorna `count == 0` y no genera cadenas de formato ni escrituras
Test: `test_drenado_dragon_sin_anomalias`

Escenario: Regresión de la suite de pipeline
Caso de uso: Verificación de no regresión en hilos y canales de emulación | Tipo: regresión
Dado el módulo `pipeline`
Cuando se ejecutan todas las pruebas con `cargo test --lib pipeline`
Entonces las 14 pruebas existentes y las 2 nuevas pasan con código de salida 0
Test: `test_suite_pipeline_completa`

### 9. Cómo lo comprobaré

- Escenarios 1 y 2: `cargo test --lib test_drenado_acumulador_anomalias_dragon test_drenado_dragon_sin_anomalias`
  Resultado que cuenta como pasar: 2 passed; 0 failed.
  Qué NO detecta: Fallos en el sistema de archivos de Android si los permisos de escritura en disco fallan.
- Escenario 3: `cargo test --lib pipeline`
  Resultado que cuenta como pasar: al menos 16 passed; 0 failed.
  Qué NO detecta: Modificaciones en shaders WGSL.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Drenado correcto de anomalías acumuladas desde EmulationBridge: ✔ Predicho: test `test_drenado_acumulador_anomalias_dragon` verifica que el conteo se extrae y resetea a cero; Real: pasa exitosamente en `cargo test --lib pipeline`.
- Drenado atómico de anomalías sin impacto cuando no hay incidencias: ✔ Predicho: test `test_drenado_dragon_sin_anomalias` retorna `count == 0` sin escrituras adicionales; Real: pasa exitosamente.
- Regresión de la suite de pipeline: ✔ Predicho: 16 pruebas pasando sin fallos; Real: 16 passed, 0 failed en `Test Runner`.
- Visualización de línea `# dragon_anomalias` en `blazing-perf.txt` en Android (visible): [pendiente de prueba en móvil con el nuevo APK compilado]

### Contexto: ¿se respetó?

Sí. Se respetó la restricción de cero impacto a 60 FPS delegando el almacenamiento temporal a un Mutex ligero en `EmulationBridge` que solo se lee una vez por segundo en `platform.rs`. Se respetó la compatibilidad de formato prefijando con `# dragon_anomalias` y usando `state.append_perf_line`, idéntico a `# hotspots` y `# regiones`.

### Archivos modificados

- `src/pipeline.rs`: `AnomaliasDragonAcumuladas`, `record_dragon_anomalia` y `drain_dragon_anomalias` en `EmulationBridge`, y registro condicional en `EmulationWorker`.
- `src/platform.rs`: Consulta de anomalías en el disparo de 1s de `state.perf.tick_at` y escritura condicional con `state.append_perf_line`.
- `src/pipeline_tests.rs`: Tests unitarios `test_drenado_acumulador_anomalias_dragon` y `test_drenado_dragon_sin_anomalias`.
- `docs/propuestas/2026-10-31_persistir-anomalias-dragon-en-blazing-perf.md`: Actualizada a verificada por agente.

### Discrepancias

Ninguna en la suite de pruebas unitarias.

### Para la próxima vez

Empaquetar y desplegar el Fat APK con el APK Builder para que el usuario pueda capturar la traza en disco.

### Fuera de alcance (hallazgos que no se arreglaron)

- Corrección final del parpadeo (a la espera del nuevo volcado de `blazing-perf.txt`).

---

## Discrepancia (solo si el estado es "discrepada")

<!-- Esta sección se llena solo si la propuesta no se cumple -->

---

## Revisión humana

- Visualización de línea `# dragon_anomalias` en `blazing-perf.txt` en Android (visible): ✔ | ✘ | ?
- Restricción de cero sobrecarga a 60 FPS (no testeable): ✔ | ✘ | ?
- Restricción de compatibilidad de formato en blazing-perf.txt (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano:
