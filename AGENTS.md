# AGENTS.md — Blazing Recomp

Directrices de arquitectura, plataforma y reglas operativas para el proyecto **Blazing Recomp**.

---

## 1. Visión y Arquitectura del Proyecto

- **Misión y Visión:** Plataforma nativa de recompilación para Android de **Fire Emblem: The Blazing Blade** (con proyección arquitectónica hacia la trilogía GBA: *Binding Blade*, *Blazing Blade* y *The Sacred Stones*). No es un simple reproductor/wrapper de emulación ni un port superficial: busca maximizar el porcentaje de funciones nativas in-house (hacia el 100% nativo), manipular directamente la composición en GPU/hardware moderno y proveer una plataforma extensible mediante mods y plugins para la comunidad (filosofía *Recomp* moderna).
- **Independencia y Créditos Upstream:** Basado inicialmente en la tecnología fundacional de [`JRickey/gba-recomp`](https://github.com/JRickey/gba-recomp) y `gba-core`, acreditando siempre sus aportes pero con una trayectoria de independencia técnica progresiva hacia subsistemas nativos propios en Rust y C. `Cargo.toml` mantiene activa la directiva `[patch]` apuntando a `../gba-recomp/crates/gba-core`. `pack.toml` es consumido exclusivamente por `recomp build`, no por el runtime en Android.
- **Topología de Bibliotecas en APK (`lib/<abi>/`):**
  - `libblazing_recomp.so`: Runtime Rust generado por `cargo-ndk`. Punto de entrada de `NativeActivity` (`android:hasCode="false"`, `lib_name="blazing_recomp"`). Gestiona GPU, audio de baja latencia con AAudio, memoria, DMA, timers, controles y subsistema de extensiones.
  - `libblazing_recompiled.so`: Lógica C11 AOT del juego compilada por Clang del NDK (144.875 bloques en `src-c/`). Se carga dinámicamente vía `dlopen`.
- **Renderizado GPU y Filosofía de Composición Directa:** **`NativePpu`** en GPU con shaders WGSL (`src/ppu_nativa.rs` y `src/present.rs`), con respaldo automático de Vulkan a OpenGL ES 3.0 vía `wgpu`. Se rechaza la emulación scanline a scanline en CPU; se manipula directamente la composición en la GPU de Android, permitiendo features nativas avanzadas (como Widescreen adaptativo 16:9 y 21:9). Conforme al ADR 42, `PpuShim` está extinto; el rasterizador por software en CPU se reserva solo como contingencia (`ppu-nativa-off.flag`).
- **Controles, Audio y Pacing:** Mando físico (Gamepad API con autodetección), touch overlay procedimental en pantalla y 60 FPS estables coordinados con `Choreographer` / VSync. Motor de audio nativo de baja latencia con AAudio NDK y mezclador de punto flotante MP2K HLE en 32 bits (`src/audio.rs`).
- **Alcance del Hito MVP (`v0.1.0`):** Juego 100% jugable de inicio a fin con persistencia de partidas `.sav` en almacenamiento Android (`filesDir`), soporte completo de mandos físicos (Bluetooth/USB) y táctiles, interfaz gráfica nativa con menús y remapeo de botones, y Widescreen adaptativo (16:9 / 21:9) como feature insignia demostrativa.

---

## 2. Datos Oficiales y Seguridad (Enforzados por Hooks)

- **ROM Oficial:** `Fire Emblem (Europe)` (16 MB) — SHA-256: `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS Oficial:** `gba_bios.bin` (16 KB) — SHA-256: `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Hooks del Arnés (`.agents/hooks/`):** Bloquean automáticamente y por software:
  - Creación o versionado de binarios `.gba`, `.bin`, `.elf`.
  - Inserción de rutas absolutas locales del sistema operativo. Toda ruta debe ser relativa al repositorio.
  - Comandos Git destructivos (`git reset`, `git restore`, `git checkout .`).

---

## 3. Reglas Técnicas y Calidad de Código

1. **Enfoque 0 Interpreter:** Priorizar cobertura nativa mapeando labels en `blazing.labels.toml`.
2. **Topología de Tests:**
   - **Satélite:** Si un módulo supera 350 líneas o sus tests superan 150 líneas, mover los tests a `src/<modulo>_tests.rs` enlazado con `#[cfg(test)] #[path = "<modulo>_tests.rs"] mod tests;`.
   - **Integración:** Pruebas de APK, manifiesto y sincronización residen en `tests/`.
3. **Exactitud Matemática:** En distancias $f32$, usar ternas pitagóricas exactas o deltas $\epsilon$ para evitar fallos por redondeo IEEE-754. Prohibido debilitar asserts.
4. **Sincronización y Anti-Contradicciones:** Al modificar subsistemas, alinear de forma unánime `README.md`, `docs/` y `GLOSSARY.md`.
5. **Higiene de Tableros:**
   - `docs/BACKLOG.md`: Todo hallazgo resuelto debe marcarse `[Cerrado: AAAA-MM-DD]`.
   - `docs/PENDIENTES.md`: Exclusivo para tareas de desarrollo reales y accionables en código (`[x]`).
6. **Versionado Canónico y Compuerta de Aprobación Previa:** El versionado de la aplicación (`Cargo.toml`, `android/AndroidManifest.xml`, `pack.toml`, `CHANGELOG.md`, `docs/`) debe gestionarse mediante `python scripts/bump_version.py`. Queda estrictamente prohibido que el agente o un proceso desasistido incremente la versión de la app en disco sin la confirmación y aprobación explícita previa del usuario.

---

## 4. Subagentes Especializados y Matriz de Disparadores

Para evitar contaminación de contexto en el agente principal y optimizar la cuota semanal, el arnés opera bajo una **arquitectura híbrida**: plantillas de encargo fijas en [`.agents/subagents/`](.agents/subagents/) combinadas con **ciclo de vida efímero** (un subproceso limpio por tarea que se desecha al entregar su reporte). El arnés **bloquea por software** (`PreToolGuard`) la ejecución de comandos verbosos o pesados en el hilo principal.

Los roles se dividen estrictamente según su naturaleza: **Razonamiento Crítico (`pro`)** y **Ejecución Mecánica (`flash_lite`)**:

### División 1: Razonamiento Crítico (Modelos Pro)
| Evento / Tarea | TypeName | Role | Modelo Antigravity | Modelo DSH | Disparador Obligatorio |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Diagnóstico y Arquitectura** | `research` | `Investigador` | `pro` | `google/gemini-1.5-pro` | Diagnóstico de causas raíz de bugs complejos, análisis de hardware GBA, PPU/Audio y ROM. |
| **Auditoría de Propuestas** | `research` | `Proposal Reviewer` | `pro` | `google/gemini-1.5-pro` | Auditar borradores de Propuestas N2 (Lenguaje claro ISO 24495-1 y verificabilidad) antes de entregar al usuario. |
| **Implementación de Código** | `self` | `Constructor` | `pro` (o `flash`) | `google/gemini-1.5-pro` | Programación de nuevos módulos, algoritmos de renderizado/audio y refactorizaciones aprobadas. |

### División 2: Ejecución Mecánica (Modelos Flash-Lite)
| Evento / Tarea | TypeName | Role | Modelo Antigravity | Modelo DSH | Disparador Obligatorio (Enforzado por Hook) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Pruebas y Regresiones** | `self` | `Test Runner` | `flash_lite` | `google/gemini-3.1-flash-lite` | Ejecutar `cargo test`, arnés de fotogramas dorados o auditoría de mutantes. |
| **Compilación y Empaquetado APK** | `self` | `APK Builder` | `flash_lite` | `google/gemini-3.5-flash-lite` | Compilar AOT C11 (`compilar_recomp_aot.ps1`), `cargo ndk`, `cargo build` o Fat APK (`package_apk.ps1`). |
| **Búsqueda Masiva (Contingencia)**| `research` | `Code Scout` | `flash_lite` | `google/gemini-3.5-flash-lite` | Búsquedas no indexadas de más de 10 archivos completos cuando CodeGraph no baste. |

> [!NOTE]
> **Ciclo de vida y absorción de roles:**
> - **Efímeros por defecto:** Cada subagente se invoca para una tarea puntual, entrega su reporte ejecutivo (3 a 5 líneas) y se destruye. Esto previene la acumulación de logs muertos y protege la cuota semanal.
> - **Excepción de ciclo corto:** El `Constructor` puede recibir 2 o 3 mensajes iterativos de corrección (`send_message`) únicamente dentro del bucle de depuración de una misma feature activa.
> - **Roles absorbidos:** `Telemetry Analyzer` y `Label Auditor` quedan absorbidos como misiones dinámicas del `Investigador`. `Doc Manager` está podado (automatizado por `post_tool_runner`).

### 4.1 Invocación en DeepSeek Harness (DSH)

En DSH la herramienta `subagent` acepta `description`, `prompt`, `provider` y `model`. Cada rol consume su encargo desde [`.agents/subagents/`](.agents/subagents/):

| Rol | Encargo | Invocación en DSH |
| :--- | :--- | :--- |
| `Investigador` | [investigador.md](.agents/subagents/investigador.md) | `subagent(description='Investigador', provider='google', model='gemini-1.5-pro', prompt=<contenido> + tarea)` |
| `Proposal Reviewer`| [proposal-reviewer.md](.agents/subagents/proposal-reviewer.md) | `subagent(description='Proposal Reviewer', provider='google', model='gemini-1.5-pro', prompt=<contenido> + tarea)` |
| `Constructor` | [constructor.md](.agents/subagents/constructor.md) | `subagent(description='Constructor', provider='google', model='gemini-1.5-pro', prompt=<contenido> + tarea)` |
| `Test Runner` | [test-runner.md](.agents/subagents/test-runner.md) | `subagent(description='Test Runner', provider='google', model='gemini-3.1-flash-lite', prompt=<contenido> + tarea)` |
| `APK Builder` | [apk-builder.md](.agents/subagents/apk-builder.md) | `subagent(description='APK Builder', provider='google', model='gemini-3.5-flash-lite', prompt=<contenido> + tarea)` |
| `Code Scout` | [code-scout.md](.agents/subagents/code-scout.md) | `subagent(description='Code Scout', provider='google', model='gemini-3.5-flash-lite', prompt=<contenido> + tarea)` |

- `subagent_fork` no acepta modelo: el hijo hereda el del padre. Solo para tareas que necesiten el contexto de esta conversación.
- En Antigravity se invoca mediante `invoke_subagent(TypeName='research'|'self', Role='<rol>', Model='pro'|'flash_lite', Prompt='...')`.

---

## 5. Skills Locales y Pipeline Operativo

Las skills completas residen en [`.agents/skills/`](.agents/skills/):
- **`propuesta-verificable`:** Flujo obligatorio antes de tocar código: Nivel 0 (trivial/texto), Nivel 1 (1 archivo en chat con Antes/Ahora), Nivel 2 (crítico en `docs/propuestas/`). Prohibido editar sin aprobación previa. En Nivel 2, incrustar obligatoriamente la previsualización interactiva con `<agent-embed>`.
- **`diataxis-docs`:** Estructura de documentación (Tutoriales, Guías, Referencia, [Explicación](docs/explicacion/README.md) / [DECISIONES.md](docs/explicacion/DECISIONES.md)).
- **`test-generation`:** Diseño de pruebas de frontera y robustez. Suite en verde antes de modificar.
- **`lenguaje-claro-iso-24495`:** Directriz transversal obligatoria: ir directo al grano, sin rodeos ni fórmulas de relleno.

> [!NOTE]
> **Automatización del Arnés:** Al guardar archivos en `docs/` o `src/`, el hook `post_tool_runner` ejecuta automáticamente `actualizar_indice_propuestas.py`, `validar_docs.py` y `cargo fmt`.

---

## 6. Comandos Canónicos de Referencia

| Propósito | Comando Oficial |
| :--- | :--- |
| **Verificar Sincronía Versiones** | `python scripts/bump_version.py --check` |
| **Simular Incremento Versión** | `python scripts/bump_version.py --dry-run --patch` |
| **Empaquetado Fat APK** | `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` |
| **Análisis de Telemetría** | `python scripts/diagnostico/analizar_telemetria.py --archivo <log.txt> --top <N>` |
| **Desensamblado ROM** | `python scripts/diagnostico/desensamblar_rom.py --buscar-prologo <addr>` |
| **Auditoría Labels AOT** | `python scripts/diagnostico/auditar_labels.py` |
| **Compilar AOT C11** | `powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1` |
| **Validar Documentación** | `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` |
