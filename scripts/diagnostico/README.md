# Suite de Diagnóstico, Telemetría e Ingeniería Inversa AOT

Herramientas auxiliares y CLI para auditar el rendimiento en Android, inspeccionar la ROM oficial de Game Boy Advance y mapear nuevas funciones hacia código nativo C11 AOT en **Blazing Recomp**.

---

## 1. Herramientas Disponibles

### `analizar_telemetria.py`
Procesa las líneas emitidas por el arnés de rendimiento del motor (`fps=... ejec=... interprete=...`) y genera un informe estructurado con:

- el ranking de *hotspots* del intérprete (`# hotspots top=...`),
- el desglose de ejecución por región de memoria (`# regiones`, desde el 2026-10-12),
- el reparto de las líneas de la PPU entre caminos de composición (`# ppucamino`, desde el
  2026-10-13): vía rápida (`rectas`) frente a las que necesitaron la segunda capa
  (`mezcla`, `ventana`, `semi`), más el desglose de efectos de color inerte, alfa, brillo y oscuro.

Un log anterior a esas fechas no trae las líneas y el informe avisa u omite la sección.

```bash
# Analizar un archivo de log específico mostrando los 15 hotspots principales:
python scripts/diagnostico/analizar_telemetria.py --archivo telemetria.txt --top 15

# Analizar directamente desde la salida de logcat / adb:
adb logcat -s BlazingRecomp:V | python scripts/diagnostico/analizar_telemetria.py
```

---

### `desensamblar_rom.py`
Inspecciona la ROM oficial en modo Thumb (16 bits) para analizar el flujo de ejecución, identificar llamadas de funciones (`BL`), retornos (`BX LR`) y ramas de tablas de salto indirectas (`mov pc, rx`).

```bash
# Desensamblar 40 instrucciones a partir de una dirección específica:
python scripts/diagnostico/desensamblar_rom.py --addr 0x08013280 --count 40

# Rastrear hacia atrás el prólogo canónico PUSH {..., lr} de un hotspot:
python scripts/diagnostico/desensamblar_rom.py --buscar-prologo 0x080132BE
```

---

### `auditar_labels.py`
Comprueba la validez sintáctica y semántica de [`blazing.labels.toml`](../../blazing.labels.toml) frente a la ROM de Fire Emblem, evitando colisiones de nombres o direcciones repetidas.

```bash
python scripts/diagnostico/auditar_labels.py
```

---

### `compilar_recomp_aot.ps1`
Automatiza la ejecución del recompilador estático `gba-recomp`, forzando la preservación de las unidades `.c` generadas (`RECOMP_KEEP_C=1`) y copiándolas directamente a [`src-c/`](../../src-c/) para su posterior compilación con el NDK.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/diagnostico/compilar_recomp_aot.ps1
```

---

### `fotogramas_dorados.py`
Arnés de fotogramas dorados por escena para detectar regresiones de píxel de la PPU.
A diferencia de un hash global de sesión, localiza el cambio en espacio (columna, fila, caja, rejilla de tiles 8×8) y en tiempo (escena y fotograma ancla).

**Tres subcomandos con el mismo contrato de códigos de salida (0 sin diferencias, 1 diferencias de píxel, 2 entrada inválida / dorados desactualizados):**

```bash
# Comparar dos PPM directamente
python scripts/diagnostico/fotogramas_dorados.py diferenciar a.ppm b.ppm --mapa m.png

# Capturar los dorados de las escenas del manifiesto
python scripts/diagnostico/fotogramas_dorados.py capturar \
    --bin <recomp.exe> --rom <rom.gba> --bios <bios.bin>

# Comparar un binario candidato contra los dorados
python scripts/diagnostico/fotogramas_dorados.py comparar \
    --bin <recomp.exe> --rom <rom.gba> --bios <bios.bin>
```

**Manifiesto de escenas versionado** (`scripts/diagnostico/escenas_doradas.toml`):
```toml
version = 1

[[escena]]
nombre = "entrada-avance"
script = "entrada_avance.input"
anclas = [600, 1800, 3600, 5400, 7200]
```

**Dorados locales** en `out/doradas/` (no versionados, cubiertos por `/out/` en `.gitignore`):
- `entrada-avance_<k>.ppm` — fotograma ancla
- `manifesto.json` — hashes de ROM, BIOS, guion, biblioteca y anclas
- `entrada-avance_<k>.diff.png` — mapa de diferencias si `comparar` falla

**Blindaje por mutación** (M1 — `alpha_blend`, M2 — `in_window_range`): mutar el motor,
reconstruir, y el arnés debe salir con código 1 nombrando un píxel. Los archivos del
motor se restauran en un bloque `finally`, sin comandos destructivos de Git.

---

### `convertir_arnes.py`, `mutar_arnes.py`, `contrastar_arnes.py`
Herramientas de la consolidación del arnés común [`examples/comun/`](../../examples/comun/mod.rs)
(propuesta [2026-10-22](../../docs/propuestas/2026-10-22_arnes-de-experimentos-comun.md)); se conservan
para repetir la operación si aparecen experimentos nuevos.

```bash
# Convertir experimentos al arnés común (quita copias locales, añade `mod comun;` y `use`)
python scripts/diagnostico/convertir_arnes.py --solo shim_bg0_710,ppu_nativa_afin
python scripts/diagnostico/convertir_arnes.py            # todos los pendientes

# Blindaje por mutación del arnés (los 5 mutantes deben morir contra tests/arnes_experimentos_tests.rs)
python scripts/diagnostico/mutar_arnes.py

# Capturar y contrastar salida + volcados de experimentos, byte a byte
python scripts/diagnostico/contrastar_arnes.py capturar <dir>
python scripts/diagnostico/contrastar_arnes.py           # contrasta contra el directorio dado
```

`mutar_arnes.py` y `contrastar_arnes.py` respaldan en memoria RAM y restauran en un bloque
`finally`, sin comandos destructivos de Git.
