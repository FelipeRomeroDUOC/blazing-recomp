# Propuesta: Cobertura AOT C11 de Subrutinas de Combate en IWRAM

Fecha: 2026-10-24
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

---

## 1. Contexto y Problema

Durante el perfilado y la telemetría del motor en Android (`docs/propuestas/2026-10-12_ppu-60fps-estable.md`), se registraron **7.483 pasos de intérprete por fotograma** durante las animaciones de batalla, concentrados de forma masiva en el hotspot `0x030035C8`.

### Causa raíz
El motor de *Fire Emblem 7* traslada bloques de código crítico desde la ROM hacia la memoria rápida IWRAM (`0x03000000`–`0x03007FFF`) durante el arranque. En `0x03003568`, el juego inicia un bucle de despacho de interpolación de combate que salta a través de una tabla indexada (`mov r0, pc; add r0, r0, #8; add r0, r0, r6, lsl #2; bx r0`) hacia las direcciones `0x030035C8` a `0x030035DC`.

En la versión actual de [`blazing.labels.toml`](../../blazing.labels.toml):
1. **0 subrutinas de IWRAM estaban catalogadas**: las 18.483 funciones catalogadas residían exclusivamente en el espacio de ROM (`0x08xxxxxx`).
2. Al ejecutar las animaciones de batalla, los casos 4 y 5 del salto (`0x030035C8`, `0x030035CC`, `0x030036BC`, `0x030036CC`) no disponían de bloques nativos C11 en `src-c/`, provocando caídas recurrentes al intérprete de software en cada fotograma.

---

## 2. Solución Propuesta

1. **Incorporar las 14 etiquetas de combate en IWRAM a [`blazing.labels.toml`](../../blazing.labels.toml):**
   - Rutina de cálculo matemático e interpolación: `0x03003478` (ARM).
   - Bucle principal de despacho: `0x03003568` (ARM).
   - Tabla de saltos de combate: `0x030035C8`, `0x030035CC`, `0x030035D0`, `0x030035D4`, `0x030035D8` (ARM).
   - Cuerpos de cálculo por caso: `0x030035DC`, `0x03003620`, `0x03003654`, `0x03003688`, `0x030036BC`, `0x030036CC` (ARM).
   - Epílogo y retorno de combate: `0x030036EC`, `0x03003708` (ARM).

2. **Ajustar el validador en `scripts/diagnostico/auditar_labels.py`:**
   - Ampliar la verificación de límite superior de IWRAM para cubrir hasta `0x03008000` (`< 0x03008000` en lugar de `< 0x03007FFF`), asegurando que cualquier dirección en el rango completo de 32 KB sea aceptada.

3. **Habilitar TinyCC en `scripts/diagnostico/compilar_recomp_aot.ps1`:**
   - Configurar `$env:GBA_RECOMP_TCC_LIB` apuntando a la biblioteca empaquetada TinyCC (`libtcc.dll`), permitiendo la compilación nativa de `recomp build` en Windows en menos de 10 segundos sin requerir compiladores externos.

4. **Regenerar el código C11 nativo (`src-c/`):**
   - Ejecutar `compilar_recomp_aot.ps1` para emitir los nuevos bloques C11 con sus correspondientes guardas FNV-1a de 64 bits (`a->guard(m, addr, len, hash)`), cubriendo el 100 % de los casos de combate.

5. **Pruebas de regresión y robustez (`test-generation`):**
   - Añadir en `src/recomp_tests.rs` un test unitario que verifique el despacho indexado O(1) de las subrutinas de combate en IWRAM.
   - Añadir en `tests/packaging_tests.rs` un test de integración que valide que todas las subrutinas de combate están catalogadas en `blazing.labels.toml` y auditadas sin errores.

6. **Sincronización documental (`diataxis-docs`):**
   - Actualizar [`docs/PENDIENTES.md`](../PENDIENTES.md) marcando la tarea como resuelta.
   - Actualizar [`docs/guias/mapear-labels-aot.md`](../guias/mapear-labels-aot.md) documentando la catalogación del clúster de combate en IWRAM.

---

## 3. Alcance Delimitado

### Qué incluye
- Edición de [`blazing.labels.toml`](../../blazing.labels.toml) para agregar las 14 etiquetas de combate.
- Ajuste del rango de validación de IWRAM en `scripts/diagnostico/auditar_labels.py`.
- Integración de `$env:GBA_RECOMP_TCC_LIB` en `scripts/diagnostico/compilar_recomp_aot.ps1`.
- Regeneración de las unidades C11 en `src-c/` mediante `recomp build`.
- Nuevos tests en `src/recomp_tests.rs` y `tests/packaging_tests.rs`.
- Actualización de `docs/PENDIENTES.md` y `docs/guias/mapear-labels-aot.md`.

### Qué NO incluye
- No modifica la capa de audio nativo (AAudio / Oboe) ni el mezclador MP2K.
- No altera el pipeline gráfico `NativePpu` ni shaders WGSL.
- No modifica el almacenamiento ni la persistencia de partidas (`.sav`).
- No introduce rutas locales absolutas en ningún archivo.

---

## 4. Criterios de Aceptación (Dado / Cuando / Entonces)

### Escenario 1: Auditoría de etiquetas de combate en IWRAM
- **Dado** el archivo `blazing.labels.toml` con las 14 nuevas funciones de combate en modo ARM en el rango `0x03003478`–`0x03003708`,
- **Cuando** se ejecuta `python scripts/diagnostico/auditar_labels.py`,
- **Entonces** el script reporta 18.497 funciones declaradas (37 ARM, 18.460 Thumb), 0 errores de unicidad y 0 advertencias de rango.

### Escenario 2: Despacho O(1) nativo en `BlockTable`
- **Dado** un arreglo de bloques `RcgBlock` que incluye las claves de combate en IWRAM,
- **Cuando** se construye la tabla `BlockTable::from_blocks`,
- **Entonces** `table.get(0x030035C8)` y `table.get(0x030035CC)` resuelven en `Some(func)` en el arreglo indexado `iwram`, sin caer al hash map `other` ni al intérprete.

### Escenario 3: Bloques nativos protegidos con guarda en C11
- **Dado** el código generado en `src-c/`,
- **Cuando** se inspecciona la unidad C correspondiente a IWRAM (`1575 - Fire Emblem (E)(Independent)-bios.0.c`),
- **Entonces** existen funciones compiladas `b_030035c8_a`, `b_030035cc_a`, `b_030036bc_a` y `b_030036cc_a` con llamadas a `a->guard(m, addr, len, hash)`.

### Escenario 4: Suite de pruebas 100% verde
- **Dado** el repositorio tras aplicar los cambios y recompilar las unidades,
- **Cuando** se ejecuta `cargo test`,
- **Entonces** los 257 tests existentes más los nuevos tests pasan en verde (259+ tests, 0 fallos).

---

## 5. Verificación Prevista

Comandos de validación:
```powershell
python scripts/diagnostico/auditar_labels.py
powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1
cargo test
python scripts/actualizar_indice_propuestas.py --verificar
```

---

## 6. Predicho vs. Real

| Aspecto | Predicho en propuesta | Resultado real obtenido | Veredicto |
|---|---|---|---|
| Etiquetas catalogadas | 18.497 funciones (37 ARM, 18.460 Thumb) | 18.497 funciones (37 ARM, 18.460 Thumb) auditadas con 0 errores | Exacto |
| Límite superior en `auditar_labels.py` | Permitir direcciones en rango completo de IWRAM hasta `0x03008000` | Actualizado `< 0x03008000`; 0 avisos ni colisiones | Exacto |
| TinyCC en `compilar_recomp_aot.ps1` | `$env:GBA_RECOMP_TCC_LIB` configurado si existe `tcc` en gba-recomp hermano | Detectado y activado automáticamente; recompilación en 10 s | Exacto |
| Bloques C11 emitidos en `src-c/` | `b_030035c8_a`, `b_030035cc_a`, `b_030036bc_a`, `b_030036cc_a` con guardas | 15/15 direcciones presentes en `src-c/` con `a->guard(m, addr, len, hash)` | Exacto |
| Despacho O(1) en `BlockTable` | Resolver en el arreglo indexado `iwram` | `test_block_table_despacha_rutinas_de_combate_en_iwram` pasa en verde | Exacto |
| Suite de pruebas de regresión | 259+ tests en verde | 259 tests (173 unitarios + 86 integración) 100 % en verde | Exacto |

---

## 7. Fuera de Alcance (Hallazgos que no se arreglaron)

- **Subrutinas de sonido MP2K en IWRAM (`0x030025E8`–`0x030027CC`)**: Aunque el volcado de memoria viva capturó los 128 puntos de entrada en IWRAM del reproductor de sonido, el backend de audio nativo de Android (AAudio/Oboe) corresponde a la Fase 2 del roadmap (impacto: bajo, no afecta la ejecución de batalla ni genera desincronización de video).
- **Rango auxiliar de combate (`0x03002B80`–`0x03003200`)**: Corresponde a buffers de batalla y estructuras de datos mutables en RAM, no a subrutinas ejecutables que requieran etiquetas AOT (impacto: nulo).
