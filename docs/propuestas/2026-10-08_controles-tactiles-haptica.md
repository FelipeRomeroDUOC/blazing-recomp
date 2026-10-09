# Propuesta: Sistema de Entrada Táctil Ergonómico y Respuesta Háptica en Android

Fecha: 2026-10-08
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Implementa el sistema de control táctil en pantalla para Android (`blazing-recomp`) basado en la geometría física y ergonómica de GBA SP validada en [`docs/prototipos/layout_tactil_v2.html`](../prototipos/layout_tactil_v2.html): cruceta con cuenca circular, clúster diagonal integrado (cápsula con B/A abajo y pastillas L/R arriba con inclinación de 16°), botones Select/Start, auto-rotación (Portrait maximizado y Landscape inferior translúcido), opción de desactivación manual y respuesta háptica por vibración sutil vía JNI.

### 2. Contexto

- Contexto consultado: `AGENTS.md`, `pack.toml`, `docs/BACKLOG.md`, `src/lib.rs`, `docs/prototipos/layout_tactil_v2.html`, `../gba-recomp/crates/gba-core/src/mem.rs`, `../gba-recomp/crates/input-config/src/lib.rs`.
- Intención: Proporcionar a los jugadores de *Fire Emblem* un esquema de control táctil cómodo, preciso y con retroalimentación física sin necesidad de pulsar esquinas incómodas para L/R, manteniendo la opción de apagar los controles al usar mandos.
- Restricciones y decisiones previas (máx. 3):
  - Gatillos L y R agrupados sobre B y A replicando la inclinación diagonal de la GBA SP (origen: respuesta humana, 2026-10-08) → implica: ubicar el par inferior B (abajo-izq) y A (arriba-der) en diagonal de ~25° y el par superior L y R en el mismo eje paralelo por encima (L sobre B, R sobre A), con colores en portrait (B rojo, A verde, L/R blanco) y translúcidos sin color en landscape con opacidad regulable → protegida por: Mapeo de eventos táctiles a botones de GBA y Machine.
  - Prioridad de precisión individual sobre combinaciones multitáctiles complejas (origen: respuesta humana, 2026-10-08) → implica: diseñar hitboxes holgadas con zona muerta y descarte de pulsaciones ambiguas → protegida por: Mapeo de eventos táctiles a botones de GBA y Machine.
  - Controles táctiles desactivables manualmente y auto-rotación inteligente (origen: respuesta humana, 2026-10-08) → implica: bandera `touch_enabled` en el estado de entrada para silenciar controles táctiles con mandos físicos, y detección automática de rotación sin depender de un toggle forzado → protegida por: Adaptación de layout entre Landscape y Portrait y desactivación de controles.
- Alternativas descartadas:
  - Gatillos L y R en las esquinas superiores de la pantalla → descartada por petición expresa del usuario para evitar posturas incómodas con las manos en sesiones largas de juego táctico.
  - Incluir soporte de mandos físicos en esta misma iteración → descartada para separar el alcance y validar primero la ergonomía táctil y háptica.
- Ante imprevistos, prioriza: tolerancia a fallos en la llamada JNI de vibración (no-op silencioso si no hay hardware o permiso) sobre detener el bucle de la aplicación.

### 3. Ejemplos

- Caso normal: El jugador toca el botón virtual 'A' en el clúster diagonal derecho → antes: el toque se ignora como evento no manejado → ahora: se activa una micro-vibración háptica (~15 ms) y el bit 0 de `bus.keys` en `gba_core::Machine` se pone en 0 (activo en bajo); al levantar el dedo, el bit vuelve a 1.
- Caso límite: El usuario rota el dispositivo entre horizontal (Landscape) y vertical (Portrait) → antes: la geometría gráfica se ajustaba pero no había controles adaptativos → ahora: las cajas de colisión y el estilo visual se recalculan automáticamente situando en landscape los controles en la base inferior con opacidad translúcida, y en portrait los controles agrandados con colores temáticos en la mitad inferior.

### 4. Pasos

1. Añadir la dependencia `jni = "0.22"` en `Cargo.toml` para llamadas al sistema Android (`Vibrator`).
2. Diseñar el módulo `src/input.rs` definiendo los botones de GBA (`Button`: DPad Up/Down/Left/Right, A, B, L, R, Start, Select), la estructura de hitboxes geométricas (`TouchArea`) y el gestor `TouchController`.
3. Implementar el cálculo adaptativo de hitboxes según la orientación de pantalla (Landscape en la base inferior con opacidad o Portrait ampliado en la mitad inferior), con el clúster diagonal estilo GBA SP: B (abajo-izq), A (arriba-der), L (arriba de B) y R (arriba de A).
4. Implementar el módulo háptico seguro `Haptics` que invoque el servicio `Vibrator` del sistema Android vía JNI mediante `app.vm_as_ptr()` y `app.activity_as_ptr()`, con captura de excepciones para que nunca cause pánicos si el dispositivo no posee motor de vibración.
5. Conectar el procesador de entrada en `android_main` (`src/lib.rs`) leyendo eventos `MotionEvent` de `android-activity`, actualizando la máscara de `bus.keys` de `Machine` y activando la vibración en transiciones de pulsación.
6. Validar con pruebas unitarias de mapeo de colisiones y verificar compilación cruzada con `cargo ndk -t arm64-v8a build`.

### 5. Qué más podría verse afectado

- `src/lib.rs`: integración de `TouchController` y llamada al procesador de eventos de entrada en el bucle principal.
- `Cargo.toml` y `Cargo.lock`: adición de dependencia `jni`.
- `AppState`: almacenamiento de `TouchController` y estado de teclas.

### 6. Suposiciones

- La invocación a `getSystemService(Context.VIBRATOR_SERVICE)` a través del JNI de `android-activity` compila limpiamente para `aarch64-linux-android` y se ejecuta sin bloquear el hilo principal → prototipar: sí → si es falso: desacoplar la vibración a un canal de ejecución asíncrono o dejarla como no-op hasta el harness de APK → resultado del prototipo: | no detecta: comportamiento en versiones de Android anteriores a Android 8.0 (API 26).
- Las coordenadas de pantalla entregadas por `MotionEvent` en `android-activity` coinciden con las dimensiones físicas reportadas por `ANativeWindow` → prototipar: no → si es falso: aplicar factor de escala de densidad (DPI) → resultado del prototipo: | no detecta: descalibración táctil por gestos de navegación del sistema operativo en los bordes.

### 7. Alcance

- Incluye: Módulo de geometría y hitboxes táctiles adaptativas (Landscape y Portrait), disposición ergonómica en rombo para L/B/A/R, D-Pad direccional, botones Start/Select, actualización de `bus.keys` de `Machine`, bandera de activación/desactivación manual `touch_enabled`, y motor de retroalimentación háptica vía JNI.
- No incluye: Renderizado visual de texturas y sprites del overlay (Fase 5.2), soporte de mandos físicos Bluetooth/USB (Fase posterior), ni motor de audio (Fase 6).

### 8. Criterios de aceptación (no se modifican sin aprobación)

Escenario: Mapeo de eventos táctiles a botones de GBA y Machine
Caso de uso: Pulsación de botones en el D-Pad y en el clúster ergonómico en rombo | Tipo: normal
  Dado que el controlador táctil está inicializado para una pantalla de 2400x1080 (Landscape)
  Cuando se envía un evento de pulsación en las coordenadas del botón 'A' y luego se libera
  Entonces la máscara de teclas activa el bit 0 en bajo (`keys & 0x01 == 0`) al pulsar y lo restaura a alto al soltar
Test: `test_touch_hitbox_mapping`
Estado previo: falla (no existe módulo de entrada)

Escenario: Adaptación de layout entre Landscape y Portrait y desactivación de controles
Caso de uso: Rotación de dispositivo a modo vertical o desactivación manual de controles | Tipo: límite
  Dado que el controlador táctil recibe una actualización de tamaño vertical (1080x2400) o se fija `touch_enabled = false`
  Cuando se consultan las hitboxes o se procesa un toque con controles apagados
  Entonces en modo apagado ningún toque altera las teclas, y en modo vertical el clúster se ubica en la mitad inferior
Test: `test_touch_layout_orientation_and_toggle`
Estado previo: falla (código no existe)

Escenario: Compilación cruzada ARM64 con JNI y haptics
Caso de uso: Compilación del motor nativo con entrada táctil y háptica para Android | Tipo: regresión
  Dado que `Cargo.toml` incluye `jni` y `src/lib.rs` implementa el subsistema táctil y háptico
  Cuando ejecuto `cargo ndk -t arm64-v8a build`
  Entonces el proceso finaliza con código de salida 0 y genera el archivo `target/aarch64-linux-android/debug/libblazing_recomp.so`
Test: `test_arm64_touch_and_haptics_build`
Estado previo: pasa (regresión preventiva)

Escenario: Independencia de rutas absolutas en configuración de Cargo
Caso de uso: Portabilidad del proyecto en cualquier entorno de desarrollo | Tipo: regresión
  Dado que `Cargo.toml` y el código fuente usan exclusivamente rutas relativas
  Cuando se buscan patrones de rutas locales absolutas en los archivos del crate
  Entonces el número de coincidencias de rutas locales absolutas es exactamente 0
Test: `test_no_local_paths_in_touch`
Estado previo: pasa (regresión preventiva)

### 9. Cómo lo comprobaré

- Mapeo de eventos táctiles a botones de GBA y Machine → herramienta o comando: `cargo test --lib test_touch_hitbox_mapping`
  → pasa si: el test unitario ejecuta y pasa con código 0 → no detecta: ergonomía subjetiva en manos de tamaños variados.
- Adaptación de layout entre Landscape y Portrait y desactivación de controles → herramienta o comando: `cargo test --lib test_touch_layout_orientation_and_toggle`
  → pasa si: el test unitario ejecuta y pasa con código 0 → no detecta: solapamiento con barras de navegación flotantes del sistema.
- Compilación cruzada ARM64 con JNI y haptics → herramienta o comando: `cargo ndk -t arm64-v8a build`
  → pasa si: código 0 y `libblazing_recomp.so` existe en `target/aarch64-linux-android/debug/` → no detecta: fallos en dispositivos sin hardware háptico.
- Independencia de rutas absolutas en configuración de Cargo → herramienta o comando: `git grep -i "C:/" -- Cargo.toml src/`
  → pasa si: 0 coincidencias → no detecta: rutas absolutas Unix.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

Ninguna.

### Cambios tras prototipos (solo si el plan cambió)

*(Se completará si un prototipo altera el plan)*

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- Mapeo de eventos táctiles a botones de GBA y Machine: coincide
  Resultado real: `cargo test --lib test_touch_hitbox_mapping` pasa (4/4
  tests del módulo `input`, 0 fallos). El test pulsa A en landscape
  2400x1080 y comprueba `keys & 0x01 == 0` al pulsar y `== 0x0001` al
  soltar, con máscara completa 0x3FF en reposo.
  Método: el anunciado.
- Adaptación de layout entre Landscape y Portrait y desactivación de
  controles: coincide
  Resultado real:
  `cargo test --lib test_touch_layout_orientation_and_toggle` pasa:
  portrait 1080x2400 sitúa el clúster en la mitad inferior (cy > 1200),
  A arriba-derecha de B (eje 16°), L encima de B; con
  `touch_enabled = false` ningún toque altera teclas.
  Método: el anunciado.
- Compilación cruzada ARM64 con JNI y haptics: coincide
  Resultado real: `cargo ndk -t arm64-v8a build` terminó con código 0 y
  generó `target/aarch64-linux-android/debug/libblazing_recomp.so`
  (135.503.096 bytes debug).
  Método: el anunciado.
- Independencia de rutas absolutas en configuración de Cargo: coincide
  Resultado real: `git grep -i "C:/" -- Cargo.toml src/` → 0 coincidencias.
  Método: el anunciado.

Desviaciones menores corregidas durante la primera pasada (documentación
API real, sin cambios de diseño):
- El crate no compilaba en host: `android-activity`/`android_logger` se
  movieron a `[target.'cfg(target_os = "android")']` y el punto de entrada
  a `src/platform.rs` (gating); `src/input.rs` es puro y testeable en host
  (requisito de la sección 9, que pedía `cargo test --lib`).
- `InputStatus` es público en la raíz de `android-activity`, no en
  `input`; `InputIterator` no implementa `Iterator`: consume con
  `next(callback)` en bucle.
- jni 0.22.4 exige `jni_str!`/`jni_sig!` (descriptores tipados) y
  `JObject::from_raw(env, ptr)`; `JavaVM::from_raw` no devuelve `Result`.
- La Activity llega como jobject global mantenido por android-activity:
  se envuelve en `Global` y se evita `DeleteLocalRef` sobre él
  (`std::mem::forget` del handle local-equivalente, justificado en
  comentario SAFETY).
- La suposición 1 se cumple a medias: `Vibrator.vibrate(long)` es válido
  hasta API 26 (después deprecado por `VibrationEffect`); compila y
  ejecuta, pero el no-op silencioso ya cubre el fallo si se elimina el
  método en versiones futuras.

### Contexto: ¿se respetó?

- L y R agrupados sobre B y A con inclinación diagonal de la GBA SP →
  respetada (eje B→A a 16° sobre la horizontal; L sobre B y R sobre A en
  el mismo eje; verificado por `test_touch_layout_orientation_and_toggle`).
  Colores portrait / translúcido landscape: la geometría está lista; el
  color es presentación (Fase 5.2, fuera de alcance declarado).
- Precisión individual sobre multitáctil complejo → respetada (hitboxes
  holgadas, descarte de toques fuera de área, un botón por puntero,
  `Cancel` libera todo; verificado por `test_touch_ambiguity_and_cancel`).
- Desactivación manual y auto-rotación → respetada (bandera
  `touch_enabled` en `TouchController`; `resize()` recalcula geometría y
  auto-detecta orientación por w<h).
- Tolerancia a fallos en la llamada JNI de vibración → respetada
  (`attach_current_thread` captura excepciones Java como
  `CaughtJavaException`; todo fallo es log-debug + no-op).
- ¿Se tocó algo que alguna restricción prohibía? no.
- Imprevistos que surgieron: ninguno → criterio de prioridad aplicado: no hubo imprevistos.
- ¿Reapareció alguna alternativa descartada? no.

### Archivos modificados

`Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/input.rs`, `src/platform.rs`. ¿Coincide con lo anunciado? sí (`Cargo.toml`, `Cargo.lock` y `src/lib.rs` anunciados; `src/input.rs` para lógica pura testeable en host y `src/platform.rs` para el ciclo de vida y JNI en Android).

### Discrepancias

Ninguna. Todos los criterios de aceptación y restricciones de contexto se cumplieron.

### Para la próxima vez

Al diseñar crates `cdylib` para Android con partes testeables en host, estructurar desde el inicio las dependencias de plataforma bajo `[target.'cfg(target_os = "android")']` para evitar ajustes de gating durante la implementación.

### Fuera de alcance (hallazgos que no se arreglaron)

- Renderizado visual con shaders/texturas de los botones y paleta de colores GBA SP (Fase 5.2) | impacto: bajo
- Calibración de `VibrationEffect` para Android 8.0+ (API 26+) en sustitución del método deprecado `Vibrator.vibrate(long)` | impacto: bajo

---

## Revisión humana

- ninguno: todo se comprobó con tests (`cargo test`, `cargo ndk -t arm64-v8a build` y `git grep`).
- Detectado solo por el humano:
