# Blazing Recomp — Fire Emblem: The Blazing Blade

Proyecto independiente de port nativo para **Fire Emblem: The Blazing Blade (GBA)** utilizando la tecnología de recompilación estática de `gba-recomp`.

---

## Estado del Port

- **ROM Requerida:** `Fire Emblem (Europe)`
- **ROM SHA-256:** `6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`
- **BIOS SHA-256:** `fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`
- **Motor de Audio:** `M4A / MusicPlayer2000` (con soporte para *enhanced audio* y *shadow mixer*)

---

## Estructura del Proyecto

- `pack.toml`: Manifiesto de empaquetado del juego para `gba-pack`.
- `blazing.labels.toml`: Mapa de funciones y puntos de entrada de memoria (generado con `--record-labels`).
- `mods/`: Modificaciones, scripts y sustitución de assets (en desarrollo).

---

## Instrucciones de Compilación y Prueba

### 1. Probar y registrar funciones con el runtime:
```powershell
# Desde el directorio del toolkit gba-recomp:
.\recomp.exe play "ruta/a/tu/rom.gba" --bios "ruta/a/gba_bios.bin" --record-labels
```

### 2. Empaquetar como ejecutable independiente:
```powershell
cargo run -p gba-pack -- build pack.toml --rom "ruta/a/rom.gba" --bios "ruta/a/bios.bin"
```

---

## Documentación Técnica

La documentación del proyecto sigue el estándar Diátaxis y se encuentra organizada en [`docs/`](docs/referencia/README.md):

- **Explicación:**
  - [Registro Histórico de Decisiones (ADR)](docs/explicacion/DECISIONES.md): Decisiones arquitectónicas, compromisos y soluciones desde el inicio del proyecto.
  - [Arquitectura del Port Android](docs/explicacion/arquitectura-port-android.md): Modelo de capas, runtime nativo en Rust y subsistemas.
  - [Estrategia de Versionado y Roadmap](docs/explicacion/roadmap-y-versionado.md): Filosofía de versiones y metas hacia la versión 1.0.
  - [Arquitectura de Código C Recompilado](docs/explicacion/arquitectura-c-recompilado.md): Análisis del pipeline AOT y ABI de registros.
  - [Caminos de Composición de la PPU por Software](docs/explicacion/ppu-caminos-de-composicion.md): Por qué hay dos caminos para dibujar una línea, qué separa al atajo de la ventana de sprites y cómo se prueba el defecto.
- **Referencia Técnica:**
  - [APIs y Módulos de Rust](docs/referencia/api-modulos-rust.md): Referencia pública extraída automáticamente del código fuente.
  - [Configuración de Manifiesto (`pack.toml`)](docs/referencia/CONFIGURACION_PACK.md): Especificación de claves y parámetros.
  - [Formato de Etiquetas (`blazing.labels.toml`)](docs/referencia/FORMATO_LABELS.md): Especificación de labels y análisis de bloques.
- **Planificación y Seguimiento:**
  - [Registro de Cambios (Changelog)](CHANGELOG.md): Historial de versiones publicadas.
  - [Backlog del Proyecto](docs/BACKLOG.md): Tareas completadas y hoja de ruta activa.

