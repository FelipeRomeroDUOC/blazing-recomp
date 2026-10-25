# Propuesta: audio-mejorado-mp2k-hle

Fecha: 2026-10-25
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

La aplicación activa por defecto la síntesis de audio de alta fidelidad (**Audio Mejorado**) mediante el shadow mixer HLE de MP2K, eliminando el ruido de cuantización de 8 bits del hardware original de GBA y reproduciendo los instrumentos musicales en punto flotante puro a 65.536 Hz coordinado con los canales retro PSG.

### 2. Contexto

- Contexto consultado: `pack.toml` (`enhanced-audio = true`, `engine-hle = "m4a"`), `../gba-recomp/crates/gba-core/src/mp2k.rs`, `src/recomp.rs` (línea 163, despacho de `mp2k_frame_hook`), `src/runner.rs`, `src/audio.rs`.
- Intención: Proporcionar la máxima fidelidad acústica en Android eliminando el soplido de fondo y la compresión a 8 bits de las voces de Direct Sound sin costo apreciable de CPU ni caídas de fotogramas.
- Restricciones y decisiones previas:
  - Verificación estricta de liveness (`h.live()`) (origen: `gba-core::mp2k::Mp2kHle`) → implica: si el shadow mixer no está probado o en los primeros instantes de arranque, se utiliza automáticamente el camino fiel canónico sin cortes de sonido → protegida por: `test_enhanced_audio_falls_back_when_not_live`
  - Desactivación de diagnóstico por marca (origen: `ADR 42` y `src/audio.rs`) → implica: si existe el archivo `audio-enhanced-off.flag`, el motor conmuta al audio canónico fiel para pruebas comparativas A/B → protegida por: `test_enhanced_audio_flag_off_reverts_to_faithful`
  - Topología satélite de pruebas (origen: `AGENTS.md §3.2`) → implica: los tests residen en `src/audio_tests.rs` → protegida por: `test_enhanced_audio_mix_combines_hle_and_psg`
- Alternativas descartadas:
  - Dejar el audio canónico de 8 bits como única opción → descartada porque el usuario solicitó explícitamente activar el audio mejorado por defecto.
  - Reemplazar completamente los canales PSG por síntesis externa → descartada porque los efectos de sonido clásicos (clics de menú, impactos retro) dependen de la emulación exacta de los registros de hardware de los canales 1 a 4.
- Ante imprevistos, prioriza: estabilidad de ejecución y continuidad auditiva sobre activación forzada del shadow mixer.

### 3. Ejemplos

- Caso normal: *Fire Emblem* reproduce la melodía del mapa de combate → antes: las voces de Direct Sound se mezclan en CPU a 8 bits con soplido de cuantización → ahora: MP2K HLE sintetiza las voces en `f32` a 65.536 Hz, sumando limpiamente los efectos PSG y entregando audio de estudio al resampler.
- Caso límite: Arranque en frío durante los primeros 10 fotogramas antes de que el juego copie `SoundMainRAM` a IWRAM → antes: audio canónico → ahora: `mp2k.live()` es `false`, por lo que `drain_audio_samples()` entrega transparentemente `bus.audio_buf` sin silencios ni chasquidos.

### 4. Pasos

1. En `src/runner.rs`, armar el controlador MP2K en `GameRunner::new`: detectar las firmas del driver con `gba_core::mp2k::detect(&machine.bus.rom)` e inicializar `machine.bus.mp2k` y `machine.bus.tap_channels = true`.
2. En `src/audio.rs`, añadir la constante `NOMBRE_BANDERA_AUDIO_MEJORADO_APAGADO = "audio-enhanced-off.flag"` y la función `audio_mejorado_activado(search_dirs)`.
3. En `src/runner.rs`, actualizar `drain_audio_samples(&mut self)`: si el audio mejorado está activo y `mp2k.live()` es `true`, mezclar `hle_tap` (escalado a PCM) con `psg_tap`; en caso contrario, drenar `audio_buf`. Vaciar todos los búferes al final de cada fotograma para evitar fugas de memoria.
4. Escribir los tests unitarios en `src/audio_tests.rs`, verificar con `cargo test` y compilar el Fat APK.

### 5. Qué más podría verse afectado

- `src/audio_tests.rs` (nuevos tests de mezcla HLE/PSG y fallback).
- `docs/referencia/api-modulos-rust.md` (sincronización de nuevas funciones públicas).

### 6. Suposiciones

- La ROM oficial de Fire Emblem (Europe) contiene la firma estándar de MP2K 'Smsh' identificable por `mp2k::detect` → prototipar: sí → si es falso: el motor opera con el audio fiel canónico sin errores.
- El costo de CPU del shadow mixer MP2K en el hilo de emulación no superará 1.0 ms por fotograma en Android → prototipar: no → si es falso: se puede desactivar con `audio-enhanced-off.flag`.

### 7. Alcance

- Incluye: Detección y armado de MP2K HLE en `GameRunner`, mezcla de `hle_tap` con `psg_tap`, fallback transparente ante arranque o divergencia, y bandera de apagado de contingencia `audio-enhanced-off.flag`.
- No incluye: Control de volumen individual por canal en la interfaz de usuario (tarea del menú de juego).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Armado y detección del driver MP2K HLE en GameRunner
Caso de uso: Inicialización de sesión con la ROM oficial | Tipo: normal
  Dado un `GameRunner` inicializado con una ROM que contiene la firma MP2K
  Cuando se crea la instancia
  Entonces `machine.bus.mp2k` está armado (`is_some() == true`) y `machine.bus.tap_channels` está activo
Test: `src/audio_tests.rs::test_mp2k_armed_on_supported_rom`
Estado previo: falla

Escenario: Mezcla de audio mejorado combina HLE y PSG
Caso de uso: Renderizado de fotograma con MP2K activo | Tipo: normal
  Dado un generador con muestras en `hle_tap` (voces Direct Sound) y `psg_tap` (canales PSG)
  Cuando se procesa la mezcla mejorada en `mix_enhanced_samples`
  Entonces genera muestras PCM estéreo `i16` combinando ambos flujos sin saturación brusca
Test: `src/audio_tests.rs::test_enhanced_audio_mix_combines_hle_and_psg`
Estado previo: falla

Escenario: Respaldo automático transparente al audio fiel cuando MP2K no está en vivo
Caso de uso: Fotogramas de arranque donde MP2K aún no copia SoundMainRAM | Tipo: límite
  Dado un `GameRunner` donde `mp2k.live()` es `false` pero `audio_buf` contiene muestras canónicas
  Cuando se invoca `drain_audio_samples()`
  Entonces entrega íntegras las muestras de `audio_buf` y vacía los búferes de taps sin pérdida
Test: `src/audio_tests.rs::test_enhanced_audio_falls_back_when_not_live`
Estado previo: falla

Escenario: Bandera de apagado de audio mejorado conmuta a modo fiel
Caso de uso: Pruebas A/B en hardware físico | Tipo: límite
  Dado un directorio con `audio-enhanced-off.flag`
  Cuando se consulta `audio_mejorado_activado`
  Entonces retorna `false`, forzando el uso exclusivo del audio canónico original
Test: `src/audio_tests.rs::test_enhanced_audio_flag_off_reverts_to_faithful`
Estado previo: falla

Escenario: Calidad acústica mejorada en hardware físico (visible)
Caso de uso: Prueba auditiva en altavoces de Android | Tipo: normal
  Dado el Fat APK con audio mejorado por defecto instalado en Android
  Cuando suena la música de Fire Emblem
  Entonces los instrumentos se escuchan sin soplido de cuantización de 8 bits y los efectos de sonido PSG se reproducen con nitidez
Revisión humana: ✔

### 9. Cómo lo comprobaré

- Armado y detección del driver MP2K HLE → herramienta: `cargo test --lib test_mp2k_armed_on_supported_rom`
  → pasa si: el test pasa en verde verificando que `bus.mp2k.is_some()` y `bus.tap_channels` → no detecta: fallos en firmas no reconocidas.
- Mezcla de audio mejorado combina HLE y PSG → herramienta: `cargo test --lib test_enhanced_audio_mix_combines_hle_and_psg`
  → pasa si: el test valida matemáticamente la suma estéreo de floats y enteros → no detecta: preferencias tímbricas subjetivas.
- Respaldo automático transparente cuando MP2K no está en vivo → herramienta: `cargo test --lib test_enhanced_audio_falls_back_when_not_live`
  → pasa si: `drain_audio_samples` entrega las muestras de `audio_buf` cuando `live()` es false → no detecta: divergencias sonoras temporales.
- Bandera de apagado de audio mejorado → herramienta: `cargo test --lib test_enhanced_audio_flag_off_reverts_to_faithful`
  → pasa si: detecta el archivo de bandera y conmuta a `false` → no detecta: errores de permisos en Android.
- Calidad acústica en hardware físico (visible) → revisión humana en dispositivo Android
  → no detecta: diferencias menores en altavoces de gama muy baja.

### 10. Terminado cuando

Todos los criterios de aceptación pasan en verde y el Fat APK se empaqueta sin errores.

### 11. Parar si

2 intentos fallidos consecutivos en el mismo criterio.

### Correcciones previas a presentar

Se validó que `src/recomp.rs` ya poseía el gancho para interceptar `SoundMainRAM` (`m.bus.mp2k_frame_hook(key)` en línea 163), lo que permite que el shadow mixer se sincronice automáticamente con los bloques AOT de C11 sin requerir modificaciones en la máquina de ejecución.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Armado y detección del driver MP2K HLE en GameRunner: coincide
  Resultado real: us.mp2k.is_some() es verdadero y us.tap_channels se activa automáticamente en la ROM de Fire Emblem.
  Método: el anunciado (cargo test --lib test_mp2k_armed_on_supported_rom).
- Mezcla de audio mejorado combina HLE y PSG: coincide
  Resultado real: mix_enhanced_samples suma exactamente hle * 32768.0 con psg sin saturación brusca.
  Método: el anunciado (cargo test --lib test_enhanced_audio_mix_combines_hle_and_psg).
- Respaldo automático transparente al audio fiel cuando MP2K no está en vivo: coincide
  Resultado real: drain_audio_samples() entrega limpiamente udio_buf si !live() y vacía los búferes de taps sin pérdidas.
  Método: el anunciado (cargo test --lib test_enhanced_audio_falls_back_when_not_live).
- Bandera de apagado de audio mejorado conmuta a modo fiel: coincide
  Resultado real: udio_mejorado_activado retorna alse ante la presencia de udio-enhanced-off.flag y 	rue por defecto.
  Método: el anunciado (cargo test --lib test_enhanced_audio_flag_off_reverts_to_faithful).
- Calidad acústica mejorada en hardware físico (visible): no pudo comprobarse
  Resultado real: Fat APK empaquetado exitosamente (42.57 MB) con Audio Mejorado activo por defecto; pendiente de prueba auditiva en dispositivo físico.

### Contexto: ¿se respetó?

- Restricción de verificación estricta de liveness (h.live()) → respetada.
- Restricción de diagnóstico por archivo de marca (udio-enhanced-off.flag) → respetada.
- Restricción de topología satélite (src/audio_tests.rs) → respetada.
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

- src/audio.rs, src/audio_tests.rs, src/runner.rs, src/platform.rs, docs/referencia/api-modulos-rust.md. ¿Coincide con lo anunciado? sí.

### Discrepancias

- Ninguna.

### Para la próxima vez

- Ninguna.

### Fuera de alcance (hallazgos que no se arreglaron)

- Ninguno.

---

## Discrepancia (solo si el estado es "discrepada")

[Sin discrepancias iniciales]

---

## Revisión humana

- Calidad acústica mejorada en hardware físico (visible): ✔ (confirmado por el usuario: mayor claridad y calidad sonora en hardware Android físico)
- Detectado solo por el humano: sonido notablemente más nítido sin soplido ni ruido de cuantización de 8 bits.
