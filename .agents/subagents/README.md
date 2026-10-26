# Subagentes de Blazing Recomp

Encargos reutilizables para delegar trabajo pesado fuera del hilo principal. Los roles y sus
disparadores son los de la matriz de [AGENTS.md §4](../../AGENTS.md); aquí vive el texto que se le pasa
al subagente y el modelo Gemini que le toca.

## Índice

| Rol | Encargo | Disparador | Modelo |
| :--- | :--- | :--- | :--- |
| **Test Runner** | [test-runner.md](test-runner.md) | `cargo test`, arnés de fotogramas dorados, auditoría de mutantes | `google/gemini-3.1-flash-lite` |
| **APK Builder** | [apk-builder.md](apk-builder.md) | Compilar AOT C11, `cargo ndk`, `cargo build`, Fat APK | `google/gemini-3.5-flash-lite` |
| **Telemetry Analyzer** | [telemetry-analyzer.md](telemetry-analyzer.md) | Analizar `blazing-perf.txt`, frametimes, cuellos de botella GPU/PPU | `google/gemini-3.1-flash-lite` |
| **Label Auditor** | [label-auditor.md](label-auditor.md) | Auditar `blazing.labels.toml`, desensamblar ROM, sembrar labels | `google/gemini-3.5-flash-lite` |
| **Code Scout** | [code-scout.md](code-scout.md) | Contingencia: búsquedas no indexadas de más de 10 archivos completos | `google/gemini-3.5-flash-lite` |

## Reparto y cuota

Cada uno de los dos modelos lite tiene 500 peticiones diarias; repartir los roles permite usar hasta
1000. DSH no cambia de modelo solo: si un modelo devuelve error de cuota, se edita el `Modelo:` del
encargo afectado y se pasa el otro id en la llamada. Dos roles usan 3.1 y tres usan 3.5.

## Cómo se invoca en DeepSeek Harness (DSH)

La herramienta `subagent` acepta `description`, `prompt`, y —desde que la selección de modelo por
subagente está activada— `provider` y `model`. El rol viaja dentro del `prompt`.

1. Abre el archivo del rol y copia su contenido completo.
2. Ese contenido es el `prompt`. Añade al final la tarea concreta.
3. Pasa el `provider` y el `model` que declara el encargo, y usa el nombre del rol como `description`.

```text
subagent(
  description = "Test Runner",
  provider    = "google",
  model       = "gemini-3.1-flash-lite",
  prompt      = "<contenido de .agents/subagents/test-runner.md>

Tarea: ejecuta cargo test --test subagentes_dsh_tests y dime si los tests pasan."
)
```

Variantes:

- `subagent_fork` no acepta modelo: el hijo hereda el del padre. Úsalo solo cuando la tarea necesite
  el contexto de esta conversación.
- `workflow` cuando hay tres o más tareas; sus `agent(prompt, { model })` también aceptan modelo, pero
  el arnés lo reserva para orquestaciones.

## Antigravity (herencia)

En el arnés de Antigravity se mantiene la invocación original, con su modelo histórico:

```text
invoke_subagent(TypeName='self', Role='Test Runner', Model='flash_lite', Prompt='...')
```

## Roles podados

- **Doc Manager:** podado. No tiene archivo de encargo y no debe crearse: los hooks `post_tool_runner`
  regeneran índices, validan Diátaxis y actualizan la preview sin intervención.
