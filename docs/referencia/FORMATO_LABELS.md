---
tipo: referencia
verificado: si
---

# Especificación Técnica: Formato de Etiquetas (`gba-labels v2`)

Esta especificación describe la estructura, atributos y comportamiento del archivo de etiquetas [`blazing.labels.toml`](../../blazing.labels.toml) utilizado por el motor de recompilación estática en `blazing-recomp`.

---

## Descripción general

El archivo de etiquetas cumple dos funciones fundamentales en el ciclo de recompilación:

1. **Semillas de Análisis (Analysis Seeds):** Le indica al desensamblador estático de `recomp` puntos de entrada garantizados de código que no pueden ser descubiertos únicamente siguiendo saltos estáticos directos (por ejemplo, punteros a funciones en tablas de saltos o rutinas dinámicas).
2. **Enriquecimiento de Símbolos (Enrichment):** Asocia nombres humanos (`name`) a direcciones de memoria hexadecimales anónimas. Esto permite que los archivos exportados en C y los registros de depuración (*stack traces* y logs de Android) muestren nombres descriptivos en lugar de identificadores anónimos (`b_XXXXXXXX_a/t`).

---

## Estructura general del documento

El archivo utiliza formato **TOML estándar** bajo la especificación `format = "gba-labels"` versión `2`:

```toml
format = "gba-labels"
version = 2

[image]
sha256 = "<hash-sha256-de-la-rom>"

[[functions]]
name = "NombreDescriptivo"
address = 0x08000000
mode = "arm"
```

---

## Campos y atributos

### Cabecera
* `format`: Cadena fija. Debe ser `"gba-labels"`.
* `version`: Entero. Debe ser `2`.
* `[image]`:
  * `sha256`: Suma de comprobación SHA-256 en minúsculas de la ROM objetivo. Si la ROM cargada no coincide exactamente con este hash, el recompilador rechazará el archivo para prevenir desalineaciones de direcciones.

---

### Entradas de funciones (`[[functions]]`)
Cada función se declara como un elemento de arreglo `[[functions]]` con los siguientes atributos:

| Campo | Tipo | Obligatorio | Valores permitidos | Descripción |
| :--- | :--- | :--- | :--- | :--- |
| `name` | String | No | Cualquier texto alfanumérico | Nombre legible de la función (ej. `Reset_Handler`, `Battle_MainLoop`). Si se omite, la dirección sigue sirviendo como semilla de análisis. |
| `address` | Entero (Hex) | **Sí** | `0x08000000` – `0x08FFFFFF` (ROM)<br>`0x03000000` – `0x03007FFF` (IWRAM) | Dirección de inicio del bloque. Debe estar alineada a número par (bit 0 = 0). |
| `mode` | String | **Sí** | `"arm"` (o `"a"`)<br>`"thumb"` (o `"t"`) | Conjunto de instrucciones en el que inicia la función: **ARM** (32 bits) o **Thumb** (16 bits). |
| `end` | Entero (Hex) | No | Dirección mayor a `address` | Límite exclusivo opcional de final de la función para delimitación precisa. |

---

## Reglas de validación y procesamiento

1. **Alineación de Direcciones:** `address` debe ser una dirección par (`addr & 1 == 0`). El modo de ejecución no se codifica en el bit 0 de la dirección, sino explícitamente en el campo `mode`.
2. **Espacio de Memoria Válido:**
   * **ROM (`0x08xxxxxx` – `0x0Dxxxxxx`):** Se verifica contra el tamaño físico de la ROM (`rom_len`).
   * **IWRAM (`0x03xxxxxx`):** Requiere un volcado binario de los 32 KB de memoria interna (*blob* de RAM capturado en vivo tras la carga de subrutinas por el motor del juego) para que el generador upstream pueda computar el hash FNV-1a de verificación (`guard`) de los bloques traducidos. En la versión actual de este proyecto, las 18.483 etiquetas declaradas pertenecen exclusivamente al espacio de ROM (`0x08000000`–`0x08FFFFFF`); el soporte y captura de subrutinas en IWRAM está registrado como tarea en [`docs/PENDIENTES.md`](../PENDIENTES.md).
3. **Invarianza de Retraducción:** Modificar el campo `name` no invalida la caché de traducción binaria (no fuerza una recompilación completa de C), ya que los nombres son únicamente metadatos de enriquecimiento.

---

## Comandos de gestión CLI

### Herramientas locales del repositorio (`scripts/diagnostico/`)

Para la inspección, auditoría y localización de símbolos sin depender de ejecutables globales externos instalados en el sistema, el repositorio incluye herramientas modulares en Python y PowerShell:

* **Auditar la integridad de las etiquetas frente a la ROM:**
  ```bash
  python scripts/diagnostico/auditar_labels.py
  ```
  Verifica que las 18.483 funciones catalogadas cumplan con alineación par, ausencia de duplicados y rangos válidos en la ROM.

* **Localizar prólogos canónicos de funciones (`PUSH {..., lr}`):**
  ```bash
  python scripts/diagnostico/desensamblar_rom.py --buscar-prologo <dirección>
  ```

* **Regenerar el código C11 nativo preservando `src-c/`:**
  ```powershell
  powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1
  ```

### Subcomandos upstream de `recomp` (cuando el binario está en el PATH)

Si se dispone del binario `recomp` del motor upstream [`JRickey/gba-recomp`](https://github.com/JRickey/gba-recomp) (compilado con `cargo build --release -p recomp` o añadido al `PATH`):

* **Importar etiquetas al sistema:**
  ```bash
  recomp labels import <rom.gba> blazing.labels.toml
  ```
* **Inspeccionar etiquetas cargadas:**
  ```bash
  recomp labels show <rom.gba>
  ```
* **Exportar etiquetas acumuladas tras jugar:**
  ```bash
  recomp labels export <rom.gba> blazing.labels.toml
  ```
