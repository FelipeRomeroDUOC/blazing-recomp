---
tipo: referencia
verificado: si
actualizado: 2026-10-24
---

# Especificación Técnica: Comandos CLI y Herramientas de Diagnóstico

## Descripción general

Esta especificación describe los comandos de línea de órdenes, scripts oficiales y utilidades de ingeniería inversa y empaquetado del proyecto `blazing-recomp`.

### Catálogo Completo de Scripts del Repositorio

| Script / Herramienta | Lenguaje | Ubicación | Propósito y Uso |
| :--- | :---: | :--- | :--- |
| `package_apk.ps1` | PowerShell | `scripts/` | Empaqueta, alinea (16 KB) y firma el Fat APK multi-ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`) con `cargo-ndk`, `aapt2`, `zipalign` y `apksigner`. |
| `extraer_referencias.py` | Python | `scripts/` | Extrae la API pública de Rust a [`docs/referencia/api-modulos-rust.md`](api-modulos-rust.md). Con `--check` valida sincronización como pre-commit gatekeeper. |
| `auditar_hallazgos.py` | Python | `scripts/` | Audita la trazabilidad de hallazgos registrados en propuestas frente a [`docs/PENDIENTES.md`](../PENDIENTES.md) y [`docs/BACKLOG.md`](../BACKLOG.md). |
| `actualizar_indice_propuestas.py` | Python | `scripts/` | Reconstruye automáticamente el índice de propuestas en [`docs/BACKLOG.md#índice-de-propuestas`](../BACKLOG.md) y la lista de revisión humana a partir de las fichas de `docs/propuestas/`. |
| `auditar_mutantes.py` | Python | `scripts/` | Ejecuta y audita tests de mutación generales en la suite de pruebas del proyecto. |
| `analizar_telemetria.py` | Python | `scripts/diagnostico/` | Analiza volcados y logs de ejecución, calculando tasas de FPS, desglose por fases (`ejec`, `ppu`, `apu`, `dibujo`, `present`) y ranking de hotspots del intérprete. |
| `desensamblar_rom.py` | Python | `scripts/diagnostico/` | Desensamblador estático Thumb/ARM sobre la ROM de GBA. Decodifica opcodes y localiza prólogos canónicos de funciones (`PUSH {..., lr}`). |
| `auditar_labels.py` | Python | `scripts/diagnostico/` | Valida la integridad estructural de [`blazing.labels.toml`](../../blazing.labels.toml) (unicidad, alineación par y límites de memoria ROM e IWRAM). |
| `compilar_recomp_aot.ps1` | PowerShell | `scripts/diagnostico/` | Invoca `recomp build` con `$env:RECOMP_KEEP_C = "1"` y sincroniza las fuentes C11 generadas en [`src-c/`](../../src-c/). |
| `compilar_c_paralelo.py` | Python | `scripts/diagnostico/` | Orquestador de compilación en paralelo de las 12 unidades de traducción C11 a bibliotecas de objetos. |
| `cosechar_jump_tables_y_leafs.py` | Python | `scripts/diagnostico/` | Escanea la ROM en busca de saltos dinámicos (`mov pc, rx`) y funciones hoja para generar nuevas entradas de labels. |
| `sembrar_labels_masivo.py` | Python | `scripts/diagnostico/` | Inserta etiquetas cosechadas por lotes masivos directamente en [`blazing.labels.toml`](../../blazing.labels.toml). |
| `fotogramas_dorados.py` | Python | `scripts/diagnostico/` | Captura y compara fotogramas del juego contra imágenes de referencia (*golden frames*) para detectar regresiones gráficas. |
| `contrastar_arnes.py` | Python | `scripts/diagnostico/` | Compara el comportamiento y trazas del arnés de pruebas contra la referencia del hardware. |
| `convertir_arnes.py` | Python | `scripts/diagnostico/` | Convierte registros de trazas de ejecución en formatos consumibles por el arnés de verificación. |
| `mutar_arnes.py` | Python | `scripts/diagnostico/` | Inyecta mutantes en el arnés de pruebas para certificar la muerte de mutantes por la suite de tests. |
| `mutar_ppu_nativa.py` | Python | `scripts/diagnostico/` | Inyecta mutantes en la lógica de renderizado y shaders de `ppu_nativa.rs` para auditar la cobertura de tests gráficos. |
| `mutar_ppu_nativa_bandera.py` | Python | `scripts/diagnostico/` | Inyecta mutantes en el mecanismo de conmutación de bandera y fallback de la PPU nativa. |
| `validar_docs.py` | Python | `.agents/skills/diataxis-docs/scripts/` | Audita la documentación contra el estándar Diátaxis, integridad de enlaces relativos y ausencia de marcas incompletas. |

---

## 1. Empaquetado de la Aplicación Android

### `scripts/package_apk.ps1`
Genera el paquete instalador final Fat APK multi-arquitectura firmado para Android sin intermediación de Gradle ni Android Studio.

#### Sintaxis de Invocación
```powershell
powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1
```

#### Pipeline de Construcción
1. **Compilación Multi-ABI con Cargo NDK:** Invoca `cargo ndk` sobre las tres arquitecturas soportadas:
   - `aarch64-linux-android` (`arm64-v8a`) — Target principal.
   - `armv7-linux-androideabi` (`armeabi-v7a`) — Compatibilidad con 32 bits.
   - `x86_64-linux-android` (`x86_64`) — Emuladores oficiales de Android.
2. **Procesamiento de Recursos con AAPT2:**
   - Compila el manifiesto `android/AndroidManifest.xml` y los recursos XML/PNG en un paquete binario plano intermedio (`resources.ap_`).
3. **Ensamblado y Alineación con ZipAlign:**
   - Incorpora las bibliotecas compartidas contiguas `libblazing_recomp.so` (runtime Rust) y `libblazing_recompiled.so` (lógica C11 recompilada) en las rutas `lib/<abi>/`.
   - Aplica alineación estricta de memoria a **16 KB (16,384 bytes)** para compatibilidad con Android 15 y Android 16.
4. **Firma Criptográfica con APKSigner:**
   - Aplica firma digital híbrida bajo los esquemas **v1 (JAR), v2 (APK Signature Scheme) y v3**.

#### Salida Generada
- Archivo: `target/blazing-recomp-release.apk` (o `target/blazing-recomp-debug.apk` con `-Debug`)
- Código de salida: `0` si el empaquetado y la verificación de firma son exitosos.

---

## 2. Suite de Diagnóstico e Inspección AOT (`scripts/diagnostico/`)

### `scripts/diagnostico/analizar_telemetria.py`
Analiza registros de ejecución (`blazing-perf.txt`, `logcat` o stdin) para diagnosticar caídas de cuadros, cuellos de botella por fases o comparar el impacto de cambios de rendimiento entre sesiones.

#### Sintaxis y Parámetros
```bash
# Análisis individual
python scripts/diagnostico/analizar_telemetria.py --archivo <ruta_log> [--top <N>]

# Comparación entre dos sesiones (Antes vs. Después)
python scripts/diagnostico/analizar_telemetria.py --comparar <log_base> --archivo <log_nuevo>

# Volcado estructurado en TSV a stdout
python scripts/diagnostico/analizar_telemetria.py --archivo <ruta_log> --volcar
```

| Parámetro | Tipo | Por defecto | Descripción |
| :--- | :---: | :---: | :--- |
| `--archivo`, `-f` | Path | *(Opcional)* | Ruta al archivo de telemetría. Si se omite, lee directamente desde `stdin`. |
| `--comparar`, `-c` | Path | — | Ruta al archivo base para generar un informe comparativo (Antes vs. Después) frente a `--archivo`. |
| `--top`, `-t` | Entero | `20` | Cantidad de funciones del intérprete a listar en el ranking de hotspots. |
| `--volcar` | Flag | `false` | Emite por salida estándar la tabla de valores interpretada en formato TSV para depuración del lector frente al motor. |

#### Métricas Emitidas
- Tasa media de fotogramas por segundo (FPS) y tiempo medio por fotograma.
- Desglose temporal en milisegundos por fases: `ejec` (CPU/AOT), `ppu` (rasterizado por software en CPU), `apu` (audio), `dibujo` (tiempo del compositor GPU) y `present` (swapchain / VSync).
- Porcentaje de ejecución derivado al intérprete de respaldo frente a código AOT C11.
- Identificación de anomalías de formato o líneas inconsistentes con [`telemetria/esquema.txt`](../../telemetria/esquema.txt).

#### Códigos de Salida
- `0`: Análisis, volcado o comparación completados con éxito.
- `1`: Error en parámetros (argumentos incompatibles, archivo inexistente, ausencia de muestras válidas o invocación sin entrada interactiva).

---

### `scripts/diagnostico/desensamblar_rom.py`
Desensamblador estático de la ROM oficial de GBA para inspeccionar instrucciones Thumb/ARM e identificar prólogos de funciones canónicas.

#### Sintaxis y Parámetros
```bash
# Desensamblado contiguo
python scripts/diagnostico/desensamblar_rom.py --addr <0x08xxxxxx> --count <N>

# Búsqueda automática de prólogo
python scripts/diagnostico/desensamblar_rom.py --buscar-prologo <0x08xxxxxx>
```

| Parámetro | Tipo | Descripción |
| :--- | :---: | :--- |
| `--addr` | Hex | Dirección de memoria en ROM de inicio del bloque a decodificar. |
| `--count` | Entero | Cantidad de instrucciones a desensamblar consecutivamente. |
| `--buscar-prologo` | Hex | Escanea hacia atrás desde la dirección indicada hasta hallar un prólogo canónico (`PUSH {..., lr}` / `0xb5xx`). |

---

### `scripts/diagnostico/auditar_labels.py`
Valida la integridad estructural del mapa de símbolos [`blazing.labels.toml`](../../blazing.labels.toml) frente al archivo ROM de referencia.

#### Sintaxis
```bash
python scripts/diagnostico/auditar_labels.py
```

#### Verificaciones Realizadas
1. **Unicidad:** Comprueba que no existan nombres de función duplicados ni colisiones de direcciones.
2. **Alineación:** Exige que todas las direcciones sean números pares (bit 0 en cero).
3. **Rango de Memoria:** Verifica que las direcciones residan estrictamente dentro de los límites físicos de la ROM (`0x08000000`–`0x08FFFFFF`) o IWRAM (`0x03000000`–`0x03007FFF`).
4. **Modos Válidos:** Confirma que el campo `mode` contenga exclusivamente `"arm"` o `"thumb"`.
- Código de salida: `0` en caso de conformidad total; `1` si detecta colisiones o desalineaciones.

---

### `scripts/diagnostico/compilar_recomp_aot.ps1`
Genera el código C11 intermedio a partir del mapa de labels actualizado y sincroniza los archivos en `src-c/`.

#### Sintaxis
```powershell
powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1
```

#### Comportamiento
- Exporta `$env:RECOMP_KEEP_C = "1"` para obligar al compilador a conservar las fuentes en C11.
- Ejecuta `recomp build` contra la ROM oficial.
- Sincroniza los archivos `.c` y `.h` resultantes en la carpeta `src-c/` del repositorio.

---

### Herramientas Upstream Históricas (`--record-labels` y `gba-mapper`)

- **`--record-labels`:** Opción de ejecución de la herramienta upstream `recomp runc` utilizada en fases tempranas para registrar direcciones ejecutadas por el intérprete y exportar candidatas a etiquetas.
- **`gba-mapper`:** Herramienta estática preliminar del ecosistema `gba-recomp`. En este repositorio fue sustituida por las herramientas modulares propias en `scripts/diagnostico/` ([`desensamblar_rom.py`](#scriptsdiagnosticodesensamblar_rompy), [`cosechar_jump_tables_y_leafs.py`](#catálogo-completo-de-scripts-del-repositorio) y [`sembrar_labels_masivo.py`](#catálogo-completo-de-scripts-del-repositorio)), optimizadas para la heurística de prólogos Thumb y jump tables de *Fire Emblem 7*.

---

## 3. Sincronización y Validación del Proyecto

### `scripts/extraer_referencias.py`
Analiza el código fuente en `src/` para generar o comprobar la documentación de la API nativa de Rust.

#### Sintaxis
```bash
# Modo de verificación (no altera archivos)
python scripts/extraer_referencias.py --check

# Modo de regeneración
python scripts/extraer_referencias.py
```

#### Comportamiento
- En modo `--check`, compara la API pública documentada en [`docs/referencia/api-modulos-rust.md`](api-modulos-rust.md) contra el código fuente real. Retorna código `0` si está sincronizada o `1` si hay discrepancias.
- En modo normal, reescribe el archivo con la definición actualizada de structs, enums y funciones.

---

### `.agents/skills/diataxis-docs/scripts/validar_docs.py`
Audita la documentación del repositorio garantizando la conformidad con el estándar Diátaxis y el Pre-Commit Gatekeeper.

#### Sintaxis
```bash
python .agents/skills/diataxis-docs/scripts/validar_docs.py .
```

#### Reglas Auditadas
1. **Cabeceras de Frontmatter:** Verifica la presencia de `tipo: <tutorial | guia-practica | referencia | explicacion>` en las páginas formales de `docs/`.
2. **Integridad de Enlaces:** Comprueba que todos los enlaces relativos apunten a archivos y anclas (`#seccion`) existentes en disco.
3. **Marcas Incompletas:** Detecta cadenas pendientes como marcas de revisión o etiquetas incompletas.
- Código de salida: `0` si no hay errores ni avisos.
