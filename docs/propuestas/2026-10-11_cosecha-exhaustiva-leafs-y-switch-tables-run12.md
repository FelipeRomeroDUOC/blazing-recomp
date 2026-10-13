# Propuesta: Cosecha Exhaustiva de Leaf Functions y Jump Tables para Erradicar el 96% de Ejecución en Intérprete (Run 12)

**Fecha:** 2026-10-11  
**Autor:** Agente IA (Antigravity)  
**Nivel de cambio:** Nivel 2 (Herramientas de diagnóstico, expansión de labels de ROM, regeneración AOT C11 y empaquetado multi-ABI)  
**Estado:** Implementada y Verificada  

---

## 1. Contexto y Diagnóstico Forense (Run 12)

Durante la sesión de telemetría de **Run 12** (690 segundos de juego real en Android TV Box), se descubrió que el cuello de botella que frena el juego a 20–45 FPS no reside en la GPU (5.78 ms) ni en los sprites (1.48 ms), sino en la **CPU de emulación (`ejec = 19.75 ms`, con picos de 38.30 ms)**:
- **Instrucciones en Intérprete:** 16,753,494 (96.0% del tiempo de CPU).
- **Instrucciones Nativas AOT:** 690,198 (4.0% del tiempo de CPU).

El ranking de hotspots sobre las 16.7 millones de instrucciones desveló que **tan solo 4 patrones de subrutinas concentran más del 96% de todos los ticks del intérprete**:

1. **Top 1 (`0x080BE900`): 74.66% de todos los ticks del intérprete**
   - Subrutina hoja (*leaf function*) canónica que abarca de `0x080BE900` a `0x080BE92E` y retorna con `bx lr` (longitud de 46 bytes).
   - Su puntero oficial reside en la tabla de ROM en `0x08658E98` (`0x080BE901`).
   - **Causa raíz de omisión previa:** El cosechador anterior utilizaba un bucle `range(0, 32, 2)` (16 instrucciones / 32 bytes) para detectar el retorno. Al medir 46 bytes, el cosechador la descartó asumiendo erróneamente que era un bloque de datos.

2. **Top 2 (`0x080841A0`): 10.24% de todos los ticks del intérprete**
   - Ramificación interna del `switch` de 128 casos en `0x08083F12` (`cmp r1, #128`).
   - Los casos 18, 19 y 20 saltan a `0x080841C8`, que enlaza con `0x080841A0`.
   - **Causa raíz de omisión previa:** El límite defensivo `count > 128` truncó la extracción a 32 casos ante comparaciones de 128 casos (`(0x80) + 1 = 129 > 128`).

3. **Top 3 (`0x0800A500` / `0x0800A514`): 7.20% de todos los ticks del intérprete**
   - Jump table de 37 casos en `0x0800A514` con puntero base indirecto `ldr r1, [pc, #8] -> .word 0x0800A51C`.

4. **Top 4 (`0x080AA9AC`): 4.50% de todos los ticks del intérprete**
   - Función leaf de 60 bytes que retorna con `bx lr`, igualmente descartada por el umbral de 32 bytes.

### Validación Empírica
Al ampliar la ventana de escaneo de `32` a `256` bytes y elevar el umbral de jump tables a `256` casos:
- Los punteros de función válidos se incrementan de **6,496 a 13,481** (+6,985 funciones leaf rescatadas).
- `0x080BE900`, `0x080AA9AC`, `0x080841A0` y `0x080841C8` quedan plenamente identificadas.
- La simulación sobre el log de 690s confirma que **se cubre el 98.44% de todos los ticks del intérprete**.

---

## 2. Alcance de la Propuesta

### Incluye:
1. **Actualización de `scripts/diagnostico/cosechar_jump_tables_y_leafs.py`:**
   - Ampliar la ventana de escaneo de retorno de leaf functions de `range(0, 32, 2)` a `range(0, 256, 2)`.
   - Ajustar el límite de casos en `cosechar_jump_tables` a `count > 256` para admitir switch tables de 128 casos sin truncamiento.
   - Mapear explícitamente las subrutinas críticas de bucle y hotspots detectadas en Run 12 (`0x080BE900`, `0x080AA9AC`, `0x080841A0`, `0x0800A502`).
2. **Cosecha y Enriquecimiento de `blazing.labels.toml`:**
   - Ejecutar la cosecha e integrar las nuevas labels sin duplicados ni colisiones.
   - Auditar integridad estricta con `python scripts/diagnostico/auditar_labels.py`.
3. **Recompilación AOT C11:**
   - Ejecutar `scripts/diagnostico/compilar_recomp_aot.ps1` (`RECOMP_KEEP_C=1`) para generar las unidades C11 actualizadas en `src-c/`.
4. **Verificación de Tests:**
   - Ejecutar `cargo test` (113 tests deben pasar en verde).
5. **Empaquetado Oficial Multi-ABI del APK:**
   - Ejecutar `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1` para generar el instalador final para Android TV Box.

### No incluye:
- No se modifica el código del motor Rust en `src/` (el límite de bloques en `src/recomp.rs` ya admite hasta 500,000 bloques).
- No se tocan shaders ni touch overlay.

---

## 3. Criterios de Aceptación (Dado / Cuando / Entonces)

### Criterio 1: Cosecha de Hotspots Críticos
- **Dado** que `0x080BE900`, `0x080AA9AC` y `0x080841A0` concentran el 92% de los ticks del intérprete,
- **Cuando** se ejecuta `python scripts/diagnostico/cosechar_jump_tables_y_leafs.py`,
- **Entonces** las 3 funciones deben quedar extraídas y etiquetadas en `blazing.labels.toml`.

### Criterio 2: Auditoría Estricta de Labels
- **Dado** el nuevo archivo `blazing.labels.toml` con ~18,400 labels,
- **Cuando** se ejecuta `python scripts/diagnostico/auditar_labels.py`,
- **Entonces** debe finalizar con código `0` confirmando cero colisiones y cero duplicados.

### Criterio 3: Compilación AOT C11 y Empaquetado APK
- **Dado** el nuevo mapa de labels auditado,
- **Cuando** se compila con `compilar_recomp_aot.ps1` y `package_apk.ps1`,
- **Entonces** las nuevas funciones deben compilarse como código C11 nativo y empaquetarse en el Fat APK multi-ABI.

### Criterio 4: Integridad de la Suite de Tests
- **Dado** el repositorio tras la cosecha,
- **Cuando** se ejecuta `cargo test`,
- **Entonces** los 113 tests de la suite deben pasar en verde al 100%.

---

## 4. Comandos de Verificación Previstos

```powershell
# 1. Cosechar subrutinas y jump tables ampliadas
python scripts/diagnostico/cosechar_jump_tables_y_leafs.py

# 2. Auditar integridad de labels
python scripts/diagnostico/auditar_labels.py

# 3. Compilar código nativo C11 con recomp
powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1

# 4. Validar suite de tests completa
cargo test

# 5. Generar APK oficial para Android TV Box
powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1
```

---

## 5. Resultados de Verificación (Predicho vs. Real)

| Métrica / Verificación | Predicho en Propuesta | Resultado Real Obtenido | Estado |
| :--- | :--- | :--- | :---: |
| **Cosecha Jump Tables** | Ampliar límite a 256 casos | **1,117 destinos** de Jump Table extraídos | Conforme |
| **Punteros y Leaf Functions** | Escaneo de retorno hasta 256 bytes | **13,481 funciones** con retorno localizadas (+6,985) | Conforme |
| **Hotspots Críticos Telemetría** | Mapear `0x080BE900`, `0x080841A0`, etc. | **4 hotspots** mapeados e integrados en `blazing.labels.toml` | Conforme |
| **Total Labels AOT** | ~18,400 labels | **18,483 labels** (+6,946 nuevas respecto a 11,537) | Conforme |
| **Auditoría de Labels** | 0 colisiones / duplicados | `auditar_labels.py`: 0 duplicados, código de salida 0 | Conforme |
| **Bloques C11 Traducidos** | Incrementar sobre 103k bloques | **144,732 bloques** (+40,901) / 1,121,473 instrucciones | Conforme |
| **Unidades C11 en `src-c/`** | 10 a 12 unidades | **12 unidades** (`*-bios.0.c` a `*-bios.11.c`) | Conforme |
| **Suite de Tests Rust** | 100% verde | **113 tests pasados** (102 unitarios + 11 packaging) | Conforme |
| **Fat APK Multi-ABI** | Compilado, firmado y alineado a 16 KB | **186.61 MB** en `target/blazing-recomp-debug.apk` | Conforme |

### Fuera de alcance (hallazgos que no se tocaron)
- Ninguno. Todas las operaciones se mantuvieron estrictamente acotadas al cosechador de labels, la auditoría, la compilación AOT y el empaquetado del APK.
