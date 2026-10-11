# Suite de Diagnóstico, Telemetría e Ingeniería Inversa AOT

Herramientas auxiliares y CLI para auditar el rendimiento en Android, inspeccionar la ROM oficial de Game Boy Advance y mapear nuevas funciones hacia código nativo C11 AOT en **Blazing Recomp**.

---

## 1. Herramientas Disponibles

### `analizar_telemetria.py`
Procesa las líneas emitidas por el arnés de rendimiento del motor (`fps=... ejec=... interprete=...`) y genera un informe estructurado con el ranking de *hotspots*.

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
