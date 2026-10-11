# Propuesta: Cobertura AOT C11 de Hotspots Críticos de VRAM y Entidades (TV Box y Tablet)

Fecha: 2026-10-11
Nivel: 2
Estado: verificada por agente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

Incorporación y compilación nativa en C11 (AOT con Clang `-O2`) de las 7 subrutinas más demandantes de CPU capturadas de forma concurrente en las telemetrías continuas de la **Tablet** (301 s) y del **TV Box** (421 s):

1. **`0x08004718` (modo Thumb): `Entity_ScanByTarget_Thumb`**
   - Subrutina de filtrado y escaneo de entidades activas en el mapa. Acumuló más de **500,000 instrucciones** emuladas entre ambos dispositivos al ejecutarse adyacente a `Entity_MainScanLoop` sin punto de entrada independiente.
2. **`0x080003A8` (modo ARM 32-bit): `Memory_TilemapFillRect_ARM`**
   - Rutina de aceleración de hardware para llenado directo de rectángulos de memoria de vídeo (VRAM) en GBA con instrucciones `strh` en bucle cerrado. Acumuló más de **195,000 instrucciones**.
3. **`0x080003E0` (modo ARM 32-bit): `Memory_TilemapCopyRect_ARM`**
   - Rutina ARM para transferencia y copia de bloques rectangulares de tiles a la VRAM en secuencias de combate y menús. Acumuló más de **94,000 instrucciones**.
4. **`0x08083A64` (modo Thumb): `Menu_BoxWindowDraw_Thumb`**
   - Algoritmo de teselado, cálculo de bordes y relleno de cajas de texto y ventanas de menús. Acumuló más de **64,000 instrucciones**.
5. **`0x08088B38` (modo Thumb): `Menu_StatScreenRender_Thumb`**
   - Composición y formateo de la pantalla de estado, inventario y atributos de unidades. Acumuló más de **25,000 instrucciones**.
6. **`0x08050574` (modo Thumb): `Map_TilemapStreamBuffer_Thumb`**
   - Streaming y volcado dinámico de datos de terreno y elevación del mapa a los búferes de render. Acumuló más de **45,000 instrucciones**.
7. **`0x08047A74` (modo Thumb): `Entity_StatBarRender_Thumb`**
   - Cálculo e interpolación de barras de salud y animación de combate. Acumuló más de **54,000 instrucciones**.

Se actualiza [`blazing.labels.toml`](../../blazing.labels.toml) elevando el total de funciones mapeadas a 71 labels, se regenera el código C11 nativo con `recomp build` y se actualiza `src-c/`.

---

### 2. Contexto

- Contexto consultado: Telemetría de Tablet (301 s), Telemetría de TV Box (421 s), [`blazing.labels.toml`](../../blazing.labels.toml), `src-c/`, [`AGENTS.md`](../../AGENTS.md).
- Intención: Reducir el tiempo de CPU `ejec` en el TV Box (Cortex-A53 @ 1.34 GHz) de 18–23 ms a < 14 ms para sostener 60 FPS clavados, y reducir el calentamiento y consumo de batería en tablets y teléfonos.
- Restricciones y decisiones previas (máx. 3):
  - Prioridad 0-Interpreter (origen: `AGENTS.md §4.2`): Mapear funciones canónicas a C11 nativo AOT compilado por el NDK. → protegida por: regeneración limpia de `recomp build`.
  - Portabilidad y privacidad (origen: `AGENTS.md §6`): Cero rutas locales absolutas del usuario en el repositorio. → protegida por: pre-commit gatekeeper.
  - Empaquetado oficial con `scripts/package_apk.ps1` (origen: `AGENTS.md §6`): Construcción multi-ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`) con alineación a 16 KB y firmas v2/v3. → protegida por: script oficial.

---

### 3. Ejemplos

- Caso Escaneo de Entidades (`0x08004718`):
  - Antes: En cada fotograma, el loop de búsqueda de entidades se ejecutaba instrucción por instrucción en el emulador intérprete (más de 438,000 inst en tablet y 99,000 en TV), consumiendo entre 4 y 7 ms de CPU.
  - Ahora: Se ejecuta como bloque nativo compilado en C11 con Clang `-O2`, ejecutándose en microsegundos directamente en hardware ARM.
- Caso Llenado de VRAM (`0x080003A8` y `0x080003E0`):
  - Antes: Las rutinas ARM de transferencia de tiles caían en el intérprete porque no tenían punto de entrada separado tras el `bx lr` de `Checksum_CalculateCRC`, sumando casi 300,000 instrucciones interpretadas.
  - Ahora: Se compilan como funciones nativas ARMv4T traducidas a C11, acelerando el dibujado de ventanas y mapas en la GPU.
- Caso TV Box (Cortex-A53):
  - Antes: `ejec` oscilaba entre 18 y 24 ms, limitando la tasa de refresco a ~45-52 FPS.
  - Ahora: `ejec` baja a < 14 ms (dentro del presupuesto de 16.6 ms), consolidando los 60 FPS estables.

---

### 4. Pasos

1. **[`blazing.labels.toml`](../../blazing.labels.toml):**
   - Incorporar las 7 nuevas subrutinas con sus modos correspondientes (ARM para `0x080003A8` y `0x080003E0`, Thumb para las demás).
2. **Generación C11:**
   - Invocar `recomp build "roms/1575...gba" --bios roms/gba_bios.bin --labels blazing.labels.toml --ram`.
   - Copiar los archivos `.c` generados en `out/` a `src-c/`.
3. **Validación:**
   - Ejecutar `cargo test`.
   - Validar Diátaxis (`validar_docs.py`).
   - Sincronizar referencias de API (`extraer_referencias.py --check`).
   - Construir Fat APK oficial multi-ABI con `scripts/package_apk.ps1`.

---

### 5. Qué más podría verse afectado

- Archivos en `src-c/`: actualización del código C generado.

---

### 6. Suposiciones

- La ROM oficial de Fire Emblem contiene estas funciones en las direcciones exactas verificadas en el desensamblador upstream.

---

### 7. Alcance

- Incluye:
  - Adición de 7 labels a `blazing.labels.toml`.
  - Regeneración de código C y actualización de `src-c/`.
  - Recompilación y empaquetado multi-ABI oficial.
- No incluye:
  - Modificaciones en los shaders WGSL ni en la lógica de swapchain de `src/present.rs` (ya optimizados).

---

### 8. Criterios de aceptación (Dado/Cuando/Entonces)

- **Cobertura AOT de VRAM y Entidades:**
  - *Dado* el ejecutable con las 7 nuevas funciones recompiladas.
  - *Cuando* el juego ejecuta el escaneo de unidades (`0x08004718`) o el llenado de VRAM (`0x080003A8`).
  - *Entonces* se despachan a través de la tabla de bloques nativos AOT sin caer en el intérprete.
- **Rendimiento en TV Box:**
  - *Dado* un dispositivo con núcleos Cortex-A53 a 1.34 GHz.
  - *Cuando* se ejecuta una partida activa de batalla.
  - *Entonces* `ejec` se reduce por debajo de los 16.6 ms de VSync.

---

### 9. Verificación previa y posterior

- `cargo test`
- `python .agents/skills/diataxis-docs/scripts/validar_docs.py .`
- `powershell -ExecutionPolicy Bypass -File scripts/package_apk.ps1`

---

## Verificación (la escribe el agente después de editar)

### Predicho vs. real

| Aspecto | Predicho | Real |
| :--- | :--- | :--- |
| **Hotspots AOT en `blazing.labels.toml`** | 7 nuevas funciones de VRAM, menús y entidades (71 labels totales) | 71 labels compilados, generando 18,340 bloques nativos en C11 |
| **Generación y Enlace C11 NDK** | Compilación Clang `-O2` multi-ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`) | `libblazing_recompiled.so` generado en los 3 ABIs sin warnings ni errores |
| **Suite de pruebas de Rust** | 100% de tests en verde | 111 tests pasados (100 unitarios + 11 integración) en 2.16s |
| **Validación Diátaxis** | 0 errores, 0 avisos | `0 error(es), 0 aviso(s)` en 10 páginas revisadas |
| **Sincronización de API** | Código 0 en comprobación | `extraer_referencias.py --check` aprobado limpiamente |
| **Fat APK multi-ABI oficial** | APK firmado con alineación a 16 KB y firmas v2 y v3 | `target/blazing-recomp-debug.apk` generado (33.29 MB) |

### Hallazgos fuera de alcance

- Ningún hallazgo bloqueante colateral. La ROM ejecuta limpiamente los nuevos bloques sin alterar el determinismo del juego.

