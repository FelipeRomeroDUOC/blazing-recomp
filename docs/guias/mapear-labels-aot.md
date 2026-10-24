---
tipo: guia-practica
verificado: si
actualizado: 2026-10-24
---

# Cómo inspeccionar, mapear y recompilar funciones AOT

Esta guía práctica explica cómo identificar subrutinas de *Fire Emblem: The Blazing Blade* mediante las herramientas de desensamblado, registrar nuevas funciones en el archivo de símbolos `blazing.labels.toml` y compilar el código C11 nativo correspondiente.

---

## Requisitos

Antes de iniciar el proceso de ingeniería inversa y mapeo, asegúrate de contar con:
1. **ROM oficial de referencia:** `Fire Emblem (Europe)` con SHA-256 `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`.
2. **Entorno Python 3.9+** con acceso a los scripts en `scripts/diagnostico/`.
3. **Recompilador `recomp`** del motor upstream [`JRickey/gba-recomp`](https://github.com/JRickey/gba-recomp), necesario para reconstruir el código C11 tras modificar etiquetas:
   - **Compilación desde fuentes:**
     ```bash
     git clone https://github.com/JRickey/gba-recomp.git
     cd gba-recomp
     cargo build --release -p recomp
     ```
     El ejecutable generado se ubica en `target/release/recomp` (o `recomp.exe` en Windows). Puedes añadirlo al `PATH` del sistema o situar el repositorio clonado como carpeta hermana adyacente (`../gba-recomp`).
   - **Binarios precompilados:** Opcionalmente, descargar el binario oficial desde [`JRickey/gba-recomp/releases`](https://github.com/JRickey/gba-recomp/releases).

---

## Pasos

### 1. Identificar la dirección de la rutina o punto de salto
Cuando un informe de telemetría (`analizar_telemetria.py`) reporta que una dirección no resuelta está cayendo al intérprete (por ejemplo, `0x0801a2c4`), esa dirección constituye un candidato prioritario para el mapeo AOT.

### 2. Localizar el prólogo canónico de la función
En la arquitectura ARMv4T de GBA, las funciones compiladas en modo Thumb suelen iniciar con la instrucción `PUSH {..., lr}` (`0xb5xx`). Utiliza el script de búsqueda hacia atrás para encontrar el punto de entrada real de la función:

```bash
python scripts/diagnostico/desensamblar_rom.py --buscar-prologo 0x0801a2c4
```

El script imprimirá la dirección inicial encontrada (por ejemplo, `0x0801a2b0`) y el desensamblado de las primeras instrucciones.

### 3. Inspeccionar el flujo de control de la rutina
Comprueba cuántas instrucciones abarca el bloque y si contiene retornos normales (`POP {..., pc}` o `BX LR`):

```bash
python scripts/diagnostico/desensamblar_rom.py --addr 0x0801a2b0 --count 20
```

Si la rutina contiene saltos indexados (`add pc, rx`), consulta la especificación técnica en [`docs/fuentes/Manual Técnico de Arquitectura Interna, Procesamiento Gráfico, Motor de Sonido y Emulación de la Game Boy Advance.md`](../fuentes/Manual%20Técnico%20de%20Arquitectura%20Interna,%20Procesamiento%20Gráfico,%20Motor%20de%20Sonido%20y%20Emulación%20de%20la%20Game%20Boy%20Advance.md) sobre *jump tables*.

### 4. Caso especial: Subrutinas dinámicas en IWRAM (`0x03xxxxxx`)
En la Game Boy Advance, la memoria IWRAM (`0x03000000`–`0x03007FFF`, 32 KB) es RAM interna ultrarrápida (bus de 32 bits a 1 ciclo de reloj sin estados de espera). El motor de *Fire Emblem 7* ejecuta allí sus rutinas más críticas de tiempo:
- **Despacho de interrupciones:** La subrutina ejecutable `IntrMain` se copia en `0x030028A0`. La palabra de memoria en `0x03007FFC` aloja el **vector de interrupción de usuario** (`User IRQ Handler`) que la BIOS consulta en `0x00000018` para saltar a `IntrMain` (es un puntero de 4 bytes, no una función).
- **Cálculo de combate y fórmulas de daño:** El clúster crítico de subrutinas de ataque, cálculo de evasión y daño reside en `0x030035C8`–`0x030036CC` (identificado en la telemetría como el principal hotspot del intérprete). El rango auxiliar `0x03002B80`–`0x03003200` aloja rutinas de apoyo y buffers de batalla.
- **Mezcla de audio:** Rutinas del reproductor Music Player 2000 (M4A/MP2K).

**Particularidad de IWRAM:** Estas funciones **no residen de forma estática en esa dirección en el archivo de la ROM**. Durante el arranque (`crt0.s`) o antes de iniciar las secuencias de combate, el juego copia bloques de código binario desde direcciones maestras en la ROM (`0x08xxxxxx`) hacia la memoria IWRAM mediante transferencias DMA3 o bucles directos `memcpy`.

**Estado actual en el repositorio:** En la versión actual de [`blazing.labels.toml`](../../blazing.labels.toml), la totalidad de las 18.483 funciones catalogadas residen en el espacio de ROM (`0x08000000`–`0x08FFFFFF`); **ninguna subrutina de IWRAM está catalogada todavía**. El generador de código estático upstream (`recomp build`) exige un volcado de memoria viva (*blob* binario de 32 KB capturado con el estado de la RAM inicializada) para generar las funciones de guarda (`guard`) con hash FNV-1a que certifiquen la integridad del código en RAM antes de saltar al bloque C11. 

**Procedimiento operativo para capturar el blob de IWRAM (`iwram_dump.bin`):**
1. Ejecutar el arnés determinista de replay hasta la escena de combate:
   ```bash
   cargo run --example ppu_barrido_nativa -- --replay secuencias/combate.txt
   ```
2. Volcar los 32 KB del bus de memoria (`0x03000000` a `0x03007FFF`) a un archivo `iwram_dump.bin`.
3. Suministrar el volcado a `recomp build` para que calcule los hashes de integridad FNV-1a de 64 bits y emita los bloques protegidos por `a->guard(m, addr, len, hash)`.
4. Mapear las subrutinas de combate (`0x030035C8`, `0x030036CC`, etc.) en `blazing.labels.toml`.

Esta tarea técnica se encuentra registrada en [`docs/PENDIENTES.md`](../PENDIENTES.md). Para mapear funciones normales en ROM, continúa directamente con el Paso 5.

### 5. Registrar la función en `blazing.labels.toml`
Abre el archivo de etiquetas [`blazing.labels.toml`](../../blazing.labels.toml) y añade la nueva entrada al final:

```toml
[[functions]]
name = "Subrutina_Combate_0801A2B0"
address = 0x0801A2B0
mode = "thumb"
```

*Asegúrate de que la dirección sea un número par (`address & 1 == 0`) y que el modo sea `"thumb"` (16 bits) o `"arm"` (32 bits).*

### 6. Auditar la integridad del archivo de labels
Ejecuta el auditor estático para comprobar que la nueva etiqueta no colisione con funciones existentes y resida en los límites válidos de la memoria:

```bash
python scripts/diagnostico/auditar_labels.py
```

### 7. Regenerar el código C11 nativo
Una vez validada la etiqueta, recompila el código estático intermedio de C11:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1
```

El script actualizará los archivos correspondientes en `src-c/`, permitiendo que la subrutina se compile directamente a lenguaje de máquina nativo (ARM64 en Android) en la siguiente construcción.

---

## Comprobar el resultado

Para confirmar que la función ha sido incorporada al camino nativo AOT y ya no se ejecuta mediante el intérprete:

1. **Verificar el código C generado:**
   Revisa los archivos en `src-c/` buscando el identificador asignado (`Subrutina_Combate_0801A2B0`). Debe existir una función nativa en C11 generada automáticamente con la traducción de las instrucciones Thumb.

2. **Ejecutar la suite de pruebas del proyecto:**
   ```bash
   cargo test
   ```
   Todos los tests deben mantenerse en verde.

3. **Verificar la telemetría en ejecución:**
   Ejecuta una partida en el dispositivo pasando por la escena correspondiente, extrae el archivo `blazing-perf.txt` y corre:
   ```bash
   python scripts/diagnostico/analizar_telemetria.py --archivo blazing-perf.txt
   ```
   Comprueba que la dirección mapeada haya desaparecido del ranking de hotspots del intérprete de respaldo y que el porcentaje de instrucciones nativas se aproxime al 100%.
