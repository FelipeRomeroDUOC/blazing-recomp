# Propuesta: Resolución integral de inconsistencias técnicas y documentales de RANT.md

Fecha: 2026-10-23
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se resuelven de forma integral las quejas de comprensión y legibilidad señaladas en [`RANT.md`](../../RANT.md), alineando la documentación técnica, los metadatos y las herramientas auxiliares con el estado real del repositorio:

1. **Instrucciones de compilación y estado del port en [`README.md`](../../README.md):**
   - Se reemplazan los comandos externos no ejecutables (`cargo run -p gba-pack...` y `recomp.exe...`) por las instrucciones oficiales y funcionales de este repositorio:
     - Empaquetado APK para Android mediante `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`.
     - Ejecución local del visor interactivo en PC mediante `cargo run --bin ppu_doble`.
   - Se transparenta el estado del subsistema de audio: se aclara que el emulador de hardware (`gba-core`) procesa las estructuras de `M4A / MusicPlayer2000`, pero que el backend de salida nativa para Android (`AAudio / Oboe`) se encuentra actualmente en desarrollo (Hito 6 del Roadmap).
   - Se actualiza la mención a `mods/` especificando que es una carpeta planificada para la fase de personalización comunitaria.
   - Se corrigen los enlaces del bloque de documentación para incluir explícitamente el cuadrante de Guías Prácticas (`docs/guias/README.md`) y enlazar al portal general [`docs/README.md`](../../docs/README.md).

2. **Creación del portal índice [`docs/README.md`](../../docs/README.md):**
   - Se crea el documento raíz de documentación que estructura el sistema Diátaxis, ofreciendo navegación clara a los cuatro cuadrantes: Guías Prácticas, Referencia Técnica, Explicación y Propuestas/Backlog.

3. **Corrección de la ABI `RtApi` y volumen de unidades C en [`docs/explicacion/arquitectura-c-recompilado.md`](../../docs/explicacion/arquitectura-c-recompilado.md):**
   - Se sustituye la definición ficticia de `RtApi` (`rb`, `rw`, `swi`) por la definición canónica real de [`src-c/1575 - Fire Emblem (E)(Independent)-bios.0.c`](../../src-c/1575%20-%20Fire%20Emblem%20%28E%29%28Independent%29-bios.0.c) y `gba-core::capi`:
     - Punteros reales: `interp_one`, `read8`, `read16`, `read32`, `write8`, `write16`, `write32`, `tick`, `guard` y `chain_gate`.
     - Explicación técnica detallada del propósito de `interp_one` (ejecución de 1 instrucción en fallback), `guard` (verificación de integridad FNV-1a para bloques en RAM) y `chain_gate` (detección de interrupciones, VBlank y detención de CPU en saltos hacia atrás).
   - Se actualiza la tabla de archivos C generados reflejando que la partición actual abarca 12 unidades de traducción (`bios.0.c` a `bios.11.c`) y el archivo de símbolos `bios.def`.

4. **Saneamiento del extractor de referencias [`scripts/extraer_referencias.py`](../../scripts/extraer_referencias.py) y [`docs/referencia/api-modulos-rust.md`](../../docs/referencia/api-modulos-rust.md):**
   - Se añaden los módulos gráficos y de presentación (`ppu_nativa.rs`, `pipeline.rs`, `present.rs`, `perf.rs`) a la lista de módulos procesados en `scripts/extraer_referencias.py`.
   - Se regenera `docs/referencia/api-modulos-rust.md` incorporando las estructuras y firmas públicas de la PPU nativa y el pipeline GPU.

5. **Aviso contextual histórico en [`docs/explicacion/ppu-caminos-de-composicion.md`](../../docs/explicacion/ppu-caminos-de-composicion.md):**
   - Se incorpora un bloque de advertencia destacado (`> [!NOTE]` / `> [!WARNING]`) al inicio del documento aclarando que describe el funcionamiento interno del rasterizador por software histórico de `gba-core` (`PpuShim`), contrastándolo con el ADR 42 que adoptó el pipeline GPU directo (`NativePpu`).

6. **Corrección de orientación de paneles en [`GLOSSARY.md`](../../GLOSSARY.md):**
   - Se rectifica la descripción de `ppu_doble`: Panel izquierdo = PPU de referencia por software en CPU; Panel derecho = PPU nativa acelerada por GPU (`wgpu`).

7. **Sincronización de tareas concluidas en [`docs/PENDIENTES.md`](../../docs/PENDIENTES.md):**
   - Se eliminan de las secciones 3 y 4 las tareas que ya fueron resueltas e implementadas según los ADRs 18 y 28 (modernización háptica a `VibrationEffect` y sincronización reactiva erradicando el busy-waiting de sondeo nulo).

8. **Corrección de enlaces rotos en [`src-c/README.md`](../../src-c/README.md) y [`examples/README.md`](../../examples/README.md):**
   - En `src-c/README.md`: se actualiza el enlace a `../docs/explicacion/arquitectura-c-recompilado.md`.
   - En `examples/README.md`: se corrige la ruta relativa a `../docs/explicacion/DECISIONES.md#42-...`.

---

### 2. Contexto

- Contexto consultado: [`RANT.md`](../../RANT.md), [`README.md`](../../README.md), [`GLOSSARY.md`](../../GLOSSARY.md), [`docs/PENDIENTES.md`](../../docs/PENDIENTES.md), [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md), [`src-c/README.md`](../../src-c/README.md), [`examples/README.md`](../../examples/README.md), [`scripts/extraer_referencias.py`](../../scripts/extraer_referencias.py).
- Intención: Cerrar la brecha entre la realidad técnica del código y la documentación del repositorio detectada en la auditoría de `RANT.md`, recuperando la máxima fidelidad y exactitud técnica exigida por el perfil de lectores de recompilación e ingeniería inversa.
- Restricciones y decisiones previas (máx. 3):
  - Prohibición de rutas locales absolutas del sistema en archivos versionados (`AGENTS.md`) → enlaces estrictamente relativos.
  - Estricta compatibilidad con Diátaxis y su validador automático (`validar_docs.py`) → metadatos y encabezados canónicos.
  - Mantener los 5 guardianes de pre-commit al 100% en verde (`AGENTS.md`).

---

### 3. Plan de verificación

#### Criterios Dado / Cuando / Entonces
- **Criterio 1 (Sincronización de Referencias de API):**
  - *Dado* que `scripts/extraer_referencias.py` incluye `ppu_nativa.rs`, `pipeline.rs`, `present.rs` y `perf.rs`.
  - *Cuando* se ejecute `python scripts/extraer_referencias.py --check`.
  - *Entonces* el script debe salir con código 0 confirmando sincronización total.
- **Criterio 2 (Validación Diátaxis):**
  - *Dado* el nuevo portal `docs/README.md` y las correcciones de enlaces en `src-c/README.md`, `examples/README.md` y `README.md`.
  - *Cuando* se ejecute `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`.
  - *Entonces* debe reportar 0 errores, 0 avisos y 0 marcas pendientes.
- **Criterio 3 (Suite de pruebas y auditoría de etiquetas intactas):**
  - *Dado* el saneamiento documental y de scripts.
  - *Cuando* se ejecute `cargo test` y `python scripts/diagnostico/auditar_labels.py`.
  - *Entonces* todos los tests y auditorías deben salir en verde al 100%.

---

## Resultado (lo escribe el agente tras verificar)

### 1. Predicho vs. real

| Predicción | Resultado real | Estado |
| :--- | :--- | :---: |
| Sustituir instrucciones de compilación rotas en `README.md` por `package_apk.ps1` y `cargo run --bin ppu_doble`. | Se redactaron las instrucciones oficiales con detalle de controles y flags en [`README.md`](../../README.md). | Conforme |
| Transparentar el estado del subsistema de audio y aclarar el propósito de `mods/`. | [`README.md`](../../README.md) ahora declara con exactitud que el core procesa M4A pero que la salida nativa AAudio/Oboe está en desarrollo (Hito 6). | Conforme |
| Crear el portal general `docs/README.md` estructurando los cuadrantes Diátaxis. | Se creó [`docs/README.md`](../../docs/README.md) con diagrama y enlaces hacia Guías Prácticas, Referencia, Explicación y Propuestas. | Conforme |
| Corregir la definición canónica de `RtApi` en `docs/explicacion/arquitectura-c-recompilado.md`. | Se reemplazaron las firmas ficticias por las reales (`interp_one`, `read8/16/32`, `write8/16/32`, `tick`, `guard`, `chain_gate`) detallando el propósito de cada una. | Conforme |
| Actualizar tabla de archivos C generados a 12 unidades (`bios.0.c` a `bios.11.c`). | La tabla en [`docs/explicacion/arquitectura-c-recompilado.md`](../../docs/explicacion/arquitectura-c-recompilado.md) refleja fielmente las 12 unidades y sus roles. | Conforme |
| Incluir módulos gráficos en `scripts/extraer_referencias.py` y regenerar `api-modulos-rust.md`. | Se agregaron `ppu_nativa.rs`, `pipeline.rs`, `present.rs`, `perf.rs`, etc. La referencia aumentó a 36.376 bytes y pasó la verificación estricta (`--check`). | Conforme |
| Contextualizar históricamente `docs/explicacion/ppu-caminos-de-composicion.md`. | Se añadió aviso destacado inicial sobre el rasterizador por software histórico de `gba-core` contrastando con el ADR 42 (`NativePpu` en GPU). | Conforme |
| Corregir orientación de paneles en `GLOSSARY.md` (Izq = CPU software / Der = GPU nativa). | Actualizado en [`GLOSSARY.md`](../../GLOSSARY.md) en total concordancia con la guía del visor doble. | Conforme |
| Depurar tareas ya resueltas en `docs/PENDIENTES.md` (háptica moderna y busy-waiting). | Se retiraron las tareas resueltas en los ADRs 18 y 28 de [`docs/PENDIENTES.md`](../../docs/PENDIENTES.md). | Conforme |
| Corregir enlaces rotos en `src-c/README.md` y `examples/README.md`. | Ambos archivos ahora enlazan a las rutas relativas existentes (`arquitectura-c-recompilado.md` y `DECISIONES.md#42...`). | Conforme |
| Suite de pruebas al 100% en verde. | `cargo test` ejecutado y aprobado con 292 tests en verde (191 unitarios + 101 integración). | Conforme |
| Validador Diátaxis sin errores ni avisos. | `validar_docs.py` ejecutado: 20 páginas revisadas, 0 errores, 0 avisos. | Conforme |
| Auditoría de labels AOT aprobada. | `auditar_labels.py` ejecutado: 18.483 funciones conformes, 0 colisiones. | Conforme |
| Cero fugas de rutas absolutas locales. | Verificación de diffs y nuevos archivos limpia de rutas absolutas del usuario o sistema. | Conforme |

### 2. Hallazgos fuera de alcance (no corregidos en esta propuesta)
- Ninguno. Se abordaron y resolvieron la totalidad de quejas y bloqueos formulados en [`RANT.md`](../../RANT.md).
