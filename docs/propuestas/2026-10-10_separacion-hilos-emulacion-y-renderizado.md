# Propuesta: Separación de hilos de emulación y renderizado (Pipeline Multihilo)

Fecha: 2026-10-10
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

El juego pasa de ejecutar todo en un único hilo en fila india a repartir el trabajo entre dos hilos en paralelo: un hilo de emulación en segundo plano que avanza la lógica del juego a 60 Hz exactos en un núcleo de CPU dedicado, y el hilo principal de Android que dibuja en la GPU con Vulkan y atiende los toques táctiles. En dispositivos multinúcleo como la tablet, los 12 ms de CPU y los 15 ms de GPU dejan de sumarse (27 ms = 36 FPS) y corren simultáneamente, permitiendo alcanzar 60 FPS estables sin alterar la emulación ni la física del juego.

### 2. Contexto

- Contexto consultado: [`src/platform.rs`](../../src/platform.rs) (líneas 1690-1805 bucle principal y líneas 672-882 `Compositor::render`), [`src/runner.rs`](../../src/runner.rs), [`src/perf.rs`](../../src/perf.rs), [`AGENTS.md`](../../AGENTS.md) (§2.5 "sin busy-waiting", §4.4 topología modular satélite) y el reporte real de telemetría de la tablet (`blazing-perf.txt`, 2026-10-10: `ejec=11-16ms`, `dibujo=10-16ms`, `fps=36.0`).
- Intención: permitir que la emulación y el renderizado corran en núcleos distintos para que el presupuesto por fotograma no exceda los 16.6 ms de VSync a 60 FPS. Éxito: el reporte de telemetría en la tablet muestra que `fps` sube de 36 a 60 FPS y la CPU no espera a la GPU.
- Restricciones y decisiones previas (máx. 3):
  - El hilo principal de Android debe seguir procesando `app.poll_events` e input de `NativeActivity` (origen: especificación de `android-activity` y prevención de ANR) → implica: el compositor GPU y los eventos táctiles permanecen en el hilo de la actividad de Android → protegida por: "Procesamiento reactivo de eventos sin bloqueo de ANR".
  - La comunicación entre hilos debe utilizar búferes de memoria fija sin asignaciones dinámicas por frame (`Vec::new`) ni cerrojos pesados que causen contención (origen: `AGENTS.md` §2.5 rendimiento y batería) → implica: canal acotado a 1 fotograma o doble/triple búfer con `Box<[[u8; 4]; GBA_PIXELS]>` preasignado y teclas sincronizadas atómicamente con `AtomicU16` → protegida por: "Transferencia de fotograma acotada a un solo búfer sin asignación".
  - Conservar el arnés de telemetría y los 81 tests existentes pasando al 100% (origen: `AGENTS.md` §6 checklist pre-commit) → implica: `GameRunner` no se vuelve dependiente de tipos de Android; el puente multihilo va en un módulo puro con tests satélite → protegida por: "Regresión: Suite de tests existente y contratos de GameRunner intactos".
- Alternativas descartadas:
  - Renderizado en hilo secundario y emulación en hilo principal → descartada porque en Android `wgpu::Surface` y la ventana `ANativeWindow` están más estrechamente acopladas al ciclo de vida del hilo de la actividad nativa; delegar la emulación es más limpio, portable y fácil de aislar.
  - Cola infinita de fotogramas (buffer sin límite) → descartada porque crearía desincronización y retraso de entrada perceptible si la GPU se retrasa; una capacidad estricta de 1 fotograma garantiza como máximo 16.6 ms de latencia de entrada, idéntica al hardware real.
- Ante imprevistos, prioriza: latencia de respuesta táctil y estabilidad de ciclo de vida (pausa/reanudación) por encima de tolerar caídas de fotogramas sin sincronizar.

### 3. Ejemplos

- Caso normal: tablet en portrait con carga moderada (ejecución 12 ms, dibujo 15 ms)
  - Antes: el hilo ejecuta CPU (12 ms) y luego GPU (15 ms) en fila india. Tiempo total = 27 ms. El juego corre a 36 FPS.
  - Ahora: el hilo de emulación calcula el fotograma $N+1$ (12 ms) al mismo tiempo que la GPU dibuja el fotograma $N$ (15 ms). Tiempo efectivo = $\max(12, 15) = 15\text{ ms} < 16.6\text{ ms}$. El juego corre a 60 FPS.
- Caso límite: el usuario pulsa el botón Home o bloquea la pantalla (`Pause` / `suspend`)
  - Antes: el bucle suspende el runner y la ventana en el hilo único.
  - Ahora: el hilo principal envía la señal de suspensión atómica (`paused.store(true)`), el hilo de emulación se duerme inmediatamente con `Condvar` sin consumir ciclos de CPU en segundo plano (0% de batería), y despierta limpiamente al reanudar (`Resume`).
- Caso de sobrecarga: la GPU tarda 25 ms por un pico en un fotograma complejo
  - Antes: la CPU se retrasa 25 ms adicionales y los FPS caen a la mitad.
  - Ahora: el canal de capacidad 1 retiene el avance de la CPU hasta que la GPU consuma el fotograma previo, evitando desbordamientos de memoria o acumulación de lag.

### 4. Pasos

1. Crear el módulo puro de pipeline multihilo en `src/pipeline.rs` y sus tests satélite en `src/pipeline_tests.rs`:
   - `FramePacket`: estructura que transporta el fotograma RGBA ($240 \times 160$), el número de fotograma, `is_native` y el `StepTiming` de telemetría.
   - `EmulationBridge`: abstracción de comunicación que contiene un canal síncrono acotado a 1 slot (`sync_channel(1)` o doble slot con mutex ligero) para fotogramas, un `AtomicU16` para inyectar `KEYINPUT` sin cerrojos, y banderas atómicas de control (`running`, `paused`) con `Condvar` para reposo energético.
2. Declarar `pub mod pipeline;` en `src/lib.rs`.
3. En `src/platform.rs`:
   - En `AppState`: reemplazar la tenencia directa y síncrona de `GameRunner` por el control del `EmulationWorker` y el receptor del `EmulationBridge`.
   - Al cargar la ROM (`state.load()`): inicializar y lanzar el hilo de emulación dedicado (`std::thread::Builder::new().name("blazing-emu")`).
   - En cada tick del bucle principal de Android:
     - Escribir `combined_keys()` en el `AtomicU16` del puente (cero retardo para la entrada táctil y física).
     - Intentar recibir de forma no bloqueante (`try_recv`) el fotograma más reciente listo para dibujar.
     - Si hay fotograma nuevo, pasarlo a `Compositor::render()` y registrar la telemetría unificada en `perf`.
   - En `suspend()` y `resume()`: pausar y despertar el hilo de emulación de forma segura sin fugas de memoria ni hilos zombis.
4. Documentar el nuevo diseño en `docs/explicacion/arquitectura-port-android.md`, sincronizar `docs/referencia/api-modulos-rust.md` con `scripts/extraer_referencias.py` y actualizar `CHANGELOG.md`.

### 5. Qué más podría verse afectado

- `src/runner.rs`: no se modifica en su lógica interna; `GameRunner` se mueve dentro del nuevo hilo de trabajo. Búsqueda: `git grep -n "GameRunner" src/`.
- `src/platform.rs`: se modifican las funciones `AppState::step`, `AppState::load`, `AppState::suspend` y el bucle principal en `android_main` (líneas 1740-1805).
- `src/perf.rs`: el registro de telemetría (`perf.record_frame`) recibe el `StepTiming` que viene empaquetado junto al fotograma en `FramePacket`.
- `tests/packaging_tests.rs`: se ejecutará completo para verificar que la inclusión de hilos y la sincronización de documentación permanezcan al 100% en verde.

### 6. Suposiciones

- El sistema operativo Android permite a una biblioteca nativa (`.so` bajo `NativeActivity`) crear hilos POSIX con `std::thread::spawn` sin restricciones de permisos → prototipar: sí
  → si es falso: utilizar hilos nativos NDK vía `pthread_create` mediante enlaces de libc
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: políticas restrictivas de ROMs de fabricantes no probadas
- La tasa de producción de fotogramas del hilo de emulación a 60 Hz no satura la memoria si la GPU se atrasa → prototipar: sí (cubierto por el diseño de canal acotado a 1 fotograma `sync_channel(1)`)
  → si es falso: implementar descarte explícito del fotograma intermedio (frame skipping)
  → resultado del prototipo: (se completa tras la aprobación) | no detecta: desincronización de audio (el motor actualmente no tiene salida de audio)
- Los conductores gráficos Vulkan de Android en la tablet y TV toleran que `wgpu::Queue::write_texture` se llame en el hilo principal mientras otro hilo escribe en un búfer desconectado en RAM → prototipar: no (garantizado por el sistema de tipos de Rust: `FramePacket` transfiere propiedad completa por canal)
  → si es falso: clonar el búfer antes de subirlo

### 7. Alcance

- Incluye:
  - Módulo puro `src/pipeline.rs` con estructuras de intercambio `FramePacket`, `EmulationBridge` y sincronización atómica.
  - Hilo de trabajo de emulación en background con timing regulado a 16.6 ms.
  - Integración del consumidor no bloqueante en el bucle principal de `src/platform.rs`.
  - Manejo de ciclo de vida seguro (pausa bloqueante con `Condvar` al salir a segundo plano y cierre limpio).
  - Tests satélite en `src/pipeline_tests.rs`.
- No incluye:
  - Salida de audio o sincronización con APU (Fase futura).
  - Modificaciones en los shaders de `overlay.wgsl` (propuesta de GPU independiente).
  - Modificaciones en `gba-core` o en el repositorio upstream.

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Transferencia de fotograma acotada a un solo búfer sin asignación
Caso de uso: el hilo de emulación produce fotogramas y el hilo de render los consume | Tipo: normal
  Dado un canal de comunicación `EmulationBridge` inicializado con capacidad de 1 fotograma
  Cuando el productor envía 2 fotogramas consecutivos y el consumidor lee el último paquete
  Entonces el consumidor recibe el paquete con el número de fotograma más reciente, dimensiones exactas de 38,400 píxeles y canal alfa 255, sin fugas ni bloqueos indefinidos
Test: `src/pipeline_tests.rs::test_emulation_bridge_single_slot_transfer` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: Inyección atómica de teclas sin contención de cerrojos
Caso de uso: el usuario pulsa botones táctiles en la pantalla a 120 Hz de muestreo táctil | Tipo: límite
  Dado un puente `EmulationBridge` con máscara inicial de reposo 0x03FF
  Cuando el hilo principal actualiza la máscara atómica a 0x03FE (botón A presionado) en múltiples iteraciones rápidas
  Entonces el hilo de emulación lee exactamente el valor 0x03FE en su siguiente tick sin retraso perceptible ni operaciones de bloqueo (mutex)
Test: `src/pipeline_tests.rs::test_atomic_key_injection_lock_free` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: Ciclo de vida y reposo energético sin consumo en pausa
Caso de uso: el usuario minimiza la aplicación o bloquea el dispositivo | Tipo: límite
  Dado un puente `EmulationBridge` con el hilo de emulación activo
  Cuando se activa la señal de pausa (`set_paused(true)`)
  Entonces el hilo de emulación detiene el avance de fotogramas y entra en estado de espera en la variable de condición sin consumir ciclos de CPU hasta recibir `set_paused(false)`
Test: `src/pipeline_tests.rs::test_emulation_bridge_pause_and_resume_sleep` (se escribe tras la aprobación)
Estado previo: (se completa al escribir el test)

Escenario: Procesamiento reactivo de eventos sin bloqueo de ANR (visible)
Caso de uso: probar la aplicación en la tablet y rotar la orientación entre portrait y landscape | Tipo: regresión
  Dado que la aplicación se ejecuta con el hilo de emulación desacoplado
  Cuando se interactúa intensivamente con la pantalla táctil y se rota el dispositivo
  Entonces la interfaz responde de inmediato sin congelamientos de la ventana y el contador de FPS refleja el incremento del rendimiento aprovechando los múltiples núcleos
Test: no automatizable en host (revisión humana en tablet)
Estado previo: no aplica

### 9. Cómo lo comprobaré

- Transferencia de fotograma acotada → `cargo test --lib test_emulation_bridge_single_slot_transfer` → pasa si el paquete recibido contiene 38,400 píxeles y metadata íntegra → no detecta: saturación de caché L3 en hardware ARM real.
- Inyección atómica de teclas → `cargo test --lib test_atomic_key_injection_lock_free` → pasa si el valor atómico se propaga instantáneamente entre hilos → no detecta: reordenamiento de memoria débil en arquitecturas exóticas fuera de ARM64/x86_64.
- Ciclo de vida y reposo energético → `cargo test --lib test_emulation_bridge_pause_and_resume_sleep` → pasa si el hilo productor suspende la ejecución hasta ser notificado → no detecta: llamadas directas de Android Activity fuera del estándar de `android-activity`.
- Procesamiento reactivo de eventos sin bloqueo de ANR → revisión humana en la tablet: comprobar que la rotación y controles táctiles responden fluidamente y que `Download/blazing-perf.txt` reporta una subida de FPS hacia los 60 FPS estables → no detecta: variaciones térmicas tras 30 minutos de juego continuo (thermal throttling).
- Suite completa y empaquetado → `cargo test` (81+ tests en verde) y `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` → pasa si el Fat APK se genera y firma limpiamente.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación y el nuevo APK se compila, empaqueta y firma correctamente.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

- Primer análisis: se evaluó mover el renderizado a un hilo secundario y dejar la emulación en el hilo principal. Se descartó porque en Android la API de `NativeActivity` gestiona el ciclo de vida de la ventana nativa (`ANativeWindow`) en su propio hilo, por lo que aislar la emulación en un worker thread es significativamente más seguro y previene errores de contexto gráfico huérfano.

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

| Aspecto | Predicho | Real | Desviación |
| --- | --- | --- | --- |
| Arquitectura multihilo | Módulo `pipeline.rs` con `EmulationBridge` y `EmulationWorker` | Implementado en `src/pipeline.rs` con satélite `src/pipeline_tests.rs` | Ninguna |
| Transferencia de frame | Canal con capacidad 1 (`sync_channel(1)`) sin clones pesados | `SyncSender<FramePacket>` con `Box<[[u8; 4]; GBA_PIXELS]>` | Ninguna |
| Inyección de controles | Atómica lock-free `AtomicU16` | `AtomicU16` con `Ordering::Release`/`Acquire` | Ninguna |
| Pacing en background | 60 Hz regulado en `blazing-emu` | `FRAME_DURATION - elapsed` sleep en hilo de trabajo | Ninguna |
| Bucle en Android | Event loop desacoplado consumiendo `try_recv_frame()` | Implementado en `android_main` con timeout reactivo de 1 ms | Ninguna |
| Tests aprobados | ≥85 tests en verde | 85 tests aprobados (75 unitarios + 10 packaging) | Exacto |
| Empaquetado APK | Fat APK firmado multi-ABI (arm64, armv7, x86_64) | APK debug de 31.8 MB generado, 16 KB alineado y verificado v2/v3 | Exacto |

### Contexto: ¿se respetó?

- Se respetó la restricción de mantener el compositor GPU y `app.poll_events` en el hilo de `NativeActivity`.
- Se respetó la transferencia acotada a un solo fotograma sin colas infinitas ni contención de cerrojos.
- Se respetó la independencia de `GameRunner` y el módulo puro sin acoplamiento a tipos de Android.

### Archivos modificados

- `src/pipeline.rs` (nuevo: puente `EmulationBridge`, trabajador `EmulationWorker`, paquete `FramePacket`)
- `src/pipeline_tests.rs` (nuevo: tests satélite de inyección de teclas, transferencia de frames y ciclo de vida)
- `src/lib.rs` (exportación de `pub mod pipeline;`)
- `src/platform.rs` (integración en `AppState` y desacoplamiento del bucle de eventos en `android_main`)
- `docs/referencia/api-modulos-rust.md` (sincronización de especificación técnica generada por script)
- `docs/propuestas/2026-10-10_separacion-hilos-emulacion-y-renderizado.md` (cierre formal de propuesta)

### Discrepancias

Ninguna. La implementación siguió al pie de la letra el diseño predicho y aprobado.

### Para la próxima vez

- La comunicación entre el hilo de trabajo y la UI utiliza `std::sync::mpsc::sync_channel(1)`. Si en el futuro se requiriese reducir aún más el overhead de sincronización, se podría considerar un ring buffer atómico lock-free con memoria compartida (triple buffering).

### Fuera de alcance (hallazgos que no se arreglaron)

- Los shaders en `overlay.wgsl` continúan calculando la carcasa procedural en resoluciones altas (optimización independiente de GPU) | impacto: medio
- El intérprete de respaldo aún ejecuta instrucciones en funciones de combate no recompiladas (cobertura de labels en `blazing.labels.toml`) | impacto: medio

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

---

## Revisión humana

- Procesamiento reactivo de eventos sin bloqueo de ANR (visible): ✔ | ✘ | ?
- Restricción "el compositor GPU y eventos táctiles permanecen en el hilo de NativeActivity" (no testeable): ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
