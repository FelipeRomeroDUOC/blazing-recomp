# Subagentes de Blazing Recomp

Encargos reutilizables para delegar trabajo técnico fuera del hilo principal. Los roles y sus
disparadores son los de la matriz de [AGENTS.md §4](../../AGENTS.md); aquí vive el texto que se le pasa
al subagente y el modelo Gemini que le corresponde.

## Índice por División de Capacidades

### 1. División Razonamiento Crítico (Modelos Pro)
| Rol | Encargo | Disparador Principal | Modelo Antigravity | Modelo DSH |
| :--- | :--- | :--- | :--- | :--- |
| **Investigador** | [investigador.md](investigador.md) | Diagnóstico de causas raíz, bugs complejos, arquitectura y ROM | `pro` | `google/gemini-1.5-pro` |
| **Proposal Reviewer** | [proposal-reviewer.md](proposal-reviewer.md) | Auditoría editorial (ISO 24495-1) y verificabilidad de propuestas N2 | `pro` | `google/gemini-1.5-pro` |
| **Constructor** | [constructor.md](constructor.md) | Programación de código nuevo y módulos aprobados en Rust/C | `pro` (o `flash`) | `google/gemini-1.5-pro` |

### 2. División Ejecución Mecánica (Modelos Flash-Lite)
| Rol | Encargo | Disparador Principal | Modelo Antigravity | Modelo DSH |
| :--- | :--- | :--- | :--- | :--- |
| **Test Runner** | [test-runner.md](test-runner.md) | Ejecución de suites de tests, fotogramas dorados y mutantes | `flash_lite` | `google/gemini-3.1-flash-lite` |
| **APK Builder** | [apk-builder.md](apk-builder.md) | Compilar AOT C11, `cargo ndk`, `cargo build`, Fat APK | `flash_lite` | `google/gemini-3.5-flash-lite` |
| **Code Scout** | [code-scout.md](code-scout.md) | Contingencia: búsquedas no indexadas de más de 10 archivos completos | `flash_lite` | `google/gemini-3.5-flash-lite` |

## Reparto y cuota

- **Modelos Pro:** Se reservan estrictamente para tareas de razonamiento profundo (diagnóstico de causas raíz, auditoría de propuestas y desarrollo de código algorítmico).
- **Modelos Flash-Lite:** Cada modelo lite tiene 500 peticiones diarias. Absorben el 90% del trabajo pesado diario (compilación de APKs y suites de tests masivas) con coste despreciable de cuota.

## Cómo se invoca en DeepSeek Harness (DSH)

La herramienta `subagent` acepta `description`, `prompt`, `provider` y `model`. El rol viaja dentro del `prompt`.

1. Abre el archivo del rol y copia su contenido completo.
2. Ese contenido es el `prompt`. Añade al final la tarea concreta.
3. Pasa el `provider` y el `model` que declara el encargo, y usa el nombre del rol como `description`.

```text
subagent(
  description = "Test Runner",
  provider    = "google",
  model       = "gemini-3.1-flash-lite",
  prompt      = "<contenido de .agents/subagents/test-runner.md>

Tarea: ejecuta la suite de tests unitarios y reporta el resultado en 3 líneas."
)
```

## Cómo se invoca en Antigravity

```text
# Para tareas de razonamiento (pro)
invoke_subagent(TypeName='research', Role='Investigador', Model='pro', Prompt='...')
invoke_subagent(TypeName='research', Role='Proposal Reviewer', Model='pro', Prompt='...')
invoke_subagent(TypeName='self', Role='Constructor', Model='pro', Prompt='...')

# Para tareas mecánicas (flash_lite)
invoke_subagent(TypeName='self', Role='Test Runner', Model='flash_lite', Prompt='...')
invoke_subagent(TypeName='self', Role='APK Builder', Model='flash_lite', Prompt='...')
```

## Roles podados o absorbidos

- **Doc Manager (Podado):** Los hooks `post_tool_runner` regeneran índices, validan Diátaxis y actualizan la preview automáticamente.
- **Telemetry Analyzer y Label Auditor (Absorbidos):** Absorbidos como misiones dinámicas del **Investigador** cuando se requiere análisis profundo de rendimiento o desensamblado ROM.
