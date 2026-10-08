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
