# Especificación Técnica: Configuración de Empaquetado (`pack.toml`)

Esta especificación documenta todas las secciones, claves y valores admitidos por el archivo de manifiesto [`pack.toml`](../../pack.toml), el cual define cómo se compila, empaqueta y ejecuta la versión nativa de **Fire Emblem: The Blazing Blade** en `blazing-recomp`.

---

## 1. Visión General

`pack.toml` es el manifiesto declarativo del proyecto. Define qué ROM y BIOS se utilizarán (mediante hashes SHA-256 inmutables, sin almacenar archivos binarios propietarios), qué módulos del motor de Rust (`gba-core`) se habilitan en tiempo de ejecución y qué formato de salida se genera.

---

## 2. Secciones del Manifiesto

### 2.1 `[package]` — Metadatos del Entregable
Controla la identidad del paquete y las plataformas destino:

```toml
[package]
name = "blazing-recomp"
version = "0.1.0"
platforms = ["windows", "linux", "macos"]
icon = "assets/icon.png"   # Opcional: PNG cuadrado (mínimo 256x256, ideal 1024x1024)
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `name` | String | *(Obligatorio)* | Nombre base del ejecutable nativo generado. |
| `version` | String | `"0.1.0"` | Versión SemVer del port. |
| `platforms` | Array de Strings | `["macos", "windows", "linux"]` | Plataformas de escritorio soportadas. Para Android se usa la compilación cruzada mediante NDK. |
| `icon` | Path (relativo) | Ninguno | Ruta a un icono maestro PNG a partir del cual se generan los iconos nativos (.ico, .icns). |

---

### 2.2 `[image]` — Verificación de ROM y BIOS
Garantiza el modelo libre de propiedad intelectual (*Clean-Room / IP-Free*):

```toml
[image]
rom-sha256  = "6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9"
bios-sha256 = "fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570"
```

| Clave | Tipo | Obligatorio | Descripción |
| :--- | :--- | :--- | :--- |
| `rom-sha256` | String | **Sí** | Hash SHA-256 de la ROM de GBA. El ejecutable rehusará iniciar con cualquier ROM distinta. |
| `bios-sha256` | String | **Sí** | Hash SHA-256 de la BIOS oficial de GBA suministrada por el usuario. |

---

### 2.3 `[labels]` — Mapa de Funciones y Símbolos
Vincula el archivo de etiquetas para la recompilación:

```toml
[labels]
file = "blazing.labels.toml"
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `file` | Path (relativo) | Ninguno | Ruta al archivo de etiquetas [`blazing.labels.toml`](../../blazing.labels.toml) (`gba-labels v2`). Ver [FORMATO_LABELS.md](FORMATO_LABELS.md). |

---

### 2.4 `[runtime]` — Módulos del Motor en Ejecución
Configura qué subsistemas emulados se enlazan en la biblioteca nativa:

```toml
[runtime]
menu = true
enhanced-audio = true
screen-sim = true
engine-hle = "m4a"
interpreter = true
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `menu` | Booleano | `true` | Incluye la interfaz de menú en juego (accesible con `Escape` o menú en pantalla en Android). |
| `enhanced-audio` | Booleano | `true` | Habilita el resampler sinc por canal, soft-clipping y mezcla sombra de alta fidelidad. |
| `screen-sim` | Booleano | `true` | Habilita la simulación del panel LCD con shaders por GPU (rejilla de subpíxeles y corrección de color). |
| `engine-hle` | String | `"auto"` | Motor de audio HLE: `"auto"`, `"m4a"` (Music Player 2000, usado en Fire Emblem), `"gax"`, `"rdrv"` o `"off"`. |
| `interpreter` | Booleano | `true` | **Red de seguridad:** Si es `true`, los saltos dinámicos no resueltos caen al intérprete seguro de Rust. Si es `false`, un salto no resuelto detiene el juego (*panic*). |

---

### 2.5 `[build]` — Opciones de Compilación de C
Ajusta qué compilador se invoca durante la generación de código:

```toml
[build]
compiler = "clang"
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `compiler` | String | `None` (TinyCC / cc del sistema) | Compilador de C a utilizar (`"clang"`, `"gcc"`, o ruta ejecutable). En Android se sobreescribe con el Clang del NDK. |

---

### 2.6 `[output]` — Artefactos Generados
Controla qué archivos emite el pipeline de empaquetado:

```toml
[output]
binary = true
c-source = true
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `binary` | Booleano | `true` | Genera la biblioteca dinámica o ejecutable nativo compilado. |
| `c-source` | Booleano | `false` | Emite la carpeta [`src-c/`](../../src-c/) con el código fuente C11 recompilado para inspección, depuración y estudio. |
