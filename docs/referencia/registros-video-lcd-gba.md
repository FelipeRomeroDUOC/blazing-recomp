---
tipo: referencia
verificado: si
---

# Especificación Técnica: Registros I/O y Modos de Video del Controlador LCD de GBA

## Descripción general

Esta especificación documenta los registros mapeados en memoria (*Memory-Mapped I/O*, MMIO) del controlador de video LCD de la Game Boy Advance (`0x04000000` a `0x04000056`), sus máscaras de bits, tipos de acceso y el comportamiento de los modos gráficos (Modos 0 a 5).

---

## 1. Mapa de Registros I/O del Controlador de Pantalla

Todos los registros residen en el espacio de E/S (`0x04000000`–`0x040003FF`) con alineación de medio-palabra (16 bits) o palabra (32 bits).

| Dirección | Identificador | Acceso | Ancho | Descripción |
| :--- | :--- | :---: | :---: | :--- |
| `0x04000000` | `DISPCNT` | R/W | 16 bits | Control general de pantalla y capas activas. |
| `0x04000002` | `GREENSWAP` | R/W | 16 bits | Intercambio de intensidad verde (no documentado / depuración). |
| `0x04000004` | `DISPSTAT` | R/W | 16 bits | Estado de pantalla e interrupciones (VBlank, HBlank, VCount). |
| `0x04000006` | `VCOUNT` | R | 16 bits | Contador de la línea de escaneo actual ($0 \le Y \le 227$). |
| `0x04000008` | `BG0CNT` | R/W | 16 bits | Control del fondo 0 (prioridad, charblock, mapblock, tamaño). |
| `0x0400000A` | `BG1CNT` | R/W | 16 bits | Control del fondo 1. |
| `0x0400000C` | `BG2CNT` | R/W | 16 bits | Control del fondo 2 (afín en modos 1 y 2). |
| `0x0400000E` | `BG3CNT` | R/W | 16 bits | Control del fondo 3 (afín en modo 2). |
| `0x04000010` | `BG0HOFS` | W | 16 bits | Desplazamiento horizontal (scroll X) del fondo 0. |
| `0x04000012` | `BG0VOFS` | W | 16 bits | Desplazamiento vertical (scroll Y) del fondo 0. |
| `0x04000014` | `BG1HOFS` | W | 16 bits | Desplazamiento horizontal del fondo 1. |
| `0x04000016` | `BG1VOFS` | W | 16 bits | Desplazamiento vertical del fondo 1. |
| `0x04000018` | `BG2HOFS` | W | 16 bits | Desplazamiento horizontal del fondo 2 (solo modo texto). |
| `0x0400001A` | `BG2VOFS` | W | 16 bits | Desplazamiento vertical del fondo 2 (solo modo texto). |
| `0x0400001C` | `BG3HOFS` | W | 16 bits | Desplazamiento horizontal del fondo 3 (solo modo texto). |
| `0x0400001E` | `BG3VOFS` | W | 16 bits | Desplazamiento vertical del fondo 3 (solo modo texto). |
| `0x04000020` | `BG2PA` | W | 16 bits | Parámetro afín DX ($A$) de rotación/escalado de BG2 (punto fijo 8.8). |
| `0x04000022` | `BG2PB` | W | 16 bits | Parámetro afín DMX ($B$) de rotación/escalado de BG2. |
| `0x04000024` | `BG2PC` | W | 16 bits | Parámetro afín DY ($C$) de rotación/escalado de BG2. |
| `0x04000026` | `BG2PD` | W | 16 bits | Parámetro afín DMY ($D$) de rotación/escalado de BG2. |
| `0x04000028` | `BG2X` | W | 32 bits | Coordenada X del punto de referencia de BG2 (punto fijo 19.8 signado). |
| `0x0400002C` | `BG2Y` | W | 32 bits | Coordenada Y del punto de referencia de BG2 (punto fijo 19.8 signado). |
| `0x04000030` | `BG3PA` | W | 16 bits | Parámetro afín DX de BG3. |
| `0x04000032` | `BG3PB` | W | 16 bits | Parámetro afín DMX de BG3. |
| `0x04000034` | `BG3PC` | W | 16 bits | Parámetro afín DY de BG3. |
| `0x04000036` | `BG3PD` | W | 16 bits | Parámetro afín DMY de BG3. |
| `0x04000038` | `BG3X` | W | 32 bits | Coordenada X del punto de referencia de BG3. |
| `0x0400003C` | `BG3Y` | W | 32 bits | Coordenada Y del punto de referencia de BG3. |
| `0x04000040` | `WIN0H` | W | 16 bits | Límites horizontales de la Ventana 0 ($X_1$ a $X_2$). |
| `0x04000042` | `WIN1H` | W | 16 bits | Límites horizontales de la Ventana 1. |
| `0x04000044` | `WIN0V` | W | 16 bits | Límites verticales de la Ventana 0 ($Y_1$ a $Y_2$). |
| `0x04000046` | `WIN1V` | W | 16 bits | Límites verticales de la Ventana 1. |
| `0x04000048` | `WININ` | R/W | 16 bits | Capas habilitadas en el interior de Ventana 0 y Ventana 1. |
| `0x0400004A` | `WINOUT` | R/W | 16 bits | Capas habilitadas en el exterior de ventanas y dentro de Ventana OBJ. |
| `0x0400004C` | `MOSAIC` | W | 16 bits | Tamaño del efecto de mosaico para fondos y objetos. |
| `0x04000050` | `BLDCNT` | R/W | 16 bits | Selección de capas y modo de efectos de color (*Blending*). |
| `0x04000052` | `BLDALPHA` | R/W | 16 bits | Coeficientes de ponderación alfa ($EVA$ y $EVB$). |
| `0x04000054` | `BLDY` | W | 16 bits | Coeficiente de brillo (*Brightness*) para aclarado/oscurecido ($EVY$). |

---

## 2. Definición Detallada de Campos de Bits

### `DISPCNT` (`0x04000000`) — Control de Pantalla
- **Bits 0–2 (Modo de Video):** `0` = Modo 0 (4 fondos de texto), `1` = Modo 1 (mixto), `2` = Modo 2 (2 fondos afines), `3` = Modo 3 (bitmap 240×160 a 15bpp), `4` = Modo 4 (bitmap paletizado 240×160 a 8bpp con doble búfer), `5` = Modo 5 (bitmap 160×128 a 15bpp con doble búfer).
- **Bit 3 (Reservado / Modo CGB):** Reservado para la BIOS (`0` = GBA).
- **Bit 4 (Selección de Cuadro):** `0` = Frame 0 (`0x06000000`), `1` = Frame 1 (`0x0600A000`) en Modos 4 y 5.
- **Bit 5 (H-Blank Interval Free):** Si es `1`, permite acceso de la CPU a OAM durante el intervalo HBlank.
- **Bit 6 (Mapeo de Tiles OBJ):** `0` = Mapeo bidimensional (matriz 32×32), `1` = Mapeo unidimensional lineal en VRAM.
- **Bit 7 (Forced Blank):** Si es `1`, apaga la salida de pantalla y libera el bus de VRAM/Palette/OAM para transferencias ultra-rápidas.
- **Bits 8–11 (Habilitación de Fondos):** Bit 8 = BG0, Bit 9 = BG1, Bit 10 = BG2, Bit 11 = BG3 (`1` = visible).
- **Bit 12 (Habilitación de Sprites/OBJ):** `1` = Renderizar capa de objetos.
- **Bits 13–14 (Habilitación de Ventanas):** Bit 13 = Ventana 0, Bit 14 = Ventana 1.
- **Bit 15 (Ventana de Objetos):** `1` = Habilita el canal de ventana guiado por sprites OBJ.

### `DISPSTAT` (`0x04000004`) — Estado del Display e Interrupciones
- **Bit 0 (VBlank Flag):** Solo lectura. Se pone a `1` durante las líneas de borrado vertical (líneas 160 a 226).
- **Bit 1 (HBlank Flag):** Solo lectura. Se pone a `1` durante el período de retorno horizontal de cada scanline.
- **Bit 2 (VCount Match Flag):** Solo lectura. Se pone a `1` cuando el scanline actual coincide con el valor programado en los bits 8–15.
- **Bit 3 (VBlank IRQ Enable):** `1` = Emite interrupción hardware `IRQ_VBLANK` en la línea 160.
- **Bit 4 (HBlank IRQ Enable):** `1` = Emite interrupción hardware `IRQ_HBLANK` al finalizar cada línea activa.
- **Bit 5 (VCount IRQ Enable):** `1` = Emite interrupción hardware `IRQ_VCOUNT` cuando `VCOUNT == VCount Setting`.
- **Bits 8–15 (VCount Setting / LYC):** Valor de scanline objetivo para disparo de coincidencia ($0 \le Y \le 227$).

### `BG0CNT` a `BG3CNT` (`0x04000008`–`0x0400000E`) — Control de Capas de Fondo
- **Bits 0–1 (Prioridad de Fondo):** $0$ a $3$ ($0$ = máxima prioridad frente a la cámara, $3$ = fondo más profundo).
- **Bits 2–3 (Character Base Block):** Bloque base de mosaicos de 16 KB ($0$ a $3$ dentro de `0x06000000`–`0x0600FFFF`).
- **Bit 6 (Mosaico):** `1` = Aplica el filtro de mosaico configurado en `MOSAIC`.
- **Bit 7 (Modo de Color):** `0` = 16 colores por 16 sub-paletas (4 bpp), `1` = 256 colores en 1 paleta única (8 bpp).
- **Bits 8–12 (Screen Base Block):** Bloque base del mapa de tiles en pasos de 2 KB ($0$ a $31$).
- **Bit 13 (Desbordamiento de Área):** En BG2/BG3 afines: `0` = transparente fuera de límites, `1` = repetición cíclica (*wraparound*).
- **Bits 14–15 (Tamaño de Pantalla):**
  - En fondos de texto (Modo 0/1): `0` = 256×256 px (32×32 tiles, 1 bloque de mapa), `1` = 512×256 px (64×32 tiles, 2 bloques horizontales), `2` = 256×512 px (32×64 tiles, 2 bloques verticales), `3` = 512×512 px (64×64 tiles, 4 bloques).
  - En fondos afines (Modo 2): `0` = 128×128 px, `1` = 256×256 px, `2` = 512×512 px, `3` = 1024×1024 px.

### `BLDCNT`, `BLDALPHA`, `BLDY` (`0x04000050`–`0x04000054`) — Efectos de Color
- **`BLDCNT` Bits 0–5 (1er Objetivo):** Bit 0 = BG0, Bit 1 = BG1, Bit 2 = BG2, Bit 3 = BG3, Bit 4 = OBJ, Bit 5 = BD (Backdrop).
- **`BLDCNT` Bits 6–7 (Modo de Efecto):**
  - `0`: Sin efecto especial (renderizado directo).
  - `1`: Mezcla Alfa (*Alpha Blending* entre 1er objetivo y 2do objetivo).
  - `2`: Aclarado (*Brightness Increase* hacia blanco puro).
  - `3`: Oscurecido (*Brightness Decrease* hacia negro puro).
- **`BLDCNT` Bits 8–13 (2do Objetivo):** Mismas capas que el 1er objetivo para la mezcla alfa.
- **`BLDALPHA`:** Bits 0–4 = Coeficiente $EVA$ ($0$ a $16$), Bits 8–12 = Coeficiente $EVB$ ($0$ a $16$). Fórmula:
  $$C_{\text{final}} = \min\left(1.0, \frac{EVA}{16} \times C_1 + \frac{EVB}{16} \times C_2\right)$$
- **`BLDY`:** Bits 0–4 = Coeficiente de brillo $EVY$ ($0$ a $16$).

---

## 3. Modos Gráficos de Video de la Game Boy Advance

| Modo | Tipo | Fondos Disponibles | Modos de Rotación/Escala | Profundidad de Color | Doble Búfer |
| :---: | :---: | :---: | :---: | :---: | :---: |
| **0** | Mosaicos / Texto | BG0, BG1, BG2, BG3 | No | 16/16 (4bpp) o 256/1 (8bpp) | No |
| **1** | Mixto | BG0, BG1 (Texto), BG2 (Afín) | Sí (solo BG2) | 16/16, 256/1 (BG0/1); 256/1 (BG2) | No |
| **2** | Afín | BG2, BG3 | Sí (ambos) | 256/1 (8bpp) | No |
| **3** | Bitmap Directo | BG2 | Sí | 32,768 colores (15bpp directo, 2 B/px) | No (75 KB VRAM) |
| **4** | Bitmap Paletizado | BG2 | Sí | 256 colores indexados (8bpp) | Sí (Frame 0 y Frame 1) |
| **5** | Bitmap Directo (160×128) | BG2 | Sí | 32,768 colores (15bpp directo, 2 B/px) | Sí (Frame 0 y Frame 1) |

*En **Fire Emblem: The Blazing Blade**, el motor gráfico opera casi exclusivamente en **Modo 0** para los mapas tácticos y diálogos, y en **Modo 0 / Modo 1** durante las secuencias de combate animadas y efectos de magias especiales.*
