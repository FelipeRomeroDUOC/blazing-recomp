# Propuesta: Pacing de Fotogramas a 60 FPS y Conexión del Pipeline de Vídeo GBA

Fecha: 2026-10-10
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Diseño e integración del subsistema de ejecución y presentación de fotogramas del juego en tiempo real:

1. **Nuevo Módulo Puro de Ejecución (`src/runner.rs`):**
   - Encapsula el modelo de hardware de la consola (`gba_core::Machine`), el simulador de panel LCD (`screen::Temporal`) y la tabla de color (`screen::ColorLut`) en una abstracción desacoplada de la plataforma: `GameRunner`.
   - Expone la API pura de avance de fotogramas:
     - `new(rom: Vec<u8>, bios: Option<&[u8]>, screen_kind: ScreenKind) -> Self`: inicialización con BIOS oficial o HLE.
     - `step_frame(&mut self) -> bool`: avanza el presupuesto de ciclos de CPU de un fotograma GBA (280,896 ciclos), extrae el framebuffer PPU ($240 \times 160$ RGB555), lo alimenta al filtro temporal con la LUT y compone el búfer RGBA final de 38,400 píxeles.
     - `set_keys(&mut self, keys: u16)`: inyecta la máscara `KEYINPUT` (activo en bajo) derivada del controlador táctil o gamepad físico.
     - `frame_rgba(&self) -> &[[u8; 4]]`: acceso de solo lectura al fotograma listo para que el `Compositor` GPU lo suba a la textura de juego.
   - Totalmente compilable y testeable en host (`cargo test`) sin requerir emuladores ni dispositivos Android.

2. **Topología Satélite de Pruebas (`src/runner_tests.rs`):**
   - Cumple con la regla de `AGENTS.md` para módulos puros: los tests unitarios residen en `src/runner_tests.rs` vinculado mediante `#[cfg(test)] #[path = "runner_tests.rs"] mod tests;`.
   - Evalúa contratos: longitud exacta del framebuffer, avance de reloj y ciclo de fotogramas, respuesta a máscaras de teclas y reinicio limpio.

3. **Game Loop y Pacing a 60 FPS en Android (`src/platform.rs`):**
   - Transforma el bucle reactivo de `android_main` en un game loop activo a 60 Hz cuando la app está activa (`Resume`):
     - Pacing de tiempo con `std::time::Instant` y límite a ~16.66 ms por fotograma coordinado con `wgpu::PresentMode::AutoVsync`.
     - Procesamiento consolidado de eventos de entrada por frame antes del paso de CPU.
     - Suspensión total en segundo plano (`Pause`): la app entra en espera bloqueante (`app.poll_events(None, ...)`), garantizando **0% de consumo de batería** cuando el usuario sale de la aplicación.
   - Enlace entre `GameRunner` y el `Compositor` GPU en `redraw()`: en lugar de subir un búfer negro constante, sube el fotograma vivo generado por la PPU.

---

### 2. Contexto

- **Contexto consultado:**
  - [`src/platform.rs`](../../src/platform.rs): Implementación actual del `Compositor` y el event loop.
  - [`src/lib.rs`](../../src/lib.rs): Declaración de módulos públicos y punto de entrada.
  - `gba-core::Machine` y `screen::Temporal` (dependencias upstream de `JRickey/gba-recomp`).
  - [`docs/explicacion/arquitectura-port-android.md`](../explicacion/arquitectura-port-android.md): Diagrama arquitectónico en 3 capas.
  - [`AGENTS.md`](../../AGENTS.md): Reglas de topología satélite, cero rutas absolutas y pruebas de frontera.
- **Intención:** Eliminar la pantalla negra estática en el marco de la consola y activar la presentación gráfica continua a 60 FPS manteniendo el consumo energético controlado.
- **Restricciones y decisiones previas:**
  - Preservar la independencia de la lógica pura en host (no poner lógica de `Machine` acoplada directamente a tipos de Android NDK).
  - Mantener los 49 tests existentes pasando al 100% en verde.
  - Cero inclusión de archivos `.gba` o `.bin` propietarios en el repositorio (`AGENTS.md §3`).

---

### 3. Ejemplos (Dado / Cuando / Entonces)

- **Caso 1: Avance de fotograma determinista (`GameRunner`):**
  - **Dado:** Una instancia de `GameRunner` inicializada con una ROM válida de prueba.
  - **Cuando:** Se invoca `runner.step_frame()`.
  - **Entonces:** La función retorna `true`, el contador `frames_elapsed()` incrementa en 1 y `runner.frame_rgba()` contiene exactamente 38,400 píxeles ($240 \times 160$) con canal alfa en 255.

- **Caso 2: Inyección de entrada (`set_keys`):**
  - **Dado:** Un `GameRunner` en ejecución.
  - **Cuando:** El usuario presiona el botón A (`keys = !0x0001 & 0x03FF = 0x03FE`).
  - **Entonces:** En el siguiente `step_frame()`, el bus de hardware refleja `machine.bus.keys == 0x03FE`.

- **Caso 3: Suspensión de batería en Android (`Pause`):**
  - **Dado:** La aplicación corriendo a 60 FPS en primer plano.
  - **Cuando:** El usuario minimiza la aplicación o recibe una llamada (evento `MainEvent::Pause`).
  - **Entonces:** El hilo nativo detiene el bucle de 60 FPS y entra en espera pasiva de eventos (`poll_events(None)`), liberando el hilo de CPU y la GPU.

---

### 4. Criterios de Aceptación

- [x] Crear [`src/runner.rs`](../../src/runner.rs) con la abstracción pura `GameRunner`.
- [x] Crear [`src/runner_tests.rs`](../../src/runner_tests.rs) con suite unitaria en host (tamaño de buffer, pacing, teclas, reinicio).
- [x] Exportar `pub mod runner;` en [`src/lib.rs`](../../src/lib.rs).
- [x] Integrar el bucle de fotogramas a 60 FPS en [`src/platform.rs`](../../src/platform.rs) coordinado con el `Compositor`.
- [x] Mantener la suspensión pasiva de batería al recibir `Pause`.
- [x] Todos los tests de la suite pasan en verde con `cargo test` (esperado: 49 tests existentes + nuevos tests de `runner`).
- [x] Validar Diátaxis (`validar_docs.py`) y sincronización de referencias técnicas sin errores.
- [x] Compilar y verificar el APK con `scripts/package_apk.ps1`.

---

### 5. Fuera de alcance (lo que NO incluye esta tarea)

- Carga interactiva mediante selector SAF de Android (Storage Access Framework) o escaneo automático en disco (corresponde a la Etapa 3 del backlog).
- Salida de audio por AAudio/Oboe (Fase 6 del backlog).
- Enlace del monolito completo de C recompilado estático (se usa el motor `gba-core` con intérprete/HLE para el pipeline de vídeo en esta etapa).

---

### 6. Comando de Verificación

```bash
cargo test
python .agents/skills/diataxis-docs/scripts/validar_docs.py .
python scripts/extraer_referencias.py --check
powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1 -SkipBuild:$false
```

---

## Verificación (Predicho vs. Real)

### 1. Qué se predijo vs. Qué se implementó

- **Predicho:**
  - Crear módulo puro `src/runner.rs` con struct `GameRunner`.
  - Crear tests satélite `src/runner_tests.rs` vinculados con `#[path]`.
  - Conectar bucle de 60 Hz con suspensión pasiva en `src/platform.rs`.
- **Real:**
  - Se implementó [`src/runner.rs`](../../src/runner.rs) encapsulando `gba_core::Machine`, `screen::Temporal` y `screen::ColorLut`.
  - Se implementaron 5 tests satélite en [`src/runner_tests.rs`](../../src/runner_tests.rs) validando:
    1. Geometría y canal alfa RGBA (`test_game_runner_framebuffer_geometry_and_alpha`).
    2. Inyección de teclas `KEYINPUT` activa en bajo (`test_game_runner_key_injection`).
    3. Avance de contador de ciclos de reloj (`test_game_runner_step_frame_clock_advance`).
    4. Carga con BIOS provisto (`test_game_runner_creation_with_bios`).
    5. Cambio de perfil de pantalla LCD (`test_game_runner_change_screen_kind`).
  - Se exportó `pub mod runner;` en [`src/lib.rs`](../../src/lib.rs).
  - Se integró el bucle a 60 FPS en [`src/platform.rs`](../../src/platform.rs) dentro de `step_and_redraw()`, gestionando el tiempo transcurrido con `std::time::Instant`, `sleep` cooperativo para alcanzar el frame time de 16.66 ms, y `poll_events(None)` durante `Pause` (0% batería en background).

### 2. Suite de Pruebas

- **Predicho:** 49 tests existentes + nuevos tests de `runner` (54 tests esperados).
- **Real:** 54 tests ejecutados y aprobados al 100% en verde (`46` unitarios en `src/lib.rs` + `8` de integración en `tests/packaging_tests.rs`).

### 3. Empaquetado APK

- **Predicho:** Compilación limpia multi-ABI y Fat APK funcional.
- **Real:** `scripts/package_apk.ps1` finalizó con código 0, generando el Fat APK de 40.19 MB firmado en `target/blazing-recomp-debug.apk` con alineación zipalign a 16 KB y firmas v2/v3 válidas.

### 4. Sincronización Documental y Diátaxis

- `scripts/extraer_referencias.py` actualizó [`docs/referencia/api-modulos-rust.md`](../referencia/api-modulos-rust.md) documentando la API completa de `GameRunner`.
- `validar_docs.py` devolvió 0 errores y 0 avisos sobre los 8 documentos del árbol Diátaxis.

### 5. Fuera de alcance (hallazgos que no se arreglaron)

- Ningún fallo colateral ni regresión detectada. Las siguientes etapas identificadas en el backlog (interfaz de selección SAF para ROM y salida de audio nativa) se mantienen agendadas para sus propuestas dedicadas correspondientes.
