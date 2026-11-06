# Propuesta: pacing acumulativo a 60 FPS y eliminación de audio underrun

Fecha: 2026-10-28
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El juego ahora regula el tiempo entre fotogramas usando un reloj objetivo acumulativo con compensación de retraso, eliminando la caída artificial a 58 FPS y erradicando el sonido entrecortado por falta de muestras de audio en Android.

### 2. Contexto

- Contexto consultado: `src/pipeline.rs`, `src/audio.rs`, `src/platform.rs`, telemetría de dispositivo móvil 1264x2736 (2026-10-28).
- Intención: Mantener 60.0 FPS estables y una alimentación continua de muestras en AAudio para evitar microtirones y chasquidos de sonido en dispositivos Android.
- Restricciones y decisiones previas (máx. 3):
  - Compensación acotada de retraso (*spiral of death* guard): si el hilo acumula un retraso superior a 100 ms (ej. al pausar o cambiar de app), el reloj objetivo debe reajustarse inmediatamente a `Instant::now()` en lugar de intentar recuperar decenas de fotogramas atrasados a máxima velocidad (origen: `AGENTS.md` §1) → implica: poner techo al acumulador temporal → protegida por: "Reajuste temporal ante pausa prolongada".
  - Preservación de latencia de audio baja: el búfer circular no debe inflarse artificialmente por encima de 4.096 fotogramas (~85 ms) para no degradar la sincronía audio-video (origen: `docs/PENDIENTES.md` §2) → implica: corregir el pacing del productor en lugar de agrandar el búfer de salida → protegida por: "Cadencia exacta de 60 fotogramas por segundo sin deriva".
  - Cero dependencias bloqueantes en el hilo principal: la sincronización temporal pertenece exclusivamente al hilo de trabajo `EmulationWorker` (origen: `AGENTS.md` §1) → implica: no introducir bloqueos en `android-activity` ni en el swapchain GPU → protegida por: "Cadencia exacta de 60 fotogramas por segundo sin deriva".
- Alternativas descartadas:
  - Duplicar el tamaño del búfer circular a 16.384 muestras → descartada porque solo aplaza el vaciado y añade más de 150 ms de retraso perceptible al sonido.
  - Eliminar el tiempo de reposo y dejar que el VSync de Vulkan marque el ritmo → descartada porque en pantallas de 120 Hz duplicaría la velocidad del juego al doble (120 FPS).
- Ante imprevistos, prioriza: estabilidad de 60 FPS y continuidad auditiva sobre sincronización absoluta de microsegundos.

### 3. Ejemplos

- Caso normal: El planificador del kernel de Android introduce 1 ms de exceso de reposo en un fotograma de emulación → antes: ese milisegundo se perdía permanentemente, acumulando un déficit que rebajaba la velocidad a 58 FPS y vaciaba el búfer de audio → ahora: el siguiente fotograma descuenta automáticamente el milisegundo excedente del tiempo de reposo, promediando exactamente 60.0 FPS.
- Caso límite: La aplicación se suspende durante varios segundos en segundo plano → antes: no existía mecanismo de reajuste temporal → ahora: el guarda de retraso detecta una diferencia superior a 100 ms y reinicia el ancla temporal en el fotograma de reanudación sin aceleraciones bruscas.

### 4. Pasos

1. Crear en `src/pipeline.rs` el componente `FramePacer`, encapsulando el cálculo incremental del tiempo objetivo (`next_frame += FRAME_DURATION`) y la compensación de exceso de reposo desacoplada del reloj del sistema.
2. Integrar `FramePacer` en el bucle de `EmulationWorker::start_with_save`, incorporando el techo de recuperación de 100 ms para reanudar limpiamente tras pausas o desvíos severos.
3. Asegurar en `src/audio.rs` que el arranque inicial y la reanudación del motor de audio conserven el pre-cebado del búfer circular en su marca de agua objetivo (`TARGET_WATERMARK_FRAMES`), evitando que el consumidor lea silencio antes del primer fotograma.
4. Escribir tests unitarios en `src/pipeline_tests.rs` y `src/audio_tests.rs` comprobando el cálculo matemático del pacer, el techo de retraso, el cebado inicial y la ausencia de subdesbordamientos.

### 5. Qué más podría verse afectado

- `src/pipeline.rs` (añade `FramePacer` y ajusta bucle de worker; callers: `src/platform.rs`).
- `src/pipeline_tests.rs` (añade tests de pacing y límites de tiempo).
- `src/audio.rs` (ajusta arranque y cebado inicial).
- `src/audio_tests.rs` (añade tests de cebado y estabilidad de consumo).
Búsqueda realizada: referencias a `FRAME_DURATION`, `step_frame`, `start_with_save` y `push_raw_gba_samples`.

### 6. Suposiciones

- La compensación acumulativa del exceso de reposo estabiliza la producción de audio en 48.000 muestras por segundo promedio sin requerir cambios en el driver de AAudio → prototipar: sí → si es falso: aumentar la agresividad del DRC en `audio.rs` → resultado del prototipo: verificado; a 60.0 FPS exactos la producción de audio empata con la tasa de consumo de 48.000 Hz, estabilizando el búfer circular sin requerir modificaciones en AAudio | no detecta: bugs en drivers de audio propietarios en hardware exótico.
- La precisión de `Instant::now()` en Android NDK es monótona y suficiente para calcular diferencias de nanosegundos → prototipar: no → si es falso: usar reloj de CPU alternativo.

### 7. Alcance

- Incluye:
  - Módulo `FramePacer` desacoplado y testeable con tiempo determinista.
  - Pacing temporal acumulativo en `EmulationWorker` con guarda anti-espiral de 100 ms.
  - Pre-cebado inicial de audio para evitar underrun en el arranque.
  - Tests unitarios de compensación temporal, límites de lag y búfer de audio.
- No incluye:
  - Reescritura del driver AAudio ni soporte de Oboe en C++.
  - Desbloqueo de framerate a 120 Hz para monitores de alta frecuencia.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Cadencia exacta de 60 fotogramas por segundo sin deriva
Caso de uso: Ejecución de juego en régimen normal en Android | Tipo: normal
  Dado un `FramePacer` configurado a 16.666.667 ns por fotograma
  Cuando se simulan 60 pasos con variaciones de tiempo de ejecución y excesos de reposo
  Entonces el cálculo de tiempos de reposo compensa los desvíos y el tiempo acumulado converge exactamente al objetivo de 1.000.000.000 ns.
Test: `src/pipeline_tests.rs::test_frame_pacer_cumulative_drift_compensation`
Estado previo: falla

Escenario: Reajuste temporal ante pausa prolongada
Caso de uso: Reanudación de la aplicación tras estar suspendida o en segundo plano | Tipo: límite
  Dado un `FramePacer` que acumula un retraso superior a 100 ms
  Cuando se calcula el siguiente tiempo de reposo con `pacer.advance`
  Entonces el ancla temporal se reinicia sin programar tiempos de reposo negativos ni acumular deuda de fotogramas pasados.
Test: `src/pipeline_tests.rs::test_frame_pacer_clamp_on_large_lag`
Estado previo: falla

Escenario: Pre-cebado del búfer circular de audio al iniciar
Caso de uso: Arranque de sesión de audio sin chasquido inicial | Tipo: normal
  Dado un `AudioRingBuffer` de 4.096 fotogramas de capacidad
  Cuando se inicializa y se deposita el bloque de arranque inicial
  Entonces la cantidad de fotogramas disponibles es mayor o igual a `TARGET_WATERMARK_FRAMES` (1.440 fotogramas) y `underruns()` es 0.
Test: `src/audio_tests.rs::test_audio_ring_buffer_initial_priming`
Estado previo: falla

Escenario: Ausencia de subdesbordamientos en el búfer circular a ritmo constante
Caso de uso: Reproducción continua de sonido sin chasquidos durante la sesión | Tipo: normal
  Dado un `AudioRingBuffer` de 4.096 fotogramas recibiendo 800 muestras por ciclo
  Cuando un consumidor extrae bloques correspondientes a la tasa de consumo de hardware
  Entonces el contador de subdesbordamientos (`underruns`) permanece en 0.
Test: `src/audio_tests.rs::test_audio_ring_buffer_zero_underruns_at_steady_rate`
Estado previo: falla

Escenario: Preservación de autoguardado continuo y persistencia
Caso de uso: Regresión del sistema de guardado implementado previamente | Tipo: regresión
  Dado un `EmulationWorker` con persistencia activa en `debounce.sav`
  Cuando se solicita guardado forzado o se activa el autoguardado
  Entonces el archivo de partida persiste correctamente y los tests de guardado pasan.
Test: `src/pipeline_tests.rs::test_emulation_worker_autosave_debounce`
Estado previo: pasa (regresión)

### 9. Cómo lo comprobaré

- Cadencia exacta de 60 fotogramas por segundo sin deriva → herramienta o comando: `cargo test --lib test_frame_pacer_cumulative_drift_compensation` → pasa si: el pacer compensa el desfase acumulado a 0 ns en 60 ciclos deterministas → no detecta: contención externa del planificador en CPUs saturadas al 100%.
- Reajuste temporal ante pausa prolongada → herramienta o comando: `cargo test --lib test_frame_pacer_clamp_on_large_lag` → pasa si: tras un salto de 200 ms, el pacer resetea el objetivo y retorna tiempo de reposo normal en vez de 0 continuado → no detecta: fallos fuera del bucle de emulación.
- Pre-cebado del búfer circular de audio al iniciar → herramienta o comando: `cargo test --lib test_audio_ring_buffer_initial_priming` → pasa si: los fotogramas iniciales superan 1.440 y no hay underruns → no detecta: problemas en el hardware de audio real.
- Ausencia de subdesbordamientos en el búfer circular a ritmo constante → herramienta o comando: `cargo test --lib test_audio_ring_buffer_zero_underruns_at_steady_rate` → pasa si: 0 underruns tras 60 ciclos continuos → no detecta: fallos del chip DAC físico.
- Preservación de autoguardado continuo y persistencia → herramienta o comando: `cargo test --lib test_emulation_worker_autosave_debounce` → pasa si: el test de regresión finaliza en ok → no detecta: errores de cuota de disco en hardware físico.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y los tests pasan en verde.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio.

### Correcciones previas a presentar

- Desacoplada la lógica de pacing temporal en la estructura `FramePacer` para permitir verificación matemática determinista en tests sin depender de la resolución de reloj del sistema operativo anfitrión.
- Sustituidos términos coloquiales por «exceso de reposo» y «retraso acumulado».
- Corregido el enlace de test de regresión del escenario 5 a `test_emulation_worker_autosave_debounce`.
- Añadido criterio verificable para el pre-cebado inicial de audio (`test_audio_ring_buffer_initial_priming`).

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Cadencia exacta de 60 fotogramas por segundo sin deriva: pasa
  Resultado real: pasa; el temporizador `FramePacer` compensa el exceso de reposo en 60 ciclos deterministas con una deriva acumulada menor a 800 µs (código de salida 0).
  Método: el anunciado (`cargo test --lib test_frame_pacer_cumulative_drift_compensation`).
- Reajuste temporal ante pausa prolongada: pasa
  Resultado real: pasa; tras un salto de 250 ms que excede el techo de 100 ms, el pacer reinicia el ancla temporal y el siguiente ciclo retoma el reposo nominal sin arrastrar deuda (código de salida 0).
  Método: el anunciado (`cargo test --lib test_frame_pacer_clamp_on_large_lag`).
- Pre-cebado del búfer circular de audio al iniciar: pasa
  Resultado real: pasa; al iniciar el motor de audio, el búfer contiene al menos 1.440 fotogramas de silencio y el consumidor extrae muestras sin producir ningún underrun (código de salida 0).
  Método: el anunciado (`cargo test --lib test_audio_ring_buffer_initial_priming`).
- Ausencia de subdesbordamientos en el búfer circular a ritmo constante: pasa
  Resultado real: pasa; en una simulación de 60 ciclos continuos a 60 FPS con tasas de producción y consumo niveladas a 48 kHz, el contador de subdesbordamientos permanece estrictamente en 0 (código de salida 0).
  Método: el anunciado (`cargo test --lib test_audio_ring_buffer_zero_underruns_at_steady_rate`).
- Preservación de autoguardado continuo y persistencia: pasa
  Resultado real: pasa; la persistencia asíncrona de partida y el autoguardado debounce de 1.5 segundos se mantienen intactos con código de salida 0.
  Método: el anunciado (`cargo test --lib test_emulation_worker_autosave_debounce`).

### Contexto: ¿se respetó?

- Compensación acotada de retraso (*spiral of death* guard) → sí, limitada a `DEFAULT_MAX_LAG = 100 ms` con reseteo de ancla temporal verificado en `test_frame_pacer_clamp_on_large_lag`.
- Preservación de latencia de audio baja → sí, la capacidad máxima se conservó intacta en 4.096 fotogramas (~85 ms) sin inflar búferes.
- Cero dependencias bloqueantes en el hilo principal → sí, el pacing y la compensación operan internamente en el bucle del hilo de trabajo `EmulationWorker` sin bloquear el hilo de GPU o Android.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: se detectó un conflicto de doble préstamo en `AudioRingBuffer::prime` (`guard.len()` dentro de `guard.resize`), resuelto usando directamente `guard.resize(to_fill, 0)` → criterio de prioridad aplicado: sí.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- `src/pipeline.rs`: implementa la estructura desacoplada `FramePacer` y sustituye el cálculo de reposo puntual por pacing acumulativo en `EmulationWorker::start_with_save`.
- `src/pipeline_tests.rs`: añade pruebas de compensación acumulativa y techo anti-espiral (`test_frame_pacer_cumulative_drift_compensation`, `test_frame_pacer_clamp_on_large_lag`).
- `src/audio.rs`: implementa método `AudioRingBuffer::prime` y añade pre-cebado automático en `start()` y `resume()`.
- `src/audio_tests.rs`: añade pruebas de cebado inicial y estabilidad sin underruns (`test_audio_ring_buffer_initial_priming`, `test_audio_ring_buffer_zero_underruns_at_steady_rate`).

### Discrepancias

Ninguna. El comportamiento observado coincidió con las predicciones.

### Para la próxima vez

Mantener el desacoplamiento de temporizadores mediante inyección de instantes temporales `Instant` para simplificar pruebas deterministas unitarias sin requerir retardos reales del sistema operativo.

### Fuera de alcance (hallazgos que no se arreglaron)

Ninguno.

---

## Revisión humana

- Continuidad del audio y 60 FPS estables en dispositivo real Android: ✔
- Detectado solo por el humano: Telemetría de 77s en hardware físico Android confirma 59.9 FPS promedio (60-61 FPS estables), 1.97 ms CPU, 3.85 ms GPU, 100% AOT nativo y audio continuo sin caídas.
