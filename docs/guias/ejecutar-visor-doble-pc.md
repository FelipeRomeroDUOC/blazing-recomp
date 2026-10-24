---
tipo: guia-practica
verificado: si
---

# Cómo ejecutar y depurar el juego localmente con el visor interactivo de doble pantalla

Esta guía práctica explica cómo compilar y ejecutar **Fire Emblem: The Blazing Blade** en un entorno de escritorio local (PC con Windows) utilizando la herramienta nativa `ppu_doble`. El visor permite jugar a 60 FPS en una ventana interactiva dividida lado a lado (`Izquierda: PPU por software` | `Derecha: PPU nativa por GPU`), comparar la fidelidad gráfica en vivo y capturar discrepancias visuales al vuelo.

> [!NOTE]
> **Nota de arquitectura (Intérprete en PC vs. Recompilación en Android):**  
> En entornos de escritorio (PC con Windows/Linux), el binario `ppu_doble` ejecuta la lógica del juego mediante el intérprete de CPU de `gba-core` (`load_native_blocks()` devuelve `None`). Su propósito fundamental es servir como **arnés interactivo de validación visual y diagnóstico de la PPU**: comprobar que la PPU nativa acelerada por GPU (`wgpu`) dibuje el 100% de los píxeles de forma idéntica al renderizador por software de referencia. La biblioteca de 144.875 bloques C11 recompilados Ahead-Of-Time (`libblazing_recompiled.so`) se enlaza dinámicamente mediante `dlopen` de forma exclusiva en el target nativo de Android.

---

## Requisitos

Antes de iniciar la ejecución local, asegúrate de cumplir las siguientes condiciones:
1. **Sistema Operativo:** Windows 10 u 11 (la ventana interactiva utiliza la API nativa de Win32/GDI para minimizar dependencias).
2. **Rust Toolchain:** Compilador estable de Rust y Cargo instalados en tu equipo.
3. **ROM oficial de GBA:** Copia legal de `Fire Emblem (Europe).gba` (o volcado No-Intro `1575 - Fire Emblem (E)(Independent).gba`) con hash SHA-256 verificado (`6f2c30823ab0237310b138739ccfad879f1a07c4a782a9cbb92317d60d03cee9`).
4. **BIOS oficial de GBA (Opcional pero recomendada):** `gba_bios.bin` con SHA-256 (`fd2547724b505f487e6dcb29ec2ecff3af35a841a77ab2e85fd87350abd36570`). Si se encuentra en `roms/gba_bios.bin` o en la misma carpeta que la ROM, se detectará automáticamente.
5. **Repositorio hermano `gba-recomp` (Worktree local):** Para compilar el arnés `ppu_doble` con la feature `proto`, el proyecto requiere el repositorio hermano clonado en `../gba-recomp` (enlazado mediante la directiva `[patch]` de `Cargo.toml`). Las extensiones de telemetría de video (`perf-video`) y captura de LUT vertical por scanline residen en dicho árbol local en proceso de integración en el upstream.

---

## Pasos

### 1. Iniciar el visor de doble pantalla desde la terminal
Abre una terminal PowerShell o consola de comandos en la raíz del repositorio y ejecuta:

```powershell
cargo run --release --features proto --bin ppu_doble -- "ruta/a/Fire Emblem (Europe).gba"
```

*Si deseas especificar la ruta exacta de la BIOS manualmente:*
```powershell
cargo run --release --features proto --bin ppu_doble -- "ruta/a/rom.gba" "ruta/a/gba_bios.bin"
```

El programa abrirá una ventana de 1440×480 píxeles (escala 3x entera de dos pantallas contiguas de 240×160 píxeles) con el juego en ejecución a 60 FPS:
- **Panel izquierdo:** Salida de la PPU de referencia por software.
- **Panel derecho:** Salida de la PPU nativa acelerada por GPU (`wgpu`).

### 2. Controlar el juego con el teclado
El visor mapea los controles físicos de la Game Boy Advance directamente a tu teclado:

| Botón de GBA | Tecla en PC | Función |
| :--- | :---: | :--- |
| **D-Pad (Cruceta)** | `Flechas` (↑ ↓ ← →) | Desplazamiento por el mapa táctico y menús. |
| **Botón A** | `Z` | Confirmar acción / interactuar. |
| **Botón B** | `X` | Cancelar / retroceder / ver rango de unidades. |
| **Start** | `Enter` | Menú del mapa / pausa. |
| **Select** | `Retroceso` (*Backspace*) | Ayuda contextual / información de estadísticas. |
| **Gatillo L** | `A` | Ciclar entre unidades aliadas disponibles. |
| **Gatillo R** | `S` | Ver ficha detallada de la unidad o terreno. |

### 3. Usar los atajos interactivos de diagnóstico
El arnés incorpora controles en caliente para inspección visual y captura de errores sin interrumpir la partida:

| Atajo | Acción | Utilidad |
| :---: | :--- | :--- |
| `Tab` | **Alternar resaltado en magenta** | Tinta en color magenta brillante los píxeles del panel derecho que difieran del panel izquierdo, permitiendo detectar desajustes sutiles al instante. |
| `F5` | **Forzar captura PNG y diagnóstico** | Envía el fotograma actual a un hilo en segundo plano que guarda una captura compuesta de 960×160 en `volcados/diferencias/` y genera un informe `.txt` con el análisis causal de capas. |
| `L` | **Alternar referencia de software** | Alterna el panel izquierdo entre el renderizado línea por línea del emulador (*scanline sync*) y la reconstrucción final de cuadro. |
| `Espacio` | **Pausar / Reanudar** | Detiene la simulación del juego para analizar un fotograma estático en detalle. |
| `N` | **Avanzar 1 fotograma** | En modo pausa, avanza exactamente un frame para examinar animaciones paso a paso. |
| `Shift` | **Modo Turbo** | Desactiva el limitador de 60 FPS mientras se mantenga presionado, acelerando la simulación. |
| `Esc` | **Salir** | Cierra la ventana y finaliza el proceso de forma limpia. |

---

## Comprobar el resultado

Para validar que el visor opera con normalidad y que no existen desajustes visuales:

1. **Inspeccionar la barra de título de la ventana:**
   Durante la partida, la barra de título se actualiza periódicamente mostrando métricas en vivo:
   ```text
   PPU Doble | Cuadro 1240 | dif: 0 px | 59.8 FPS | ref: línea
   ```
   - Si `dif: 0 px`, la PPU nativa por GPU es idéntica píxel a píxel a la referencia de software.
   - Si se detecta una diferencia gráfica, el visor registrará el número de píxeles divergentes y guardará automáticamente un informe diagnóstico sin congelar la partida.

2. **Revisar capturas generadas en `volcados/diferencias/`:**
   Si se pulsó `F5` o se detectó una discrepancia gráfica nueva, navega a la carpeta `volcados/diferencias/`:
   - `cuadro-<N>-manual.png`: Imagen compuesta de 960×160 píxeles que contiene cuatro paneles contiguos: `[Software Fin | Software Línea | Nativa GPU | Diferencias en Magenta]`.
   - `cuadro-<N>-manual.txt`: Informe técnico que desglosa qué capa causó la diferencia (fondos BG0–BG3, sprites de la OAM o modos de mezcla alfa).
