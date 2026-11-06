# Subagentes de Blazing Recomp

Encargos reutilizables para delegar trabajo técnico con razonamiento real fuera del hilo principal. Los roles y sus
disparadores son los de la matriz de [AGENTS.md §4](../../AGENTS.md); aquí vive el texto que se le pasa
al subagente y el modelo Gemini que le corresponde.

> [!IMPORTANT]
> **Protección de Cuota Argon, Prohibición de `inherit` y Scripts Deterministas:**
> - El parámetro `Model` en Antigravity debe declararse **siempre explícitamente** (`flash`, `flash_lite` o `pro`). No se permite `inherit`.
> - **Prioridad Gemini 3.8 Flash (`flash`):** Todo el trabajo analítico, diagnóstico (`Investigador`), auditoría editorial (`Proposal Reviewer`) y construcción de código (`Constructor`) se ejecuta en **Gemini 3.8 Flash**. **Gemini 4 Argon (`pro`)** queda reservado exclusivamente como escalamiento de último recurso (`Deep Debugger`).
> - **Roles podados / reemplazados por código determinista:**
>   - `Doc Manager`: podado (automatizado por `post_tool_runner.py`).
>   - `Test Runner`: podado y reemplazado por `python scripts/run_tests_summary.py`.
>   - `APK Builder`: podado y reemplazado por `python scripts/build_apk_summary.py`.

## Índice por División de Capacidades

### 1. División Reserva Crítica de Escalamiento (Gemini 4 Argon / Pro)
| Rol | Encargo | Disparador Principal | Modelo Antigravity | Modelo DSH |
| :--- | :--- | :--- | :--- | :--- |
| **Deep Debugger** *(Escalamiento)* | [investigador.md](investigador.md) | Solo si `Investigador` (`3.8 Flash`) no logra aislar un fallo crítico tras un primer intento | `pro` | `google/gemini-4-argon` |

### 2. División Núcleo Técnico y Analítico (Gemini 3.8 Flash)
| Rol | Encargo | Disparador Principal | Modelo Antigravity | Modelo DSH |
| :--- | :--- | :--- | :--- | :--- |
| **Investigador** | [investigador.md](investigador.md) | Diagnóstico de causas raíz, arquitectura PPU/Audio/DMA, CodeGraph y ROM | `flash` | `google/gemini-3.8-flash` |
| **Proposal Reviewer** | [proposal-reviewer.md](proposal-reviewer.md) | Auditoría editorial (ISO 24495-1) y verificabilidad de propuestas N2 | `flash` | `google/gemini-3.8-flash` |
| **Constructor** | [constructor.md](constructor.md) | Programación de código nuevo, tests unitarios y módulos aprobados en Rust/C | `flash` | `google/gemini-3.8-flash` |

### 3. División Exploración Masiva (Flash-Lite)
| Rol | Encargo | Disparador Principal | Modelo Antigravity | Modelo DSH |
| :--- | :--- | :--- | :--- | :--- |
| **Code Scout** | [code-scout.md](code-scout.md) | Contingencia: búsquedas no indexadas de más de 10 archivos completos | `flash_lite` | `google/gemini-3.5-flash-lite` |

## Cómo se invoca en Antigravity

```text
# 1. Núcleo Técnico y Analítico (flash -> Gemini 3.8 Flash)
invoke_subagent(TypeName='self', Role='Investigador', Model='flash', Prompt='...')
invoke_subagent(TypeName='self', Role='Proposal Reviewer', Model='flash', Prompt='...')
invoke_subagent(TypeName='self', Role='Constructor', Model='flash', Prompt='...')

# 2. Exploración Masiva (flash_lite -> Gemini 3.5 Flash-Lite)
invoke_subagent(TypeName='self', Role='Code Scout', Model='flash_lite', Prompt='...')

# 3. Escalamiento Crítico de Último Recurso (pro -> Gemini 4 Argon)
invoke_subagent(TypeName='self', Role='Deep Debugger', Model='pro', Prompt='...')
```

## Registro Persistente y Consulta entre Agentes (`ledger.py`)

Toda acción del Orquestador y de los subagentes queda registrada persistentemente en SQLite (`.agents/telemetry/agent_ledger.db`) y reflejada en `.agents/telemetry/ACTIVITY_LOG.md`.

```bash
# Ver las últimas 20 acciones de todos los agentes
python .agents/telemetry/ledger.py --tail 20

# Filtrar acciones de un rol o sesión específica
python .agents/telemetry/ledger.py --role "Investigador"

# Ver estadísticas de uso por nivel de modelo (Argon, Flash 3.8, Flash-Lite)
python .agents/telemetry/ledger.py --stats

# Abrir el visor nativo de escritorio (para humanos, sin navegador)
python .agents/telemetry/ledger.py --gui
```
