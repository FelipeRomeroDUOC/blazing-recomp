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
  - Uso de `Model='inherit'`, debilitamiento de `assert!`, comparación directa `f32` en `assert_eq!` y módulos mayores a 350 líneas con tests inline.

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

### 3.1 Diagnóstico de Errores y Prevención de Bucles de Parches

1. **Bisección desde el primer minuto:** Antes de proponer cambios en código ante un fallo, identificar todas las capas por donde pasa el flujo afectado (`Origen/Lógica -> Transporte/Sincronía -> Transformación -> Salida`) y realizar una comprobación que divida el problema para aislar en qué frontera nace el error, en lugar de parchar el último eslabón visible.
2. **Cero "saneamientos" heurísticos por caso o escena:** Queda prohibido introducir condiciones que detecten combinaciones específicas de una pantalla, cuadro o momento del juego para forzar estados o registros artificiales. Toda corrección debe cumplir el contrato general del hardware o del subsistema.
3. **Un solo fallo por hipótesis (*1-Strike Rule*):** Si una propuesta resulta **`discrepada`**, está prohibido iterar variantes de la misma idea sobre el mismo punto. Antes de presentar una nueva propuesta es obligatorio aportar evidencia nueva que explique por qué falló la suposición anterior y descarte esa capa.
4. **Reproducción local en PC antes de editar:** Salvo fallos exclusivos de las APIs o ciclo de vida de Android, todo error del motor o del juego debe reproducirse y verificarse localmente en PC antes de modificar código y empaquetar un APK.
5. **Principio de absolución previa:** Antes de modificar un archivo o shader, formular la prueba mínima que demuestre si ese componente es inocente (ej. ¿el rasterizador software de referencia falla con los mismos datos de entrada?). Si la referencia falla idénticamente, el componente queda absuelto y se descarta de inmediato para no perder tiempo en falsas sospechas.
6. **Pruebas de humo deterministas (*Headless* en PC):** Preferir scripts de ejecución sin ventana en PC que avancen $N$ cuadros y verifiquen el estado en segundos, reservando el ciclo lento de empaquetado de APK e instalación manual en Android para la confirmación de hitos o fallos intrínsecos de la plataforma móvil.

---

## 4. Subagentes Especializados y Matriz de Disparadores

Para evitar contaminación de contexto en el agente principal y optimizar la cuota semanal, el arnés reserva los subagentes exclusivamente para tareas que requieren **razonamiento real**, con plantillas fijas en [`.agents/subagents/`](.agents/subagents/) y **ciclo de vida efímero**. Las ejecuciones mecánicas (`cargo test`, empaquetado de APK y validaciones de plantillas) están automatizadas mediante scripts deterministas en `scripts/`.

Los roles de IA se dividen en tres niveles según su naturaleza técnica, prohibiéndose el uso de herencia pasiva (`inherit`):

### División 1: Reserva Crítica de Escalamiento (Gemini 4 Argon / Pro)
| Evento / Tarea | TypeName | Role | Modelo Antigravity | Modelo DSH | Disparador Obligatorio |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Escalamiento Crítico (Último Recurso)** | `self` | `Deep Debugger` | `pro` | `google/gemini-4-argon` | **Solo bajo escalamiento:** Cuando `Investigador` (`3.8 Flash`) no logre aislar la causa raíz tras un primer intento, o ante fallos críticos de sincronización hardware/ROM. |

### División 2: Núcleo Técnico y Analítico (Gemini 3.8 Flash)
| Evento / Tarea | TypeName | Role | Modelo Antigravity | Modelo DSH | Disparador Obligatorio (Enforzado por Hook) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Extracción y Diagnóstico** | `self` | `Investigador` | `flash` | `google/gemini-3.8-flash` | Ojos del Orquestador: extracción de snippets verbatim, CodeGraph y decodificación de registros (máx 8 llamadas). |
| **Auditoría de Propuestas (Opcional)** | `self` | `Proposal Reviewer` | `flash` | `google/gemini-3.8-flash` | Auditoría opcional bajo demanda: solo para propuestas críticas de arquitectura Nivel 2 a solicitud explícita. |
| **Implementación de Código** | `self` | `Constructor` | `flash` | `google/gemini-3.8-flash` | Programación en `src/` o `src-c/`, auto-verificación obligatoria con `cargo check` y unit test puntual (máx 20 llamadas). |

### División 3: Exploración Masiva (Flash-Lite)
| Evento / Tarea | TypeName | Role | Modelo Antigravity | Modelo DSH | Disparador Obligatorio (Enforzado por Hook) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Búsqueda Masiva (Contingencia)**| `self` | `Code Scout` | `flash_lite` | `google/gemini-3.5-flash-lite` | Búsquedas no indexadas de más de 10 archivos completos cuando CodeGraph no baste (máx 8 llamadas). |

> [!IMPORTANT]
> **Protección de Cuota Argon, Prohibición de `inherit` y Scripts Deterministas:**
> - El argumento `Model` en `invoke_subagent` debe especificarse **siempre de forma explícita** (`flash`, `flash_lite` o `pro`). Queda terminantemente prohibido el uso de `inherit`.
> - **Prioridad 3.8 Flash:** Toda investigación diagnóstica, auditoría de propuestas e implementación de código se ejecuta por defecto en **`Gemini 3.8 Flash` (`Model='flash'`)**. **`Gemini 4 Argon` (`Model='pro'`)** se reserva exclusivamente para escalamiento de último recurso (`Deep Debugger`).
> - **Ejecutores Deterministas (Sin Subagente):** Las pruebas (`cargo test`) y la compilación de APK (`package_apk.ps1` / `compilar_recomp_aot.ps1`) no usan subagentes; se ejecutan mediante `python scripts/run_tests_summary.py` y `python scripts/build_apk_summary.py`.

> [!NOTE]
> **Ciclo de vida, Micro-Investigación Paralela (Map-Reduce) y Caché de Hallazgos:**
> - **Efímeros por defecto:** Cada subagente se invoca para una tarea puntual, entrega su reporte ejecutivo (3 a 5 líneas) y se destruye.
> - **Micro-Investigaciones Paralelas (Map-Reduce):** Queda prohibido enviar megaprompts abiertos a un solo `Investigador`. Cuando un problema involucre varios frentes (ej. registros I/O en volcados, empaquetado en Rust y shader WGSL), el Orquestador debe dividirlo en micro-preguntas atómicas y lanzar **2 o 3 instancias de `Investigador` en paralelo en el mismo turno** (ej. `Role='Investigador: Volcados'`, `Role='Investigador: WGSL'`).
> - **Caché Persistente de Hallazgos y Circuit Breakers:** Antes de cerrar, cada `Investigador` persiste su descubrimiento con `ledger.py --save-finding` e incluye `archivo:L1-L2`. Presupuesto por subagente: `Investigador` (máx 8 llamadas), `Constructor` (máx 20 llamadas con auto-verificación `cargo check`), `Code Scout` (máx 8 llamadas).
> - **Roles podados / absorbidos:** `Test Runner` y `APK Builder` están podados (reemplazados por `run_tests_summary.py` y `build_apk_summary.py`). `Telemetry Analyzer` y `Label Auditor` quedan absorbidos como misiones dinámicas del `Investigador` (`3.8 Flash`). `Doc Manager` está podado (automatizado por `post_tool_runner`).

### 4.1 Invocación en DeepSeek Harness (DSH)

En DSH la herramienta `subagent` acepta `description`, `prompt`, `provider` y `model`. Cada rol consume su encargo desde [`.agents/subagents/`](.agents/subagents/):

| Rol | Encargo | Invocación en DSH |
| :--- | :--- | :--- |
| `Investigador` | [investigador.md](.agents/subagents/investigador.md) | `subagent(description='Investigador', provider='google', model='gemini-3.8-flash', prompt=<contenido> + tarea)` |
| `Proposal Reviewer`| [proposal-reviewer.md](.agents/subagents/proposal-reviewer.md) | `subagent(description='Proposal Reviewer', provider='google', model='gemini-3.8-flash', prompt=<contenido> + tarea)` |
| `Constructor` | [constructor.md](.agents/subagents/constructor.md) | `subagent(description='Constructor', provider='google', model='gemini-3.8-flash', prompt=<contenido> + tarea)` |
| `Deep Debugger` | [investigador.md](.agents/subagents/investigador.md) | `subagent(description='Deep Debugger', provider='google', model='gemini-4-argon', prompt=<contenido> + tarea)` *(Solo escalamiento)* |
| `Code Scout` | [code-scout.md](.agents/subagents/code-scout.md) | `subagent(description='Code Scout', provider='google', model='gemini-3.5-flash-lite', prompt=<contenido> + tarea)` |

- `subagent_fork` no acepta modelo: el hijo hereda el del padre. Solo para tareas que necesiten el contexto de esta conversación.
- En Antigravity se invoca mediante `invoke_subagent(TypeName='self', Role='<rol>', Model='flash'|'flash_lite'|'pro', Prompt='...')` (prohibido `inherit`).

---

## 5. Skills Locales, Zonas Sensibles y Fuentes de Verdad Auto-Indexadas

Las skills completas residen en [`.agents/skills/`](.agents/skills/):
- **`propuesta-verificable`:** Flujo obligatorio antes de tocar código: Nivel 0 (trivial/texto), Nivel 1 (1 archivo en chat con Antes/Ahora), Nivel 2 (crítico en `docs/propuestas/`). Prohibido editar sin aprobación previa. En Nivel 2, incrustar obligatoriamente la previsualización interactiva con `<agent-embed>`.
  - **Zonas Sensibles (fuerzan Propuesta Nivel 2):** `src/ppu_nativa.rs`, `src/ppu_nativa.wgsl`, `src/audio.rs`, `src/recomp.rs`, `src/pipeline.rs`, `src/runner.rs`, `../gba-recomp/crates/gba-core/**`, `Cargo.toml`, `pack.toml`, `android/AndroidManifest.xml` y `.agents/hooks/*.py`.
- **`diataxis-docs`:** Estructura de documentación (Tutoriales, Guías, Referencia, [Explicación](docs/explicacion/README.md) / [DECISIONES.md](docs/explicacion/DECISIONES.md)).
- **`test-generation`:** Diseño de pruebas de frontera y robustez. Suite en verde antes de modificar.
- **`lenguaje-claro-iso-24495`:** Directriz transversal obligatoria: ir directo al grano, sin rodeos ni fórmulas de relleno.

### 5.1 Protocolo Dual de Grafos (CodeGraph vs. Graphify) y Fuentes de Verdad

El hook `session_init.py` y el indexador `scripts/indexar_contexto_y_fuentes.py` mantienen sincronizados y libres de código histórico (`docs/historico/` excluido) dos grafos complementarios:
- **CodeGraph (`.codegraph/`):** Exclusivo para código fuente vivo (`src/`, `tests/`, `scripts/`, `../gba-recomp/crates/gba-core/`), extracción de snippets verbatim y call-graph.
- **Graphify (`graphify-out/wiki/index.md` y `.agents/telemetry/sources_index.json`):** Grafo de conocimiento y Fuentes de Verdad ([`docs/fuentes/`](docs/fuentes/README.md), [`docs/referencia/`](docs/referencia/README.md), [`docs/explicacion/DECISIONES.md`](docs/explicacion/DECISIONES.md), [`GLOSSARY.md`](GLOSSARY.md) y símbolos de shaders `src/*.wgsl`). Se auto-expande automáticamente al añadir o editar documentos en `docs/`.

> [!NOTE]
> **Automatización del Arnés:** Al guardar archivos en `docs/` o `src/`, el hook `post_tool_runner` ejecuta automáticamente `actualizar_indice_propuestas.py`, `validar_propuesta_y_skills.py`, `validar_docs.py`, `indexar_contexto_y_fuentes.py` y `cargo fmt`.

---

## 6. Comandos Canónicos de Referencia

| Propósito | Comando Oficial |
| :--- | :--- |
| **Resumen Determinista de Tests** | `python scripts/run_tests_summary.py [args]` |
| **Resumen Determinista de APK / AOT** | `python scripts/build_apk_summary.py` (o `--aot`) |
| **Indexar Fuentes, WGSL y Grafos** | `python scripts/indexar_contexto_y_fuentes.py` |
| **Validar Propuestas y Skills** | `python scripts/validar_propuesta_y_skills.py <ruta.md>` |
| **Activar Live Dock** | `python scripts/activar_live_dock.py [artifact_dir]` |
| **Verificar Sincronía Versiones** | `python scripts/bump_version.py --check` |
| **Simular Incremento Versión** | `python scripts/bump_version.py --dry-run --patch` |
| **Empaquetado Fat APK** | `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` |
| **Análisis de Telemetría** | `python scripts/diagnostico/analizar_telemetria.py --archivo <log.txt> --top <N>` |
| **Desensamblado ROM** | `python scripts/diagnostico/desensamblar_rom.py --buscar-prologo <addr>` |
| **Auditoría Labels AOT** | `python scripts/diagnostico/auditar_labels.py` |
| **Compilar AOT C11** | `powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1` |
| **Validar Documentación** | `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` |
| **Consultar Registro de Agentes** | `python .agents/telemetry/ledger.py --tail 25` (o `--stats`, `--role <rol>`) |
| **Visor Nativo de Agentes (GUI)** | `python .agents/telemetry/ledger.py --gui` (o `abrir_visor_agentes.bat`) |
