# Propuesta: Reversionado a v0.0.3 (Cimientos GPU & AOT C11 Consolidados)

Fecha: 2026-10-24
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

---

## 1. Contexto y Problema

Tras la exitosa integración y verificación del pipeline gráfico acelerado por GPU (**PPU Nativa**, ADR 42) y la ampliación de cobertura de recompilación estática AOT en C11 para rutinas de combate en IWRAM (demostrando empíricamente **0 pasos de intérprete** y **60 FPS estables** durante más de 4 minutos de telemetría continua en Android), se hace necesario avanzar la versión formal de la aplicación.

Conforme a la estrategia de versionado semántico del proyecto ([`docs/explicacion/roadmap-y-versionado.md`](../explicacion/roadmap-y-versionado.md)):
1. El prefijo `0.0.x` cubre la Fase 0 de Cimientos y Prototipo antes del hito `v0.1.0` (MVP Jugable de inicio a fin con guardado persistente `.sav` y audio).
2. La fórmula estándar de sincronización para Android establece:
   $$\text{versionCode} = \text{MAJOR} \times 10000 + \text{MINOR} \times 100 + \text{PATCH}$$
   Para `0.0.3`, $\text{versionCode} = 3$.

---

## 2. Alcance

### Qué incluye
- `Cargo.toml`: `version = "0.0.3"`
- `android/AndroidManifest.xml`: `android:versionCode="3"`, `android:versionName="0.0.3"`
- `pack.toml`: `version = "0.0.3"`
- `CHANGELOG.md`: Cierre de la sección `[Sin publicar]` bajo `## [0.0.3] - 2026-10-24` con registro de los dos hitos arquitectónicos (PPU Nativa y 0 Intérprete).
- Sincronización en la documentación:
  - [`docs/explicacion/roadmap-y-versionado.md`](../explicacion/roadmap-y-versionado.md): actualización de la tabla de versiones y diagramas.
  - [`docs/BACKLOG.md`](../BACKLOG.md): cabecera de versión actual y regeneración del índice de propuestas.
  - [`docs/referencia/CONFIGURACION_PACK.md`](../referencia/CONFIGURACION_PACK.md): versión alineada.

### Qué NO incluye
- Modificaciones al código de ejecución del emulador, shaders WGSL o scripts de empaquetado.

---

## 3. Criterios de Aceptación (Dado / Cuando / Entonces)

- **Dado** el repositorio en la rama `main`.
- **Cuando** se actualicen los metadatos de versión a `0.0.3` / `versionCode = 3`.
- **Entonces:**
  - `cargo check` y `cargo test` compilan y pasan al 100% en verde.
  - `python .agents/skills/diataxis-docs/scripts/validar_docs.py` finaliza con 0 errores y 0 advertencias.
  - Todos los manifiestos coinciden de forma unánime en `0.0.3` y `versionCode = 3`.

---

## 4. Resultados de la Verificación

- `cargo check`: Completado sin errores (`Checking blazing-recomp v0.0.3`).
- `cargo test`: 259 tests pasados al 100% en verde (173 unitarios + 86 de integración).
- `validar_docs.py`: 23 páginas revisadas, 0 errores, 0 avisos.
- `actualizar_indice_propuestas.py`: 160 propuestas indexadas con éxito en `docs/BACKLOG.md`.
