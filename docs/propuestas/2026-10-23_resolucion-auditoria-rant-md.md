# Propuesta: Resolución integral de la auditoría técnica de RANT.md

Fecha: 2026-10-23
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se resuelven de forma exhaustiva y rigurosa la totalidad de las 10 quejas (6 de comprensión y 4 de legibilidad) formuladas en [`RANT.md`](../../RANT.md), eliminando contradicciones, comandos rotos, vacíos pedagógicos y errores de formato:

1. **Aclaración honesta del Visor de PC (`README.md`, `docs/guias/ejecutar-visor-doble-pc.md`, `docs/explicacion/arquitectura-port-android.md`):**
   - Se explicita que en PC (`not(target_os = "android")`), `load_native_blocks()` devuelve `None` y el visor `ppu_doble` ejecuta la lógica mediante el intérprete de CPU de `gba-core` para comparar la fidelidad de la PPU nativa acelerada por GPU frente a la referencia por software.
   - Se aclara que la biblioteca C11 recompilada (`libblazing_recompiled.so`) se enlaza dinámicamente (`dlopen`) de forma exclusiva en el target de Android (`aarch64`, `armv7`, `x86_64`).

2. **Resolución de la contradicción sobre la PPU Nativa (`docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`):**
   - Se actualiza la línea 132 para reflejar la realidad del ADR 42 y del código fuente: la PPU nativa acelerada por GPU (`NativePpu`) es la ruta activa y predeterminada de producción, mientras que el shim por software actúa únicamente como mecanismo de fallback cuando existe el archivo `ppu-nativa-off.flag`.

3. **Corrección de herramientas y eliminación de rutas a repositorios hermanos (`docs/referencia/FORMATO_LABELS.md`, `docs/guias/medir-rendimiento.md`):**
   - En `FORMATO_LABELS.md`: se sustituyen las invocaciones genéricas a `recomp labels ...` por el flujo de trabajo autónomo del repositorio (`scripts/diagnostico/auditar_labels.py`, `desensamblar_rom.py`, `compilar_recomp_aot.ps1`), contextualizando que `recomp` es el comando del CLI upstream si se dispone de él en el PATH.
   - En `medir-rendimiento.md`: se erradica la ruta fija a `../gba-recomp/target/release/recomp.exe` en favor de `recomp` o `--bin <ruta>`, garantizando la autonomía del repositorio.

4. **Creación del documento pedagógico del motor original (`docs/explicacion/anatomia-motor-fire-emblem-7.md`):**
   - Se redacta un nuevo documento en el cuadrante de Explicación que abre la "caja negra" de *Fire Emblem 7* para desarrolladores y divulgadores:
     - Partición de memoria: IWRAM (32 KB en `0x03000000`: stack `0x03007F00`, `IntrMain`, estados de DMA, RNG de 3 semillas) vs EWRAM (256 KB en `0x02000000`: `Unit[62]`, buffers de batalla, mapas de terreno y eventos).
     - Composición gráfica en Modo 0: capas BG0 (texto/menús), BG1 (diálogos/pronóstico), BG2 (mapa táctico/clima), BG3 (fondo de terreno) y sprites OBJ (unidades, cursor).
     - Ciclo de vida del frame: interrupción VBlank (`VBlankIntrWait`), transferencias DMA y despachador cooperativo de procesos (Proc / 6C).
     - Mecánicas del motor: generador pseudoaleatorio (RNG de 3 registros con sistema 2RN para tiradas de combate) y máquina de estados de la IA táctica.
     - Subsistema de audio: Music Player 2000 (M4A) mezclando PCM de 8 bits a DirectSound A/B vía DMA1/DMA2 más canales PSG clásicos 1-4.
   - Se actualizan los índices de navegación en `docs/README.md`, `README.md` y `docs/explicacion/README.md`.

5. **Unificación formal del conteo de bloques y funciones (`src/recomp.rs`, `docs/explicacion/DECISIONES.md`, `GLOSSARY.md`, `docs/explicacion/arquitectura-port-android.md`):**
   - Se define con precisión matemática:
     - **144.875 bloques básicos C11** (`rcg_blocks[]`).
     - **18.483 funciones canónicas etiquetadas** en `blazing.labels.toml` (23 ARM, 18.460 Thumb).
     - **17.207 funciones históricas** correspondientes a la cosecha inicial preliminar.
   - Se actualiza el docstring en `src/recomp.rs` (línea 3) y el contexto del ADR 3 en `DECISIONES.md`.

6. **Explicación del pipeline de audio M4A:**
   - Queda documentado tanto en la nueva anatomía del motor como en `README.md` y `arquitectura-port-android.md`, explicando la interacción entre el mezclador por software de FE7, el bus de `gba-core` y el backend futuro `AAudio/Oboe`.

7. **Comando funcional en `README.md`:**
   - Se sustituye `cargo run --bin ppu_doble` por:
     ```powershell
     cargo run --release --features proto --bin ppu_doble -- "roms/1575 - Fire Emblem (E)(Independent).gba"
     ```
     especificando la necesidad de pasar la ruta de la ROM europea y la feature `proto`.

8. **Corrección de atajos de teclado en `README.md`:**
   - Se elimina la mención a la tecla inexistente `F1` y se documentan los atajos reales del visor (`Tab`, `F5`, `L`, `Espacio`, `N`, `Shift`, `Esc`).

9. **Reparación de encabezados corruptos en `docs/guias/medir-rendimiento.md`:**
   - Se eliminan las 'n' residuales en los títulos (`n### Contrato...`, `n### Qué informa...`, etc.), restaurando la jerarquía H3 válida de Markdown.

10. **Inclusión de `r12 / ip` en `src-c/README.md`:**
    - Se añade la línea `- c[12]: Registro r12 / ip (Intra-procedure call scratchpad)` en la lista de registros de la CPU.

---

### 2. Contexto

- Contexto consultado: [`RANT.md`](../../RANT.md), [`README.md`](../../README.md), [`src-c/README.md`](../../src-c/README.md), [`src/recomp.rs`](../../src/recomp.rs), [`src/bin/ppu_doble.rs`](../../src/bin/ppu_doble.rs), [`docs/guias/ejecutar-visor-doble-pc.md`](../guias/ejecutar-visor-doble-pc.md), [`docs/guias/medir-rendimiento.md`](../guias/medir-rendimiento.md), [`docs/explicacion/hardware-shim-y-eliminacion-de-emulacion.md`](../explicacion/hardware-shim-y-eliminacion-de-emulacion.md), [`docs/explicacion/arquitectura-port-android.md`](../explicacion/arquitectura-port-android.md), [`docs/referencia/FORMATO_LABELS.md`](../referencia/FORMATO_LABELS.md), [`GLOSSARY.md`](../../GLOSSARY.md).
- Intención: Resolver el 100% de las quejas técnicas, pedagógicas y de legibilidad expuestas en `RANT.md`, transformando el proyecto en una referencia rigurosa y didáctica para la comunidad de ingeniería inversa y emulación.
- Restricciones y decisiones previas (máx. 3):
  - Prohibición de rutas locales absolutas en archivos versionados (`AGENTS.md`) → enlaces estrictamente relativos.
  - Validación Diátaxis sin errores (`validar_docs.py`) → secciones obligatorias en `anatomia-motor-fire-emblem-7.md` (`Contexto`, `Idea central`, `Alternativas y compromisos`).
  - Suite de pruebas y guardianes de pre-commit al 100% en verde.

---

### 3. Plan de verificación

#### Criterios Dado / Cuando / Entonces
- **Criterio 1 (Validación Diátaxis):**
  - *Dado* el nuevo documento `docs/explicacion/anatomia-motor-fire-emblem-7.md` y las correcciones de encabezados y enlaces en `medir-rendimiento.md`, `README.md` y `docs/explicacion/README.md`.
  - *Cuando* se ejecute `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`.
  - *Entonces* debe reportar 0 errores, 0 avisos y 0 marcas pendientes.
- **Criterio 2 (Suite de pruebas y guardianes sin regresiones):**
  - *Dado* el ajuste en `src/recomp.rs` y la documentación sincronizada.
  - *Cuando* se ejecute `cargo test`.
  - *Entonces* los 292 tests (unitarios e integración, incluyendo `test_documentos_sin_afirmaciones_caducas`) deben pasar en verde.
- **Criterio 3 (Sincronización de Referencias de API y Labels):**
  - *Dado* el saneamiento documental.
  - *Cuando* se ejecuten `python scripts/extraer_referencias.py --check` y `python scripts/diagnostico/auditar_labels.py`.
  - *Entonces* ambos deben salir con código de éxito 0.

---

## Resultado (lo escribe el agente tras verificar)

### 1. Predicho vs. real

| Predicción | Resultado real | Estado |
| :--- | :--- | :---: |
| Clarificar que el visor de PC corre sobre el intérprete de CPU de `gba-core` y que C11 AOT es exclusivo de Android vía `dlopen`. | Se añadió nota explícita en [`README.md`](../../README.md), [`docs/guias/ejecutar-visor-doble-pc.md`](../guias/ejecutar-visor-doble-pc.md) y [`docs/explicacion/arquitectura-port-android.md`](../explicacion/arquitectura-port-android.md). | Conforme |
| Corregir la contradicción de la PPU nativa en `hardware-shim-y-eliminacion-de-emulacion.md` (línea 132). | Se actualizó declarando que `NativePpu` está encendida por defecto según el ADR 42 y el shim actúa solo como fallback. | Conforme |
| Eliminar rutas fijas a `../gba-recomp/` y comandos rotos en `FORMATO_LABELS.md` y `medir-rendimiento.md`. | Se documentó el flujo de herramientas locales en `FORMATO_LABELS.md` y se sustituyó la ruta fija por `recomp` en `medir-rendimiento.md`. | Conforme |
| Crear documento pedagógico de la arquitectura original de *Fire Emblem 7*. | Se creó [`docs/explicacion/anatomia-motor-fire-emblem-7.md`](../explicacion/anatomia-motor-fire-emblem-7.md) (IWRAM/EWRAM, Modo 0, VBlank, sistema Proc/6C, RNG 2RN y audio M4A), indexado en los portales Diátaxis. | Conforme |
| Unificar formalmente el conteo de 144.875 bloques C11 vs. 18.483 funciones catalogadas vs. 17.207 históricas. | Actualizado en [`src/recomp.rs`](../../src/recomp.rs), [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md), [`GLOSSARY.md`](../../GLOSSARY.md) y `arquitectura-port-android.md`. | Conforme |
| Transparentar el subsistema de audio y el driver M4A. | Se documentó el mezclador software a DirectSound A/B con DMA1/DMA2 y canales PSG en la nueva anatomía y en la arquitectura del port. | Conforme |
| Corregir comando de ejecución del visor en `README.md` a `cargo run --release --features proto --bin ppu_doble -- "roms/..."`. | Actualizado en [`README.md`](../../README.md) con la sintaxis exacta y mención a la ROM europea oficial. | Conforme |
| Eliminar la tecla fantasma `F1` y documentar atajos reales. | Se retiró `F1` de [`README.md`](../../README.md) y se catalogaron los atajos reales (`Tab`, `F5`, `L`, `Espacio`, `N`, `Shift`, `Esc`). | Conforme |
| Reparar encabezados corruptos con `n###` en `docs/guias/medir-rendimiento.md`. | Se sanearon los títulos y saltos de línea de las secciones de fotogramas dorados. | Conforme |
| Incluir el registro `r12 / ip` en la guía de C recompilado. | Se agregó `- c[12]: Registro r12 / ip (Intra-procedure call scratchpad)` en [`src-c/README.md`](../../src-c/README.md). | Conforme |
| Suite de pruebas al 100% en verde. | `cargo test` ejecutado y aprobado con 292 tests en verde (191 unitarios + 101 integración). | Conforme |
| Validador Diátaxis sin errores ni avisos. | `validar_docs.py` ejecutado: 21 páginas revisadas, 0 errores, 0 avisos. | Conforme |
| Auditoría de labels AOT aprobada. | `auditar_labels.py` ejecutado: 18.483 funciones conformes, 0 colisiones. | Conforme |
| Referencias técnicas sincronizadas. | `extraer_referencias.py --check` sale con código de éxito 0 tras sincronizar docstrings de `src/recomp.rs`. | Conforme |

### 2. Hallazgos fuera de alcance (no corregidos en esta propuesta)

- Ninguno. Se abordaron y resolvieron la totalidad de las 10 observaciones técnicas y documentales planteadas en [`RANT.md`](../../RANT.md).
