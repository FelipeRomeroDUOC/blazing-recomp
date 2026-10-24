---
tipo: referencia
verificado: si
actualizado: 2026-10-24
---

# Especificación Técnica: Configuración de Empaquetado (`pack.toml`)

Esta especificación documenta las secciones, claves y valores del archivo de manifiesto [`pack.toml`](../../pack.toml), utilizado por la herramienta upstream `recomp build` / `gba-pack` para guiar la traducción estática inicial de la ROM oficial a código C11.

---

## Descripción general

> [!NOTE]
> **Alcance arquitectónico y delimitación de `pack.toml`:**  
> Este manifiesto es procesado **únicamente por el generador de código estático upstream (`recomp build`)**. El runtime en Rust (`blazing-recomp`), el visor de pruebas y el script de empaquetado del APK para Android (`scripts/package_apk.ps1`) **no leen este archivo en tiempo de ejecución**.
> 
> En particular:
> - El campo `platforms = ["macos", "windows", "linux"]` en `[package]` aplica exclusivamente al empaquetador de escritorio upstream. La compilación cruzada para Android está gobernada por `cargo-ndk` y `scripts/package_apk.ps1` (`arm64-v8a`, `armeabi-v7a`, `x86_64`).
> - La sección `[runtime]` define los módulos enlazados en el binario de escritorio upstream; en Android no tiene ningún efecto en tiempo de ejecución, ya que el ciclo de vida, la GPU (`NativePpu`), el audio y los fallbacks se configuran en código Rust en `src/` y mediante dependencias de `Cargo.toml`.

`pack.toml` es el manifiesto declarativo de entrada para la herramienta de recompilación upstream. Define qué ROM y BIOS se utilizarán (mediante hashes SHA-256 inmutables, sin almacenar archivos binarios propietarios), qué archivo de etiquetas se incluye y cómo `recomp build` estructura la salida en C11.

---

## Secciones del manifiesto

### `[package]` — Metadatos del entregable
Controla la identidad del paquete y las plataformas destino:

```toml
[package]
name = "blazing-recomp"
version = "0.0.2"
platforms = ["windows", "linux", "macos"]
icon = "assets/icon.png"   # Opcional: PNG cuadrado (mínimo 256x256, ideal 1024x1024)
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `name` | String | *(Obligatorio)* | Nombre base del ejecutable nativo generado. |
| `version` | String | `"0.0.2"` | Versión SemVer del port (alineada con `Cargo.toml`). |
| `platforms` | Array de Strings | `["macos", "windows", "linux"]` | Plataformas de escritorio soportadas. Para Android se usa la compilación cruzada mediante NDK. |
| `icon` | Path (relativo) | Ninguno | Ruta a un icono maestro PNG a partir del cual se generan los iconos nativos (.ico, .icns). |

---

### `[image]` — Verificación de ROM y BIOS
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

### `[labels]` — Mapa de funciones y símbolos
Vincula el archivo de etiquetas para la recompilación:

```toml
[labels]
file = "blazing.labels.toml"
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `file` | Path (relativo) | Ninguno | Ruta al archivo de etiquetas [`blazing.labels.toml`](../../blazing.labels.toml) (`gba-labels v2`). Ver [FORMATO_LABELS.md](FORMATO_LABELS.md). |

---

### `[runtime]` — Módulos del motor en ejecución (arnés de escritorio upstream)
Configura qué subsistemas emulados se enlazan cuando la herramienta `recomp` genera un binario de escritorio empaquetado.

> [!NOTE]
> En la aplicación móvil de Android (`blazing-recomp`), estas opciones no tienen efecto en tiempo de ejecución: los subsistemas se inicializan directamente en código Rust nativo (`src/platform.rs`, `src/ppu_nativa.rs`, `src/runner.rs`) y se compilan según las features de Cargo.

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

### `[build]` — Opciones de compilación de C
Ajusta qué compilador se invoca durante la generación de código:

```toml
[build]
compiler = "clang"
```

| Clave | Tipo | Por defecto | Descripción |
| :--- | :--- | :--- | :--- |
| `compiler` | String | `None` (TinyCC / cc del sistema) | Compilador de C a utilizar (`"clang"`, `"gcc"`, o ruta ejecutable). En Android se sobreescribe con el Clang del NDK. |

---

### `[output]` — Artefactos generados
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
