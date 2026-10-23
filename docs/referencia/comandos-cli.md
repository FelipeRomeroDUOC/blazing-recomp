---
tipo: referencia
verificado: si
---

# Especificación Técnica: Comandos CLI y Herramientas de Diagnóstico

## Descripción general

Esta especificación describe los comandos de línea de órdenes, scripts oficiales y utilidades de ingeniería inversa y empaquetado del proyecto `blazing-recomp`.

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
   - Incorpora las bibliotecas compartidas `libblazing_recomp.so` en las rutas `lib/<abi>/`.
   - Aplica alineación estricta de memoria a **16 KB (16,384 bytes)** para compatibilidad con Android 15 y Android 16.
4. **Firma Criptográfica con APKSigner:**
   - Aplica firma digital híbrida bajo los esquemas **v1 (JAR), v2 (APK Signature Scheme) y v3**.

#### Salida Generada
- Archivo: `android/build/blazing-recomp.apk`
- Código de salida: `0` si el empaquetado y la verificación de firma son exitosos.

---

## 2. Suite de Diagnóstico e Inspección AOT (`scripts/diagnostico/`)

### `scripts/diagnostico/analizar_telemetria.py`
Analiza registros de ejecución (`logcat` o volcados de texto del runtime) para diagnosticar caídas de cuadros y cuellos de botella por fases.

#### Sintaxis y Parámetros
```bash
python scripts/diagnostico/analizar_telemetria.py --archivo <ruta_log> [--top <N>]
```

| Parámetro | Tipo | Por defecto | Descripción |
| :--- | :---: | :---: | :--- |
| `--archivo` | Path | *(Obligatorio)* | Ruta al archivo de texto con la telemetría generada por el runtime. |
| `--top` | Entero | `10` | Cantidad de funciones del intérprete a listar en el ranking de hotspots. |

#### Métricas Emitidas
- Tasa media de fotogramas por segundo (FPS) y tiempo medio por fotograma.
- Desglose temporal en milisegundos por fases: `ejec` (CPU/AOT), `ppu` (rasterizado de líneas), `apu` (audio), `dibujo` (shaders de renderizado) y `present` (swapchain / VSync).
- Porcentaje de ejecución derivado al intérprete de respaldo frente a código AOT C11.

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
1. **Cabeceras de Frontmatter:** Verifica la presencia de `tipo: <tutorial | guia | referencia | explicacion>` en las páginas formales de `docs/`.
2. **Integridad de Enlaces:** Comprueba que todos los enlaces relativos apunten a archivos y anclas (`#seccion`) existentes en disco.
3. **Marcas Incompletas:** Detecta cadenas pendientes como marcas de revisión o etiquetas incompletas.
- Código de salida: `0` si no hay errores ni avisos.
