# Especificación Técnica: Formato de Etiquetas (`gba-labels v2`)

Esta especificación describe la estructura, atributos y comportamiento del archivo de etiquetas [`blazing.labels.toml`](../../blazing.labels.toml) utilizado por el motor de recompilación estática en `blazing-recomp`.

---

## 1. Propósito del Archivo de Etiquetas

El archivo de etiquetas cumple dos funciones fundamentales en el ciclo de recompilación:

1. **Semillas de Análisis (Analysis Seeds):** Le indica al desensamblador estático de `recomp` puntos de entrada garantizados de código que no pueden ser descubiertos únicamente siguiendo saltos estáticos directos (por ejemplo, punteros a funciones en tablas de saltos o rutinas dinámicas).
2. **Enriquecimiento de Símbolos (Enrichment):** Asocia nombres humanos (`name`) a direcciones de memoria hexadecimales anónimas. Esto permite que los archivos exportados en C y los registros de depuración (*stack traces* y logs de Android) muestren nombres descriptivos en lugar de identificadores anónimos (`b_XXXXXXXX_a/t`).

---

## 2. Estructura General del Documento

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

## 3. Campos y Atributos

### 3.1 Cabecera
* `format`: Cadena fija. Debe ser `"gba-labels"`.
* `version`: Entero. Debe ser `2`.
* `[image]`:
  * `sha256`: Suma de comprobación SHA-256 en minúsculas de la ROM objetivo. Si la ROM cargada no coincide exactamente con este hash, el recompilador rechazará el archivo para prevenir desalineaciones de direcciones.

---

### 3.2 Entradas de Funciones (`[[functions]]`)
Cada función se declara como un elemento de arreglo `[[functions]]` con los siguientes atributos:

| Campo | Tipo | Obligatorio | Valores permitidos | Descripción |
| :--- | :--- | :--- | :--- | :--- |
| `name` | String | No | Cualquier texto alfanumérico | Nombre legible de la función (ej. `Reset_Handler`, `Battle_MainLoop`). Si se omite, la dirección sigue sirviendo como semilla de análisis. |
| `address` | Entero (Hex) | **Sí** | `0x08000000` – `0x08FFFFFF` (ROM)<br>`0x03000000` – `0x03007FFF` (IWRAM) | Dirección de inicio del bloque. Debe estar alineada a número par (bit 0 = 0). |
| `mode` | String | **Sí** | `"arm"` (o `"a"`)<br>`"thumb"` (o `"t"`) | Conjunto de instrucciones en el que inicia la función: **ARM** (32 bits) o **Thumb** (16 bits). |
| `end` | Entero (Hex) | No | Dirección mayor a `address` | Límite exclusivo opcional de final de la función para delimitación precisa. |

---

## 4. Reglas de Validación y Procesamiento

1. **Alineación de Direcciones:** `address` debe ser una dirección par (`addr & 1 == 0`). El modo de ejecución no se codifica en el bit 0 de la dirección, sino explícitamente en el campo `mode`.
2. **Espacio de Memoria Válido:**
   * **ROM (`0x08xxxxxx` – `0x0Dxxxxxx`):** Se verifica contra el tamaño físico de la ROM (`rom_len`).
   * **IWRAM (`0x03xxxxxx`):** Solo es translatable si existe una captura local de memoria viva (*blob* de IWRAM) registrado durante una sesión con `--record-labels`.
3. **Invarianza de Retraducción:** Modificar el campo `name` no invalida la caché de traducción binaria (no fuerza una recompilación completa de C), ya que los nombres son únicamente metadatos de enriquecimiento.

---

## 5. Comandos de Gestión CLI

El recompilador proporciona comandos nativos para manipular este archivo:

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
