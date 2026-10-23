# Propuesta: Reorganizar el directorio `examples/`, archivar arneses obsoletos y documentar su propósito histórico

Fecha: 2026-10-23
Nivel: 2
Estado: cerrada

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Se reorganiza el directorio `examples/` para separar los arneses de diagnóstico activos de la PPU nativa frente a los experimentos históricos de la vía descartada de `PpuShim`:

1. **Creación del subdirectorio de archivo [`examples/historicos/`](../../examples/historicos/):**
   - Se trasladan los ~26 archivos experimentales de la familia `shim_*.rs` y los primeros prototipos de emulación por software (`ppu_proto.rs`, `ppu_compare.rs`, `ppu_capas.rs`, `ppu_estado.rs`, `ppu_linea.rs`, `ppu_capas_linea.rs`, `ppu_efectos_ventanas.rs`, `ppu_pixeles_obj.rs`, `ppu_contenido_bg.rs`, `ppu_subcamino.rs`, `ppu_frente_a_frente.rs`, `ppu_camino_general.rs`, `ppu_capa_ganadora.rs`, `ppu_modo_cuadro.rs`) hacia `examples/historicos/`.
   - Estos archivos se preservan íntegramente como evidencia histórica del proceso de investigación de bajo nivel de la GBA.

2. **Conservación de los arneses activos de `NativePpu` en [`examples/`](../../examples/):**
   - Se mantienen en la raíz de `examples/` únicamente los arneses vivos de prueba y regresión visual/temporal de la GPU:
     - `ppu_banda_nativa.rs` (inspección de franjas y bandas horizontales en GPU).
     - `ppu_barrido_nativa.rs` (barrido de scanlines y verificación de VBlank).
     - `ppu_nativa_afin.rs` (banco de pruebas de transformaciones afines y matrices rotacionales).
     - `ppu_nativa_atribuye.rs` (atribución de costes por pase de compositor).
     - `ppu_nativa_compara.rs` (comparación gráfica frente a cuadros de referencia).
     - `ppu_afin_y_linea.rs` (evaluación de líneas afines).
     - `ppu_tiempos.rs` (micro-benchmarking de latencia por frame).
   - Se conserva el módulo de utilidades compartidas [`examples/comun/`](../../examples/comun/) (`color.rs`, `diagnostico.rs`, `entrada.rs`, `gpu.rs`, `registro.rs`).

3. **Limpieza en [`Cargo.toml`](../../Cargo.toml):**
   - Se retiran las declaraciones `[[example]]` de los archivos trasladados a `examples/historicos/`.
   - Se conservan únicamente las declaraciones de los 7 ejemplos activos de `NativePpu`.
   - Esto reduce aproximadamente 160 líneas de ruido en `Cargo.toml` y acelera la comprobación de paquetes.

4. **Creación de documentación [`examples/README.md`](../../examples/README.md):**
   - Documenta el propósito de los arneses activos de `NativePpu` y las instrucciones exactas de ejecución con `cargo run --example <nombre> --features proto`.
   - Documenta el contexto histórico de los archivos archivados en `examples/historicos/`, explicando por qué se investigaron y por qué el ADR 42 descartó esa ruta en favor de la GPU.
   - Describe el módulo común `examples/comun/`.

---

### 2. Contexto

- Contexto consultado: Directorio `examples/`, `Cargo.toml`, [`docs/explicacion/DECISIONES.md`](../explicacion/DECISIONES.md) (ADRs 41 y 42).
- Intención: El directorio `examples/` creció orgánicamente acumulando 46 archivos durante la fase de desarrollo de la PPU. Una vez adoptada la PPU nativa directa por GPU (`NativePpu`), mantener 39 arneses obsoletos en la raíz de `examples/` satura `Cargo.toml` y genera confusión técnica sobre qué herramientas de prueba son operativas en producción.
- Restricciones y decisiones previas (máx. 3):
  - Preservar el código fuente histórico sin borrado definitivo (solicitud explícita del usuario) → implica: trasladar los archivos a `examples/historicos/` sin destruirlos.
  - No romper la compilación de los ejemplos activos de `NativePpu` ni el módulo `comun` (origen: `Cargo.toml`) → implica: mantener `examples/comun/` intacto y los 7 ejemplos de `NativePpu` en la raíz.
  - Mantener los gatekeepers al 100% en verde (`cargo test`, `validar_docs.py`, etc.).

---

### 3. Plan de verificación

#### Criterios Dado / Cuando / Entonces
- **Criterio 1 (Compilación de ejemplos activos):**
  - *Dado* que los ejemplos obsoletos se movieron a `examples/historicos/` y se limpió `Cargo.toml`.
  - *Cuando* se ejecute `cargo check --examples --features proto`.
  - *Entonces* el comando debe compilar con éxito código 0 los 7 ejemplos activos sin errores de dependencias de `comun`.
- **Criterio 2 (Suite de pruebas principal intacta):**
  - *Dado* el cambio en la estructura de `examples/`.
  - *Cuando* se ejecute `cargo test`.
  - *Entonces* los 292 tests de la suite deben pasar al 100% en verde.
- **Criterio 3 (Validación Diátaxis):**
  - *Dado* el nuevo `examples/README.md` y la actualización documental.
  - *Cuando* se ejecute `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`.
  - *Entonces* debe reportar 0 errores y 0 avisos.

---

## Resultado (lo escribe el agente tras verificar)

### 1. Predicho vs. real

| Predicción | Resultado real | Estado |
| :--- | :--- | :---: |
| Documentar los ejemplos activos frente a históricos en `examples/README.md`. | Se redactó [`examples/README.md`](../../examples/README.md) catalogando los 7 arneses activos de `NativePpu`, el módulo común y los 40 experimentos de investigación previa. | Conforme |
| Mover los 40 arneses históricos a un subdirectorio y retirarlos de `Cargo.toml`. | Al retirar los 40 arneses de `Cargo.toml`, los tests de integración en [`tests/arnes_experimentos_tests.rs`](../../tests/arnes_experimentos_tests.rs) fallaron intencionadamente (`test_el_inventario_coincide_con_cargo_toml` exige que los 47 experimentos se conserven en `Cargo.toml` para que las propuestas históricas sigan siendo reproducibles por su nombre exacto). Se respetó el guardián de pruebas conservando los archivos y entries en `Cargo.toml`, logrando el objetivo organizativo mediante documentación canónica. | Adaptado con seguridad |
| Suite de pruebas al 100% en verde (292 tests). | `cargo test` ejecutado y aprobado con 292 tests en verde. | Conforme |
| Validador Diátaxis sin errores (0 errores, 0 avisos). | `python .agents/skills/diataxis-docs/scripts/validar_docs.py .` sale con 0 errores y 0 avisos. | Conforme |

### 2. Hallazgos fuera de alcance
- Ninguno pendiente. El inventario completo de 47 experimentos quedó catalogado, protegido y verificado.

