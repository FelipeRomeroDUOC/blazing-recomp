# Propuesta: motor-audio-nativo-aaudio

Fecha: 2026-10-25
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La aplicación ahora reproduce música y efectos sonoros de Game Boy Advance en tiempo real a través del subsistema de audio nativo de baja latencia de Android (AAudio), manteniendo el sonido y la imagen sincronizados a 60 FPS sin retrasos ni distorsiones.

### 2. Contexto

- Contexto consultado: `docs/PENDIENTES.md` §1 (Audio Nativo), `docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md` (Fase 3), `../gba-recomp/crates/gba-core/src/mem.rs` (`pump_audio`, `audio_buf`), `src/pipeline.rs`, `src/platform.rs`, `Cargo.toml`.
- Intención: Proporcionar salida de audio audible nativa de alta fidelidad y baja latencia en Android con AAudio sin emulación por software de la capa de audio del sistema operativo, desacoplada mediante un búfer circular y sincronizada con el bucle de presentación de 60 FPS.
- Restricciones y decisiones previas:
  - `minSdkVersion = 26` en AndroidManifest.xml (origen: `android/AndroidManifest.xml`) → implica: AAudio está disponible de forma nativa en el 100% de dispositivos soportados, sin recurrir a OpenSL ES ni JNI Java AudioTrack → protegida por: `test_audio_backend_configuration_aaudio` | no testeable
  - Topología satélite de pruebas (origen: `AGENTS.md §3.2`) → implica: tests de audio residen en `src/audio_tests.rs` enlazados mediante `#[path = "audio_tests.rs"] mod tests;` → protegida por: `test_audio_resampler_and_ring_buffer_transfer`
  - Contingencia de diagnóstico por archivo de marca (origen: `ADR 42` y `src/ppu_nativa.rs`) → implica: el audio puede apagarse en caliente si existe `audio-off.flag` en los directorios de telemetría → protegida por: `test_audio_flag_off_disables_stream`
- Alternativas descartadas:
  - Usar Java `AudioTrack` mediante JNI → descartada por añadir sobrecarga de recolección de basura (GC) y latencias inestables superiores a 40-60 ms.
  - Usar OpenSL ES → descartada por ser una API obsoleta (*deprecated*) reemplazada por AAudio en Android 8.0+.
  - Modificar el reloj maestro de emulación en función del audio (*audio-driven clock*) bloqueando el hilo de presentación → descartada porque en Android el ritmo de fotogramas está gobernado por el Choreographer/VSync a 60 Hz; en su lugar se usa compensación dinámica de deriva (*dynamic rate control*).
- Ante imprevistos, prioriza: estabilidad de fotogramas a 60 FPS sobre fidelidad de muestreo extremo.

### 3. Ejemplos

- Caso normal: El juego arranca y pasa a la pantalla de título → antes: `audio_buf` se llena hasta `0x2_0000` y el dispositivo permanece en silencio absoluto consumiendo ciclos de APU en vacío → ahora: `drain_audio_samples()` entrega ~2.184 muestras por fotograma, se remuestrean a 48 kHz y AAudio emite la melodía principal con latencia inferior a 25 ms.
- Caso límite: El hilo emulador experimenta un tirón de fotograma (*frame drop*) y la cola de audio se vacía momentáneamente → antes: silencio total → ahora: el consumidor rellena el faltante con atenuación a cero (*soft fade/zero-fill*) evitando chasquidos o clics (*buffer pops*) y reporta *underrun* en contadores.

### 4. Pasos

1. Habilitar la característica `audio` en la dependencia `ndk` en `Cargo.toml` para `cfg(target_os = "android")`.
2. Crear el módulo `src/audio.rs` con el búfer circular SPSC acotado (`AudioRingBuffer`), el remuestreador lineal con acumulador de punto fijo (`AudioResampler`), el control dinámico de deriva de reloj (`DynamicRateControl`) y el backend nativo AAudio / Null backend para host.
3. Exponer `drain_audio_samples()` en `GameRunner` (`src/runner.rs`) para vaciar `machine.bus.audio_buf` al final de cada fotograma.
4. Integrar el pipeline de audio en `EmulationWorker` (`src/pipeline.rs`) para transferir muestras remuestreadas hacia el búfer circular sin bloquear el hilo emulador.
5. Enlazar el ciclo de vida en `src/platform.rs` (`android_main`): inicializar el flujo AAudio al arrancar, pausarlo en `AppEvent::Pause` y reanudarlo en `AppEvent::Resume`.
6. Validar con tests unitarios satélite en `src/audio_tests.rs`, sincronizar documentación con `scripts/extraer_referencias.py` y compilar el Fat APK.

### 5. Qué más podría verse afectado

- `src/lib.rs` (declaración de `pub mod audio;`).
- `docs/referencia/api-modulos-rust.md` (actualización de firmas públicas del nuevo módulo).
- `tests/packaging_tests.rs` (verificación de sincronía de documentación de referencia).

### 6. Suposiciones

- AAudio en `ndk = 0.9.0` enlaza limpiamente con `libaaudio.so` en Android NDK sin dependencias adicionales de compilación en `Cargo.toml` → prototipar: sí → si es falso: enlazar explícitamente `#[link(name = "aaudio")]` en un módulo FFI local.
- El formato de muestra nativo preferente de la mayoría de los dispositivos Android modernos es PCM 16-bit estéreo o Float32 a 48.000 Hz → prototipar: no → si es falso: el resampler soporta cualquier frecuencia arbitraria consultada al flujo abierto mediante `stream.get_sample_rate()`.

### 7. Alcance

- Incluye: Generador de audio AAudio de baja latencia en Android, remuestreo lineal de 65.536 Hz a frecuencia de hardware, búfer circular protegido contra caídas, control dinámico de sincronización A/V y pausa/reanudación con el ciclo de vida de la actividad.
- No incluye: Síntesis HLE en punto flotante del shadow mixer MP2K (Fase posterior de optimización acústica; se utiliza la mezcla canónica fiel de `gba-core`), selector de volumen en menú interactivo (corresponde a la tarea del menú de juego).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Resampling lineal exacto y transferencia al búfer circular
Caso de uso: Remuestreo de 65.536 Hz a 48.000 Hz entre emulador y hardware de audio | Tipo: normal
  Dado un lote de 1.092 frames estéreo a 65.536 Hz (2.184 muestras `i16`) generado por el APU
  Cuando se procesa con `AudioResampler::resample` hacia 48.000 Hz y se deposita en `AudioRingBuffer`
  Entonces produce exactamente 800 frames estéreo (±1 por redondeo de fase) y el buffer los entrega íntegros al consumidor sin alteración de datos
Test: `src/audio_tests.rs::test_audio_resampler_and_ring_buffer_transfer`
Estado previo: falla

Escenario: Resistencia a subdesbordamiento del búfer circular sin caídas
Caso de uso: Comportamiento ante tirones de fotograma o arranque en frío | Tipo: límite
  Dado un `AudioRingBuffer` con solo 64 frames estéreo disponibles
  Cuando el consumidor solicita leer 256 frames estéreo
  Entonces entrega los 64 frames válidos, rellena los 192 restantes con ceros sin producir pánico, e incrementa en 1 el contador de *underruns*
Test: `src/audio_tests.rs::test_audio_ring_buffer_underrun_graceful_fill`

Escenario: Drenaje de `machine.bus.audio_buf` evita acumulación y respeta presupuesto de memoria
Caso de uso: Prevención de fuga de memoria o estancamiento de muestras tras múltiples fotogramas | Tipo: regresión
  Dado un `GameRunner` simulando 10 fotogramas consecutivos
  Cuando se invoca `drain_audio_samples()` al término de cada fotograma
  Entonces `runner.machine.bus.audio_buf.len()` nunca supera las 2.300 muestras al final de cada paso y no se trunca la generación de audio
Test: `src/audio_tests.rs::test_audio_buf_drained_each_frame_bounds_memory`

Escenario: Detección de bandera de apagado de audio en caliente
Caso de uso: Diagnóstico y contingencia en hardware físico para pruebas A/B | Tipo: límite
  Dado un directorio temporal con el archivo `audio-off.flag`
  Cuando se consulta `audio_apagado_por_marca`
  Entonces retorna `true`, instruyendo al motor a suspender el flujo de audio
Test: `src/audio_tests.rs::test_audio_flag_off_disables_stream`

Escenario: Salida de sonido audible de baja latencia en hardware físico (visible)
Caso de uso: Verificación acústica en dispositivo Android | Tipo: normal
  Dado el Fat APK instalado en un dispositivo Android físico
  Cuando inicia el juego y avanza la introducción/título
  Entonces la música y los efectos sonoros se escuchan claramente por los altavoces sin chasquidos ni retardo perceptible con la animación
Revisión humana: ✔

### 9. Cómo lo comprobaré

- Resampling lineal exacto y transferencia al búfer circular → herramienta: `cargo test --lib test_audio_resampler_and_ring_buffer_transfer`
  → pasa si: el test pasa en verde con 0 fallos → no detecta: fallos específicos del controlador de hardware del dispositivo móvil.
- Resistencia a subdesbordamiento del búfer circular sin caídas → herramienta: `cargo test --lib test_audio_ring_buffer_underrun_graceful_fill`
  → pasa si: el test pasa en verde verificando relleno seguro y contador de fallos → no detecta: artefactos psicoacústicos sutiles en el oído humano.
- Drenaje de `machine.bus.audio_buf` evita acumulación y respeta presupuesto de memoria → herramienta: `cargo test --lib test_audio_buf_drained_each_frame_bounds_memory`
  → pasa si: `len()` se mantiene dentro del umbral de 1 fotograma a lo largo de los 10 pasos → no detecta: comportamiento tras horas de emulación ininterrumpida.
- Detección de bandera de apagado de audio en caliente → herramienta: `cargo test --lib test_audio_flag_off_disables_stream`
  → pasa si: detecta la presencia del archivo de bandera y retorna `true` → no detecta: permisos de lectura en directorios restringidos del sistema Android.
- Salida de sonido audible de baja latencia en hardware físico (visible) → revisión humana: escuchar la melodía de arranque en el dispositivo Android físico
  → no detecta: desajustes de fase menores a 10 ms imperceptibles para el oído humano.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y el Fat APK se compila exitosamente.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

Se descartó la alternativa de regular el reloj del emulador basándose en el consumo del flujo de audio (*audio-driven pacing*) para no romper la cadencia fija a 60 FPS del `Choreographer` en Android; se adoptó en su lugar un remuestreador dinámico con ajuste fino de deriva (*dynamic rate control*).

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Resampling lineal exacto y transferencia al búfer circular: coincide
  Resultado real: 800 frames producidos a 48.000 Hz a partir de 1.092 frames a 65.536 Hz, transferidos íntegros a AudioRingBuffer.
  Método: el anunciado (cargo test --lib test_audio_resampler_and_ring_buffer_transfer).
- Resistencia a subdesbordamiento del búfer circular sin caídas: coincide
  Resultado real: 64 frames válidos leídos, 192 frames restantes rellenados con ceros sin pánico, contabilizando 1 evento de underrun.
  Método: el anunciado (cargo test --lib test_audio_ring_buffer_underrun_graceful_fill).
- Drenaje de machine.bus.audio_buf evita acumulación y respeta presupuesto de memoria: coincide
  Resultado real: en 10 pasos continuos, udio_buf.len() permanece < 2.300 muestras por paso sin estancamiento.
  Método: el anunciado (cargo test --lib test_audio_buf_drained_each_frame_bounds_memory).
- Detección de bandera de apagado de audio en caliente: coincide
  Resultado real: udio_apagado_por_marca retorna 	rue con el archivo udio-off.flag y alse sin él.
  Método: el anunciado (cargo test --lib test_audio_flag_off_disables_stream).
- Salida de sonido audible de baja latencia en hardware físico (visible): no pudo comprobarse
  Resultado real: Fat APK compilado y empaquetado exitosamente (44.62 MB); requiere despliegue en dispositivo físico.

### Contexto: ¿se respetó?

- Restricción minSdkVersion = 26 con AAudio nativo sin JNI → respetada.
- Restricción de topología satélite (src/audio_tests.rs) → respetada.
- Restricción de contingencia por archivo de marca (udio-off.flag) → respetada.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- Cargo.toml, Cargo.lock, src/lib.rs, src/audio.rs, src/audio_tests.rs, src/runner.rs, src/pipeline.rs, src/platform.rs, docs/referencia/api-modulos-rust.md. ¿Coincide con lo anunciado? sí.

### Discrepancias

- Ninguna.

### Para la próxima vez

- Evaluar la activación opcional del shadow mixer flotante HLE MP2K (hle_tap) cuando el menú de configuración de audio esté disponible.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno.

---

## Discrepancia (solo si el estado es "discrepada")

[Sin discrepancias iniciales]

---

## Revisión humana

- Salida de sonido audible de baja latencia en hardware físico (visible): ✔ (confirmado por el usuario: audio nítido y sin latencia en sesión física de 961s)
- minSdkVersion = 26 garantiza soporte nativo de AAudio en todo dispositivo compatible: ✔
- Detectado solo por el humano: telemetría sostenida durante 16 minutos a 59.6 FPS promedio con audio activo continuo.
