# AGENTS.md — Blazing Recomp

Directrices de arquitectura, plataforma y reglas operativas para el proyecto **Blazing Recomp**.

---

## 1. Visión y Arquitectura del Proyecto

- **Objetivo:** Port nativo de **Fire Emblem: The Blazing Blade (GBA)** para **Android** mediante recompilación estática AOT (*Ahead-Of-Time*) sin emulación tradicional de CPU.
- **Topología de Bibliotecas en APK (`lib/<abi>/`):**
  - `libblazing_recomp.so`: Runtime Rust generado por `cargo-ndk`. Punto de entrada de `NativeActivity` (`android:hasCode="false"`, `lib_name="blazing_recomp"`). Gestiona GPU, audio, memoria, DMA, timers y controles.
  - `libblazing_recompiled.so`: Lógica C11 AOT del juego compilada por Clang del NDK (144.875 bloques en `src-c/`). Se carga dinámicamente vía `dlopen`.
- **Renderizado GPU:** **`NativePpu`** en GPU con shaders WGSL (`src/ppu_nativa.rs` y `src/present.rs`), con respaldo automático de Vulkan a OpenGL ES 3.0 vía `wgpu`. Conforme al ADR 42, `PpuShim` está extinto; el rasterizador por software en CPU se reserva solo como contingencia (`ppu-nativa-off.flag`).
- **Controles y Pacing:** Mando físico (Gamepad API), touch overlay en pantalla y 60 FPS estables coordinados con `Choreographer` / VSync.
- **Upstream y dependencias:** Motor base en [`JRickey/gba-recomp`](https://github.com/JRickey/gba-recomp). `Cargo.toml` mantiene activa la directiva `[patch]` apuntando a `../gba-recomp/crates/gba-core`. `pack.toml` es consumido exclusivamente por `recomp build`, no por el runtime en Android.

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

Para evitar contaminación de contexto en el agente principal y reducir consumo de cuota, el arnés **bloquea por software** (`PreToolGuard`) la ejecución de comandos verbosos o pesados en el hilo principal. Es **obligatorio** delegar mediante el mecanismo del arnés activo, configurando el rol correspondiente: `invoke_subagent` con modelo `flash_lite` (o `flash`) en Antigravity, o la herramienta `subagent` de DeepSeek Harness (ver §4.1).

| Evento / Tarea | TypeName | Role | Modelo | Disparador Obligatorio (Enforzado por Hook) |
| :--- | :--- | :--- | :--- | :--- |
| **Pruebas y Regresiones** | `self` | `Test Runner` | `google/gemini-3.1-flash-lite` | Ejecutar `cargo test`, arnés de fotogramas dorados o auditoría de mutantes. |
| **Compilación y Empaquetado APK** | `self` | `APK Builder` | `google/gemini-3.5-flash-lite` | Compilar AOT C11 (`compilar_recomp_aot.ps1`), `cargo ndk`, `cargo build` o generar Fat APK (`package_apk.ps1`). |
| **Telemetría y Rendimiento** | `self` | `Telemetry Analyzer` | `google/gemini-3.1-flash-lite` | Analizar `blazing-perf.txt`, frametimes o cuellos de botella de GPU/PPU (`analizar_telemetria.py`). |
| **Etiquetas AOT** | `self` | `Label Auditor` | `google/gemini-3.5-flash-lite` | Mapear o auditar `blazing.labels.toml`, desensamblar ROM o sembrar labels (`auditar_labels.py`). |

> [!NOTE]
> **Poda y automatización:**
> - **Gestión Documental:** El rol `Doc Manager` ha sido podado. Los hooks automáticos (`post_tool_runner`) se encargan de regenerar índices, validar Diátaxis y actualizar la preview sin intervención.
> - **Exploración de Código:** El agente principal utiliza directamente la herramienta MCP `codegraph_explore` para consultas puntuales. El rol `Code Scout` (`research`) se reserva exclusivamente como contingencia para búsquedas masivas no indexadas que abarquen más de 10 archivos completos.

El subagente ejecuta el trabajo de forma aislada y reporta al hilo principal una síntesis ejecutiva de 3 a 5 líneas.

Los dos modelos lite tienen 500 peticiones diarias cada uno: el reparto de la tabla permite usar hasta 1000. DSH no cambia de modelo solo, así que si uno agota su cuota se edita el encargo del rol y se pasa el otro id.

### 4.1 Invocación en DeepSeek Harness (DSH)

En DSH no existe `invoke_subagent`: la herramienta se llama `subagent` y acepta `description`, `prompt`, `provider` y `model`. El rol viaja dentro del `prompt`; el modelo se pasa por llamada, con los dos ids permitidos. Cada rol de la tabla tiene su encargo reutilizable en [`.agents/subagents/`](.agents/subagents/):

| Rol | Encargo | Invocación |
| :--- | :--- | :--- |
| `Test Runner` | [test-runner.md](.agents/subagents/test-runner.md) | `subagent(description='Test Runner', provider='google', model='gemini-3.1-flash-lite', prompt=<contenido del encargo> + tarea)` |
| `APK Builder` | [apk-builder.md](.agents/subagents/apk-builder.md) | `subagent(description='APK Builder', provider='google', model='gemini-3.5-flash-lite', prompt=<contenido del encargo> + tarea)` |
| `Telemetry Analyzer` | [telemetry-analyzer.md](.agents/subagents/telemetry-analyzer.md) | `subagent(description='Telemetry Analyzer', provider='google', model='gemini-3.1-flash-lite', prompt=<contenido del encargo> + tarea)` |
| `Label Auditor` | [label-auditor.md](.agents/subagents/label-auditor.md) | `subagent(description='Label Auditor', provider='google', model='gemini-3.5-flash-lite', prompt=<contenido del encargo> + tarea)` |
| `Code Scout` (contingencia) | [code-scout.md](.agents/subagents/code-scout.md) | `subagent(description='Code Scout', provider='google', model='gemini-3.5-flash-lite', prompt=<contenido del encargo> + tarea)` |

- `subagent_fork` no acepta modelo: el hijo hereda el del padre. Solo para tareas que necesiten el contexto de esta conversación.
- `workflow` cuando hay tres o más tareas, o para modelar varias a la vez.
- En Antigravity se mantiene `invoke_subagent(TypeName='self', Role='<rol>', Model='flash_lite', Prompt='...')` sin cambios.

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
