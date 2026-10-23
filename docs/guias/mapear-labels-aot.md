---
tipo: guia-practica
verificado: si
---

# Cómo inspeccionar, mapear y recompilar funciones AOT

Esta guía práctica explica cómo identificar subrutinas de *Fire Emblem: The Blazing Blade* mediante las herramientas de desensamblado, registrar nuevas funciones en el archivo de símbolos `blazing.labels.toml` y compilar el código C11 nativo correspondiente.

---

## Requisitos

Antes de iniciar el proceso de ingeniería inversa y mapeo, asegúrate de contar con:
1. **ROM oficial de referencia:** `Fire Emblem (Europe)` con SHA-256 `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`.
2. **Entorno Python 3.9+** con acceso a los scripts en `scripts/diagnostico/`.
3. **Recompilador `recomp`** instalado o accesible en la ruta del sistema para la reconstrucción de C11.

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

### 4. Registrar la función en `blazing.labels.toml`
Abre el archivo de etiquetas [`blazing.labels.toml`](../../blazing.labels.toml) y añade la nueva entrada al final:

```toml
[[functions]]
name = "Subrutina_Combate_0801A2B0"
address = 0x0801A2B0
mode = "thumb"
```

*Asegúrate de que la dirección sea un número par (`address & 1 == 0`) y que el modo sea `"thumb"` (16 bits) o `"arm"` (32 bits).*

### 5. Auditar la integridad del archivo de labels
Ejecuta el auditor estático para comprobar que la nueva etiqueta no colisione con funciones existentes y resida en los límites válidos de la memoria:

```bash
python scripts/diagnostico/auditar_labels.py
```

### 6. Regenerar el código C11 nativo
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
