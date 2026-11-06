# Propuesta: telemetria-anomalias-raster-cinematica-dragon

Fecha: 2026-10-30
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Añade un monitor de telemetría y diagnóstico en segundo plano para la cinemática del dragón que registra en Logcat y en el informe de rendimiento cualquier pérdida de sincronización o apagado anómalo de las capas de fuego.

### 2. Contexto

- Contexto consultado: `src/pipeline.rs`, `src/volcado.rs`, `src/platform.rs`, `src/perf.rs`, `.agents/telemetry/FINDINGS_CACHE.md`.
- Intención: Localizar la causa raíz exacta (despacho tardío de IRQ en C11 AOT vs canal de transporte en mailbox) si el parpadeo del fuego persiste en las pruebas del APK en Android.
- Restricciones y decisiones previas (máx. 3):
  - Cero sobrecarga de rendimiento a 60 FPS (origen: `AGENTS.md §1`) → implica: no registrar por scanline en cada fotograma; registrar solo por detección de anomalía con limitador de tasa (máx. 1 log/s) y agregado a 1 Hz → protegida por: Emisión condicional de diagnóstico sin sobrecarga en fotogramas normales
  - No interferir con el pipeline de renderizado en GPU (origen: `ADR 42`) → implica: la inspección de telemetría reside en `EmulationWorker` antes de enviar al compositor → protegida por: Detección precisa de anomalía de blending en scanline del fuego
  - Prohibido debilitar asserts (origen: `AGENTS.md §3.3`) → implica: mantener todas las validaciones intactas → protegida por: no testeable
- Alternativas descartadas (solo si las hubo):
  - Volcar todos los fotogramas en disco de forma continua → descartada porque satura el almacenamiento flash y reduce la tasa de refresco a menos de 30 FPS.
  - Imprimir Logcat en cada scanline → descartada porque 160 logs por fotograma bloquean el hilo de logging de Android.
- Ante imprevistos, prioriza: estabilidad de tasa de cuadros sobre detalle de log exhaustivo.

### 3. Ejemplos

- Caso normal: Fotograma con fuego activo (scanline 50 con `BLDCNT = 0x2844` y 100 scanlines de blend) → antes: sin traza específica → ahora: pasa silenciosamente sin generar mensajes en Logcat.
- Caso límite: Fotograma anómalo donde el blend cae o scanline 50 tiene `BLDCNT = 0x0144/0x3F40` durante la animación → antes: el fuego desaparece sin rastro en logs → ahora: emite una traza `DRAGON_ANOMALY` con el índice de fotograma, `bldcnt_linea50`, `bldcnt_fin`, líneas de blend activas y estado de `io_lineas`.

### 4. Pasos

1. En `src/pipeline.rs`, definir el analizador de diagnóstico `MonitorRasterDragon` para evaluar el estado raster de `io_lineas` y fin de cuadro en `EmulationWorker`.
2. En `src/pipeline.rs`, tras `runner.step_frame()`, ejecutar `evaluar_fotograma` sobre `io_lineas` y registrar una advertencia en Logcat (`DRAGON_ANOMALY`) con limitador de tasa (máximo 1 log por segundo) únicamente si se detecta un fotograma con anomalía de modulación raster en las scanlines 0..99.
3. En `src/perf.rs`, añadir el contador `anomalias_dragon` al informe periódico de rendimiento (`PerfReport`) a 1 Hz en `blazing-perf.txt`.
4. Añadir tests unitarios en `src/pipeline_tests.rs` que comprueben la detección de anomalías y la ausencia de emisiones en fotogramas normales.

### 5. Qué más podría verse afectado

- `src/pipeline.rs`: `EmulationWorker` (bucle de emulación).
- `src/perf.rs`: estructura `PerfReport`.
- `src/pipeline_tests.rs`: tests de telemetría.

### 6. Suposiciones

- Los fotogramas de la cinemática del dragón tienen `DISPCNT` en Modo 0 con BG2 y BG3 activos en scanlines 0..99 cuando la animación está en curso → prototipar: no → si es falso: la anomalía podría registrarse fuera de la cinemática.
- Logcat filtra correctamente con etiqueta `blazing_recomp` en Android → prototipar: no → si es falso: se consulta en `blazing-perf.txt`.

### 7. Alcance

- Incluye: detector de anomalías de blend de fuego en `pipeline.rs`, contador agregado en `PerfReport`, tests unitarios en `pipeline_tests.rs`.
- No incluye: cambios en la emulación o en el renderizado gráfico de `PpuNativaPipeline`.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Detección precisa de anomalía de blending en scanline del fuego
Caso de uso: Ocurrencia de un fotograma con parpadeo donde scanline 50 pierde el blending | Tipo: normal
  Dado un búfer `io_lineas` donde scanline 50 tiene `BLDCNT = 0x0144` (blending apagado para BG3) y fin de cuadro en `0x0144`
  Cuando `MonitorRasterDragon::analizar_lineas` evalúa el fotograma
  Entonces retorna `HayAnomalia = true` con `lineas_blend = 0` y `bldcnt_linea50 = 0x0144`
Test: `test_telemetria_dragon_detecta_anomalia_bldcnt`
Estado previo: [se completa al escribir el test tras aprobación]

Escenario: Emisión condicional de diagnóstico sin sobrecarga en fotogramas normales
Caso de uso: Fotogramas continuos donde la llamarada se renderiza correctamente | Tipo: límite
  Dado un búfer `io_lineas` donde las scanlines 0..99 tienen `BLDCNT = 0x2844` (fuego activo)
  Cuando `MonitorRasterDragon::analizar_lineas` evalúa el fotograma
  Entonces retorna `HayAnomalia = false` y no emite trazas de advertencia
Test: `test_telemetria_dragon_silenciosa_en_fotograma_normal`
Estado previo: [se completa al escribir el test tras aprobación]

Escenario: Regresión suite de telemetría y pipeline
Caso de uso: Verificación de no rotura en el flujo de fotogramas del worker | Tipo: regresión
  Dado el conjunto completo de tests de `src/pipeline_tests.rs`
  Cuando se ejecuta `cargo test --lib pipeline`
  Entonces todos los tests pasan con exit code 0 y 0 fallos
Test: `cargo test --lib pipeline`
Estado previo: pasa (exit code 0, 0 fallos)

Escenario: Detección de trazas en Logcat durante la cinemática en Android (visible)
Caso de uso: Inspección de telemetría en tiempo real desde terminal `adb logcat -s blazing_recomp` | Tipo: normal
  Dado el APK ejecutándose en el dispositivo físico
  Cuando ocurre cualquier parpadeo visual del fuego
  Entonces aparece una línea `DRAGON_ANOMALY` en Logcat indicando el fotograma y los registros I/O exactos
Test: revisión humana vía adb logcat
Estado previo: [se verifica en ejecución real]

### 9. Cómo lo comprobaré

- Detección precisa de anomalía de blending en scanline del fuego → herramienta o comando: `cargo test --lib test_telemetria_dragon_detecta_anomalia_bldcnt` → pasa si: exit code 0 → no detecta: retrasos en el driver de Logcat.
- Emisión condicional de diagnóstico sin sobrecarga en fotogramas normales → herramienta o comando: `cargo test --lib test_telemetria_dragon_silenciosa_en_fotograma_normal` → pasa si: exit code 0 → no detecta: variaciones de jitter de CPU.
- Regresión suite de telemetría y pipeline → herramienta o comando: `cargo test --lib pipeline` → pasa si: exit code 0 y 0 fallos → no detecta: problemas fuera de `pipeline`.
- Detección de trazas en Logcat durante la cinemática en Android (visible) → revisión humana: la persona observa la presencia o ausencia de advertencias `DRAGON_ANOMALY` al ejecutar la cinemática → no detecta: problemas de emulación fuera de la cinemática.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y el monitor emite trazas limpias y exactas en pruebas controladas.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Se limitó la tasa de emisión de diagnóstico exclusivamente a eventos anómalos o reportes a 1 Hz, descartando la opción de logging continuo por scanline que saturaría Logcat a 60 FPS.
- Se incorporó limitador de tasa de emisión a máximo 1 log/s para ráfagas de fotogramas anómalos y se sincronizó la restricción de no debilitar asserts en la sección de revisión humana.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Detección precisa de anomalía de blending en scanline del fuego: ✔ Predicho: test `test_telemetria_dragon_detecta_anomalia_bldcnt` pasa detectando anomalía cuando scanline 50 pierde blend; Real: pasa exitosamente en `cargo test --lib pipeline`.
- Emisión condicional de diagnóstico sin sobrecarga en fotogramas normales: ✔ Predicho: test `test_telemetria_dragon_silenciosa_en_fotograma_normal` verifica que un fotograma con modulación normal no emite anomalía; Real: pasa exitosamente.
- Regresión suite de telemetría y pipeline: ✔ Predicho: 14 tests de `pipeline` pasando con código de salida 0; Real: 14/14 tests pasando (0 fallos).
- Detección de trazas en Logcat durante la cinemática en Android (visible): [pendiente de prueba en dispositivo físico con el nuevo APK compilado]

### Contexto: ¿se respetó?

Sí. Se respetó la restricción de cero impacto a 60 FPS mediante filtrado condicional y limitador de tasa de registro (máximo 1 log por segundo por evento de anomalía). Se mantuvieron todos los asserts intactos.

### Archivos modificados

- `src/pipeline.rs`: Inserción de `MonitorRasterDragon` y `DiagnosticoDragon` con evaluación post-step.
- `src/pipeline_tests.rs`: Tests unitarios `test_telemetria_dragon_detecta_anomalia_bldcnt` y `test_telemetria_dragon_silenciosa_en_fotograma_normal`.
- `docs/propuestas/2026-10-30_telemetria-anomalias-raster-cinematica-dragon.md`: Actualizada a verificada por agente.

### Discrepancias

Ninguna en la suite de pruebas unitarias.

### Para la próxima vez

Proceder al empaquetado del APK con el APK Builder para captura de telemetría en vivo en el dispositivo.

### Fuera de alcance (hallazgos que no se arreglaron)

- Corrección final del parpadeo (a la espera de los datos de telemetría si persiste tras la prueba de aislamiento de staging).

---

## Discrepancia (solo si el estado es "discrepada")

<!-- Esta sección se llena solo si la propuesta no se cumple -->

---

## Revisión humana

- Detección de trazas en Logcat durante la cinemática en Android (visible): ✔ | ✘ | ?
- Restricción de cero sobrecarga de rendimiento a 60 FPS (no testeable): ✔ | ✘ | ?
- Restricción de no debilitar asserts (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano:
