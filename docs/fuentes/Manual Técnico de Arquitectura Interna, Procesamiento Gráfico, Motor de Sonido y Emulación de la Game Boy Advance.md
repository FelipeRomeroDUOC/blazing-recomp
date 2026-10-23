# Manual Técnico de Arquitectura Interna, Procesamiento Gráfico, Motor de Sonido y Emulación de la Game Boy Advance

## 1. Arquitectura Central del Procesador ARM7TDMI y Gestor de Memoria

El dominio profundo de la arquitectura del procesador central y la organización del mapa de memoria del sistema Game Boy Advance (GBA) constituye un requisito fundamental para los ingenieros dedicados a la emulación precisa, la adaptación de software (*porting*) y la recompilación binaria. Comprender la interacción entre el núcleo de procesamiento, los buses de datos y las restricciones físicas de la memoria permite estructurar modelos de ejecución que repliquen el comportamiento determinista del hardware original con fidelidad a nivel de ciclo. A efectos de especificación de ingeniería moderna, esta documentación abarca el núcleo ARM7TDMI tanto en la arquitectura nativa de Game Boy Advance como dentro de su entorno de retrocompatibilidad y co-procesamiento en la plataforma Nintendo DS, discriminando con precisión las expansiones físicas del mapa de memoria entre ambas revisiones de hardware.

El núcleo central del sistema se fundamenta en un procesador RISC de 32 bits ARM7TDMI (arquitectura ARMv4T), el cual opera a una frecuencia de reloj nominal de 16.78 MHz en la consola GBA nativa y a 33.514 MHz cuando actúa como subsistema de control de E/S y audio en el entorno DS. Este procesador implementa la convención de almacenamiento de datos *Little-Endian* y destaca por su capacidad de ejecución dual mediante dos conjuntos de instrucciones diferenciados: el modo ARM de 32 bits y el modo Thumb de 16 bits. La arquitectura carece de unidad de punto flotante (FPU) por hardware y de instrucciones complejas CISC, ejecutando la mayoría de las instrucciones en un único ciclo de reloj en condiciones ideales de bus mediante un cauce de canalización (*pipeline*) de 3 etapas (*Fetch*, *Decode*, *Execute*).

Para clarificar la topología del sistema, la siguiente tabla detalla la asignación de direcciones del espacio de memoria, especificando las variaciones entre la consola nativa GBA y su modo heredado/extendido, los anchos de bus, las restricciones de acceso y sus funciones operativas:

| Espacio de Memoria / Región                        | Ancho de Bus                 | Restricciones de Acceso y Velocidad                                                                                                                                   | Rol en el Sistema                                                                                                                                    |
| -------------------------------------------------- | ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| **IWRAM** (*Internal Working RAM*) (`0x03000000`)  | 32 bits                      | Alta velocidad (0 estados de espera). 32 KB en GBA nativa / 64 KB a 96 KB mapeados para ARM7 en entorno DS.                                                           | Almacenamiento exclusivo de código crítico, vectores de interrupción (`0x03007FFC` / `0x027FF000`), pila (*stack*) y variables de alto rendimiento.  |
| **EWRAM / Main RAM** (`0x02000000`)                | 16 bits (GBA) / 32 bits (DS) | 256 KB en GBA nativa (2 estados de espera) / 4 MB en DS (`0x02000000–0x023FFFFF`) compartidos con arbitraje de bus.                                                   | Código general de la aplicación, datos globales, variables no constantes, bucles de lógica principal y búferes de trabajo.                           |
| **Espejo RAM No Cacheado** (`0x02400000`)          | 32 bits                      | Acceso directo a RAM principal puenteando las líneas de caché L1/TCM del sistema.                                                                                     | Espejo de lectura/escritura obligatoria para transferencias DMA e intercambio de datos entre procesadores (*IPC*) sin incoherencia de caché.         |
| **VRAM** (*Video RAM*) (`0x06000000`)              | 16 bits / 32 bits            | 96 KB en GBA nativa (límites 240x160) / Hasta 656 KB distribuidos en bancos asignables (VRAM_A–I) en DS (límites 256x192). **Prohibidas escrituras puras de 8 bits**. | Almacenamiento de matrices de mosaicos (*charblocks*), mapas de fondo (*map bases*), estructuras de cuadro lineal (*bitmaps*) y mosaicos de objetos. |
| **OAM** (*Object Attribute Memory*) (`0x07000000`) | 16 bits / 32 bits            | Memoria integrada de 1 KB (128 entradas de objetos, 4 palabras de 16 bits por *sprite*). Bloqueada durante *VDraw*.                                                   | Control de parámetros de renderizado de objetos/sprites (coordenadas X/Y, forma, tamaño, paleta, banderas de prioridad y matrices afines).           |
| **SRAM / Save RAM** (Slot-2) (`0x0E000000`)        | 8 bits                       | Memoria no volátil de guardado (hasta 64 KB). **Lecturas y escrituras estrictamente limitadas a 8 bits**.                                                             | Almacenamiento persistente de partidas guardadas, archivos de configuración y datos de usuario en cartucho.                                          |
| **ROM de Cartucho** (Slot-2) (`0x08000000`)        | 16 bits                      | Mapeo de espacio de almacenamiento masivo de hasta 32 MB. **El bus físico solo soporta escrituras de 16 bits**.                                                       | Vector de almacenamiento no volátil conteniendo el ejecutable binario y los recursos de arte/audio de la aplicación.                                 |
| **Registros I/O Mapeados** (`0x04000000`)          | 16 bits / 32 bits            | Mapeo directo a puertos de E/S de los circuitos periféricos (requiere calificador C `volatile`).                                                                      | Interfaz de control del procesador sobre subsistemas de video, audio, temporizadores, DMA e interrupciones.                                          |

Las restricciones de ancho de bus y las normas de acceso imponen condicionantes severos sobre la velocidad de ejecución y la fidelidad del emulador. Debido a que el bus de la VRAM está optimizado para accesos de 16/32 bits, cualquier intento de realizar una escritura de 8 bits mediante la instrucción ARM `STRB` resulta en una escritura duplicada del byte en el medio-palabra (16 bits) correspondiente o en la corrupción intencionada del dato contiguo. De forma análoga, el bus externo del espacio de cartucho en Slot-2 sólo acepta escrituras de 16 bits; el intento de escribir bytes individuales causa fallos de bus a menos que el software implemente una rutina manual de lectura-modificación-escritura (*read-modify-write*). Por el contrario, la memoria SRAM exige un direccionamiento byte a byte mediante un bus de 8 bits dedicado; los accesos de 16 o 32 bits devuelven datos basura o escriben únicamente en el byte menos significativo. Ignorar estas características físicas en la emulación deriva en desincronización de memoria o bloqueos definitivos del sistema.

Comprendidas las características del procesador y la topología del bus de memoria, es necesario examinar las dinámicas de la canalización de instrucciones, la selección de modos de ejecución y los mecanismos de transferencia masiva de datos mediante hardware dedicado.

## 2. Pipeline de Ejecución, Modos de Instrucción y Transferencias de Datos (DMA)

La optimización del rendimiento en la arquitectura ARM7TDMI depende de la selección del modo de instrucción y de la gestión del controlador de acceso directo a memoria (DMA). El procesador ejecuta un cauce de 3 etapas (*Fetch*, *Decode*, *Execute*); cualquier salto condicional o incondicional no predecible provoca un vaciado completo del *pipeline*, penalizando la ejecución con una pérdida de 2 ciclos de reloj.

La comparación técnica entre el modo ARM de 32 bits y el modo Thumb de 16 bits establece criterios claros para el diseño de sistemas y recompiladores:

- **Densidad de Código:**
  - **Modo Thumb (16 bits):** Presenta una densidad de código sustancialmente superior al reducir la longitud de la instrucción a 16 bits. Permite comprimir binarios hasta un 30-40% en comparación con la representación ARM equivalente.
  - **Modo ARM (32 bits):** Requiere el doble de espacio de almacenamiento por instrucción, reduciendo la eficiencia de empaquetado en memorias pequeñas.
- **Eficiencia de Memoria y Caché:**
  - **Modo Thumb (16 bits):** Es la modalidad idónea para operar desde espacios de memoria conectados a buses de 16 bits (como la ROM de cartucho o la EWRAM nativa). Permite alimentar la etapa de lectura (*Fetch*) del *pipeline* mediante una sola transferencia de bus por instrucción.
  - **Modo ARM (32 bits):** Exige ubicarse en memorias ultrarrápidas alimentadas por buses de 32 bits sin estados de espera (como la IWRAM o memorias de acoplamiento estrecho ITCM) para operar a máxima velocidad.
- **Saturación del Bus:**
  - **Modo Thumb (16 bits):** Minimiza la saturación de los buses de 16 bits, evitando cuellos de botella y manteniendo un flujo contiguo de ejecución.
  - **Modo ARM (32 bits):** Cuando se ejecuta desde regiones con bus de 16 bits, cada instrucción requiere dos accesos consecutivos al bus para completar la lectura de los 32 bits, lo que duplica el tiempo de lectura e introduce un estado de espera obligatorio que devalúa el rendimiento global. Su uso debe restringirse a rutinas matemáticas críticas cargadas en IWRAM.

El subsistema DMA proporciona un motor de transferencia de datos por hardware que opera de forma autónoma respecto a la CPU. El controlador integra 4 canales priorizados jerárquicamente: Canal 0 (máxima prioridad, reservado para transferencias críticas en memoria interna), Canal 1 y Canal 2 (optimizados para el flujo de audio hacia las FIFOs de sonido) y Canal 3 (prioridad más baja, destinado a transferencias masivas de datos entre RAM, VRAM y cartucho). Cada canal se configura mediante registros de control de 32 bits (`DMAxSAD` dirección origen, `DMAxDAD` dirección destino, `DMAxCNT_L` recuento de transferencias y `DMAxCNT_H` registro de control a la dirección `0x040000B8` para Canal 3). Sus modos de disparo (*trigger*) se seleccionan en `DMAxCNT_H`: ejecución inmediata, sincronizada con *HBlank*, con *VBlank*, o por solicitud de refresco de FIFO de audio/captura de pantalla.

```
                  +-----------------------------------+
                  |   Controlador DMA (Canales 0-3)   |
                  +-----------------------------------+
                                    |
            +-----------------------+-----------------------+
            | Petición de Bus                               | Inhabilitación
            v                                               v
+-----------------------+                       +-----------------------+
|  Bus de Memoria RAM   |                       |  Núcleo CPU ARM7TDMI  |
|  (Bloqueado por DMA)  |                       |  (Stalled / Frozen)   |
+-----------------------+                       +-----------------------+
            |                                               |
            | Transferencia Directa                         | Excepción: Código
            v                                               v e Instrucciones en
+-----------------------+                       +-----------------------+
|  VRAM / OAM / EWRAM   |                       |  IWRAM / Caché ITCM   |
+-----------------------+                       +-----------------------+
```

Durante una transferencia DMA activa, el controlador asume el dominio del bus de direcciones y datos, congelando la CPU (*CPU stalling*) si esta intenta acceder al mismo bus de memoria principal. Para prevenir parálisis de procesamiento en secciones de tiempo real, la CPU debe estar ejecutando código residente en la memoria IWRAM de 32 bits. Asimismo, en sistemas que integran estructuras de caché de datos (como la memoria DTCM del ARM9 en arquitectura DS), los circuitos del DMA no poseen coherencia de caché por hardware. Por ello, es técnicamente obligatorio realizar una operación de purga y alineación de líneas de caché de 32 bytes (`DC_FlushAll` o `DC_FlushRange`) sobre las direcciones de origen ubicadas en RAM antes de iniciar la transferencia DMA. De lo contrario, el DMA leerá datos obsoletos alojados en la memoria física RAM. Alternativamente, deben emplearse punteros dirigidos al espejo de memoria no cacheado (`0x02400000–0x027FFFFF`).

Adicionalmente, el hardware integra aceleradores matemáticos independientes para operaciones de división y raíz cuadrada, mapeados en los registros de E/S del sistema:

- **Acelerador de División:** Controlado por el registro `DIV_CR` (`0x04000280`), `REG_DIV_NUMERATOR` (`0x04000290`, 64 bits/32 bits) y `REG_DIV_DENOMINATOR` (`0x04000298`, 32 bits). Opera en modos de división de 32 o 64 bits firmados/no firmados (`div32`, `divf32`). Posee una latencia fija de **18 ciclos de reloj**.
- **Acelerador de Raíz Cuadrada:** Controlado por el registro `SQRT_CR` (`0x040002B0`) y `REG_SQRT_PARAM` (`0x040002B8`, 64 bits/32 bits). Ejecuta la extracción de raíz cuadrada en aritmética entera y de punto fijo con una latencia fija de **13 ciclos de reloj**.

Al escribir los operandos en los registros de entrada, la unidad completa el cálculo en segundo plano sin consumir ciclos de ejecución de la CPU. Un emulador debe modelar con exactitud los ciclos de latencia entre la escritura de los parámetros de entrada y la lectura de los resultados en los registros de salida (`REG_DIV_RESULT`, `REG_SQRT_RESULT`) para mantener la sincronización entre hilos.

Una vez dominados los mecanismos de flujo de código y transferencias masivas en memoria, se debe analizar cómo se procesan y estructuran los gráficos bidimensionales a nivel de hardware.

## 3. Motor de Gráficos 2D: Modos de Video, Capas de Fondo y Display Engine

El procesamiento de gráficos 2D está gobernado por una GPU especializada que genera la imagen mediante un renderizado progresivo por líneas de escaneo (*scanline rendering*). El hardware procesa los píxeles en tiempo real de izquierda a derecha y de arriba a abajo. El espacio coordenado de pantalla nativo de GBA abarca una resolución de 240x160 píxeles, mientras que el motor gráfico extendido en arquitectura DS expande los bordes activos hasta 256x192 píxeles (X \in [0, 255], Y \in [0, 191]).

El motor maneja tres profundidades de color primarias:

1. **Modo Paletizado de 4 bits (16 colores / Sub-paletas):** Cada píxel ocupa 4 bits (*nibble*) e indexa una de las 16 sub-paletas disponibles de 16 colores cada una. Ofrece la menor ocupación de VRAM, permitiendo alborcar un volumen masivo de mosaicos (*tiles*).
2. **Modo Paletizado de 8 bits (256 colores):** Cada píxel ocupa 1 byte e indexa una paleta global de 256 entradas. La paleta almacena valores de 16 bits estructurados en formato BGR de 15 bits (x555 BGR: 5 bits para el componente Rojo, 5 bits para el Verde y 5 bits para el Azul; el bit más significativo x se utiliza a menudo como indicador de transparencia o bandera alfa).
3. **Modo Color Directo de 16 bits:** Definición directa de color por píxel en memoria en formato xBBBBBGGGGGRRRRR. Aunque elimina el uso de paletas, duplica el consumo de ancho de banda y el espacio ocupado en VRAM.

El *Display Engine* cuenta con tres tipos de capas de fondo (*Backgrounds* - BG), cuya versatilidad y capacidades de transformación se comparan en la siguiente tabla técnica:

| Tipo de Fondo                                | Capacidad de *Tiles* y Mapas                                                          | Límites de Mapa en Pantalla                                                                    | Desplazamiento Hardware                                      | Transformaciones Afines (Matriz 2x2)                                                                         |
| -------------------------------------------- | ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| **Text Backgrounds** (*Text BG*)             | Hasta 1024 *tiles* por conjunto (índices de 10 bits). Entradas de mapa de 16 bits.    | Dimensiones de matriz: 32x32, 32x64, 64x32 y 64x64 *tiles* (256x256 hasta 512x512 píxeles).    | Sí (registros `BGxHOFS` y `BGxVOFS` dedicados).              | No. Admite volteo horizontal/vertical individual por *tile* en los bits de control del mapa.                 |
| **Rotoscale Backgrounds** (*Rotation BG*)    | Máximo 256 *tiles* diferentes (índices de 8 bits). Entradas de mapa de 8 bits.        | Matrices cuadradas fijas: 16x16, 32x32, 64x64 o 128x128 *tiles* (128x128 a 1024x1024 píxeles). | Sí (mediante el vector de origen de la matriz afín X_0/Y_0). | **Sí**. Aplicación de matriz afín 2x2: rotación, escalado, inclinación (*shearing*) y traslación.            |
| **Extended Rotoscale** (*Extended BG - ERB*) | Hasta 1024 *tiles* (entradas de 16 bits) o búfer de cuadro lineal (*Bitmap* directo). | Matrices extensas de hasta 1024x1024 píxeles o mapa lineal asignado en bancos VRAM.            | Sí (desplazamiento matricial y vectorial acelerado).         | **Sí**. Soporta el uso de sub-paletas completas en modo paletizado de 256 colores y modos *Bitmap* directos. |

La organización de la VRAM para fondos se fragmenta en dos estructuras lógicas delimitadas:

- **Charblocks (*****Tile Bases*****):** Bloques contiguos de 16 Kilobytes que almacenan los datos de píxeles puros de los mosaicos gráficos (patrones de 8 \times 8 píxeles).
- **Map Bases (*****Screen Bases*****):** Bloques contiguos de 2 Kilobytes que contienen las matrices de mapeo de pantalla.

En los fondos de texto (*Text BG*), cada entrada de la matriz de la *Map Base* ocupa exactamente 2 bytes (16 bits). La distribución interna de bits se define del siguiente modo:

```
 Bit:  15  14  13  12 | 11 | 10 | 9  8  7  6  5  4  3  2  1  0
      +---------------+----+----+-------------------------------+
      | Sub-Paleta    | V  | H  | Índice de Tile (0 a 1023)     |
      | (0 a 15)      |Flip|Flip|                               |
      +---------------+----+----+-------------------------------+
```

- **Bits 0–9:** Índice de *tile* en el *charblock* (valores del 0 al 1023).
- **Bit 10:** Banderas de volteo horizontal (*H-Flip*).
- **Bit 11:** Banderas de volteo vertical (*V-Flip*).
- **Bits 12–15:** Selector de la sub-paleta de 16 colores (utilizado únicamente en el modo paletizado de 4 bits).

Mediante este esquema de pavimentado (*pavage*) por hardware, la CPU reconstruye fondos de pantalla completos con desplazamientos y rotaciones modificando únicamente pequeños vectores en las *map bases*, reduciendo a cero el cómputo de rasterización por software a nivel de procesador.

Comprendido el funcionamiento de los fondos estructurados en *tiles*, se procede a analizar la gestión de las entidades gráficas dinámicas o *sprites*, administrados mediante el subsistema de objetos.

## 4. Subsistema de Objetos (Sprites), Memoria OAM y Transformaciones Afines

La gestión de entidades dinámicas en pantalla (personajes, proyectiles y elementos interactivos) se procesa a través del subsistema de objetos mediante la Memoria de Atributos de Objetos (OAM). La OAM es una memoria de acceso ultrarrápido dedicada de 1 KB, integrada directamente en el núcleo del motor gráfico 2D.

La tabla OAM alberga exactamente 128 entradas de *sprites*. Cada entrada está compuesta por cuatro palabras de 16 bits (Atributos 0, 1, 2 y 3), cuyos campos formateados definen las propiedades del objeto en pantalla:

- **Atributo 0 (**`**0x07000000 + x*8**`**):**
  - **Bits 0–7/8:** Coordenada Y en pantalla (0 a 191 en DS, 0 a 159 en GBA nativa; con ajuste de envolvente lógica o *wrapping* hasta 255/512).
  - **Bit 8:** Flag de rotación/escalado (activa el procesamiento afín).
  - **Bit 9:** Si el Bit 8 está activo: indica doble tamaño de la caja de renderizado (*Double-Size Flag*) para prevenir el recortado en rotaciones. Si el Bit 8 está inactivo: actúa como flag de inhabilitación/ocultamiento del *sprite*.
  - **Bits 10–11:** Modo de objeto: 00 = Normal, 01 = Translucidez (*Semi-transparent*), 10 = Ventana de objeto (*OBJ Window*).
  - **Bit 12:** Flag de mosaico.
  - **Bit 13:** Profundidad de color: 0 = 16 colores/4 bits (16 sub-paletas), 1 = 256 colores/8 bits.
  - **Bits 14–15:** Forma del *sprite* (*Shape*): 00 = Cuadrado, 01 = Horizontal, 10 = Vertical.
- **Atributo 1 (**`**0x07000002 + x*8**`**):**
  - **Bits 0–8:** Coordenada X en pantalla (0 a 511, con envolvente lógica).
  - **Bits 9–13:** Si la transformación afín está activa: selector del conjunto de matriz de rotación/escalado (*Rotset Index*, de 0 a 31). Si está inactiva: Bits 9–11 no se usan; Bit 12 = Flag de volteo horizontal (*H-Flip*); Bit 13 = Flag de volteo vertical (*V-Flip*).
  - **Bits 14–15:** Tamaño del *sprite* (*Size*): Combinado con el campo *Shape* del Atributo 0, define dimensiones desde 8 \times 8 hasta 64 \times 64 píxeles.
- **Atributo 2 (**`**0x07000004 + x*8**`**):**
  - **Bits 0–9:** Índice del *tile* base en la memoria VRAM de objetos (OVRAM).
  - **Bits 10–11:** Prioridad de renderizado del *sprite* frente a las capas de fondo (0 a 3, donde 0 indica prioridad visual superior).
  - **Bits 12–15:** Selector de la sub-paleta de 16 colores (utilizado solo en modo de 4 bits).
- **Atributo 3 (**`**0x07000006 + x*8**`**):**
  - Almacena los parámetros de las matrices afines (PA, PB, PC, PD) entrelazados en grupos de 4 entradas contiguas dentro del espacio de la OAM.

El color transparente de un *sprite* se gestiona a través de una clave de color (*colorkey*). Los píxeles cuyo índice coincida con el valor 0 de la paleta (o con la entrada transparente configurada) no se renderizan, permitiendo la visualización de las capas de fondo subyacentes. En el desarrollo de recursos gráficos de origen, el color magenta puro (x555 BGR: R=31, G=0, B=31 \rightarrow \text{RGB15: } \text{0x7C1F}) se utiliza como la clave de transparencia por convención.

El motor afín de objetos procesa hasta 32 conjuntos de transformación hardware (*rotsets*). Cada *rotset* consta de una matriz de 2 \times 2 operada en aritmética de punto fijo de 16 bits (formato 8.8: 1 bit de signo, 7 bits enteros y 8 bits fraccionarios):

\begin{pmatrix} X_{texture} \\ Y_{texture} \end{pmatrix} = \begin{pmatrix} PA & PB \\ PC & PD \end{pmatrix} \begin{pmatrix} X_{screen} - X_0 \\ Y_{screen} - Y_0 \end{pmatrix}

Debido a la precisión finita de la aritmética de punto fijo, recalcular incrementalmente la matriz multiplicando el estado del cuadro anterior genera una degradación acelerada por acumulación de errores de redondeo. La metodología exige mantener los parámetros flotantes o enteros de ángulo y escala en las variables de estado del juego y recomputar la matriz afín desde cero en cada cuadro antes de empaquetarla en formato 8.8.

Dado que el espacio físico de la OAM (`0x07000000`) se bloquea completamente durante el intervalo de dibujo activo (*VDraw*), las escrituras directas de la CPU sobre la OAM durante este periodo provocan la pérdida de datos o la corrupción de los *sprites* en pantalla. Para solucionar esto, se implementa una técnica de doble búfer por software mediante una **Sombra de OAM** (*Shadow OAM*) en RAM principal:

```
        +-----------------------------------------------+
        |             Lógica del Juego (CPU)            |
        |   (Actualiza coordenadas y parámetros 2D)     |
        +-----------------------------------------------+
                               |
                               v (Escribe libremente durante VDraw o VBlank)
        +-----------------------------------------------+
        |    Sombra OAM en RAM (EWRAM / IWRAM)         |
        |     (Estructura de 512 Bytes / 1024 Bytes)    |
        +-----------------------------------------------+
                               |
                               v Sincronización atómica exclusiva vía DMA 
                               | o memcpy durante el intervalo VBlank
        +-----------------------------------------------+
        |       Memoria OAM Hardware (0x07000000)       |
        +-----------------------------------------------+
                               |
                               v Lectura directa por el rasterizador GPU
        +-----------------------------------------------+
        |             Pantalla / Renderizado            |
        +-----------------------------------------------+
```

1. **Reserva de Memoria:** Se declara en RAM rápida (IWRAM/EWRAM) un arreglo de estructuras con la alineación exacta de los atributos de la OAM (1024 bytes para abarcar los 128 *sprites* y las 32 matrices afines).
2. **Actualización de Estado:** A lo largo del ciclo del juego, la CPU actualiza de forma transparente los datos de posición, escala y animación en la Sombra de OAM sin acceder al bus de la GPU.
3. **Transferencia Atómica en VBlank:** Al dispararse la interrupción `IRQ_VBLANK`, se ejecuta un canal DMA (Canal 3) o una rutina ultrarrápida `memcpy` que copia en un bloque atómico los 1024 bytes de la Sombra de RAM hacia la dirección base de la OAM (`0x07000000`) antes de que se inicie el siguiente período de dibujo activo.

Una vez cubierto el procesamiento gráfico de capa e imágenes estáticas y dinámicas, el siguiente subsistema a detallar es la arquitectura de generación de sonido.

## 5. Arquitectura del Motor de Sonido y Canales de Audio

El motor de audio del sistema se encuentra bajo la gestión directa del procesador ARM7, siendo el único núcleo con acceso físico a los registros del sintetizador y mezclador de audio. El hardware integra un generador y mezclador por hardware de 16 canales estéreo independientes (canales 0 a 15), el cual procesa muestras digitales en paralelo con un impacto inferior al 5% sobre el ancho de banda del bus global de memoria.

Las especificaciones operativas del motor de sonido se resumen a continuación:

- **Formatos de Muestra Admitidos:** Canales configurables de forma individual para decodificar e interpretar datos PCM de 8 bits con signo/sin signo, PCM no comprimido de 16 bits y flujos comprimidos ADPCM por hardware.
- **Panorámica y Control de Salida:** Salida estéreo de dos vías con ajuste de volumen independiente (máximo de 128 niveles) y desplazamiento de panorámica (*pan*) de izquierda a derecha asignable en cada canal.
- **Convertidor Digital-Analógico (DAC):** El DAC interno procesa la señal de audio a 10 bits de resolución de amplitud con una frecuencia de muestreo de salida fija de **32,768 Hz**.

Cualquier flujo de audio introducido en el hardware a una frecuencia distinta de 32,768 Hz (por ejemplo, 11,025 Hz, 22,050 Hz o 44,100 Hz) es remuestreado internamente a 32,768 Hz mediante un algoritmo por hardware de interpolación por el vecino más cercano (*nearest-neighbor*). Debido a la ausencia de filtros de interpolación lineal o polinómica en la etapa de salida, la interpolación por el vecino más cercano introduce un artefacto de distorsión por solapamiento (*aliasing*) en las frecuencias altas. Para mitigar esta distorsión en proyectos de desarrollo o emulación, el software original debe aplicar un filtro de paso bajo por software a las muestras de origen antes de transferirlas a la memoria de audio.

Para la alimentación continua del motor de sonido sin sobrecargar a la CPU con interrupciones por muestra, se utiliza el streaming mediante bucles FIFO alimentados por DMA:

```
+---------------------+      DMA Canal 1 / 2      +---------------------+
| Búfer de Audio RAM  | ------------------------> | FIFO de Audio       |
| (PCM / ADPCM)       |   (Disparo por Timer)     | (16 palabras / 32 B)|
+---------------------+                           +---------------------+
                                                             |
                                                             v Resampling 32,768 Hz
                                                  +---------------------+
                                                  | DAC de 10 bits      |
                                                  | (Salida Analógica)  |
                                                  +---------------------+
```

Los Canales 1 y 2 del controlador DMA se configuran en modo de disparo especial sincronizado con los desbordamientos de los temporizadores (`TIMER0` o `TIMER1`). Cuando el canal de audio consume los datos almacenados en su FIFO de 16 palabras (32 bytes), el hardware de audio emite una petición DMA automática que transfiere instantáneamente un nuevo bloque de datos desde la memoria RAM hacia el registro FIFO del canal correspondiente.

En la arquitectura del software de audio existen tres aproximaciones principales con un impacto directo sobre la CPU y la memoria:

1. **Audio Raw No Comprimido (PCM 8/16 bits):** Mínimo consumo de CPU (< 1%), pero exige un consumo masivo de memoria RAM o ROM en cartucho. Reservado para efectos de sonido ultracortos (*SFX*).
2. **Secuenciación MOD / Tracker (Sintetizador de 16 canales):** Formato preferido en desarrollos comerciales y *homebrew*. Proporciona música mediante secuencias de notas que disparan muestras de instrumentos en bucle a través de los 16 canales del hardware. Consume menos del 5% del ancho de banda y mantiene un tamaño de archivo reducido.
3. **Flujos de Audio Comprimidos (MP3 / Ogg Vorbis):** Mediante el uso de librerías como *Helix MP3* (desarrollada exclusivamente con matemática de punto fijo) o *Tremor* (implementación de Ogg Vorbis sin FPU), se consiguen ratios de compresión del 1800% al 4800% (utilizando codificación VBR). No obstante, la decodificación por software exige entre el 50% y el 100% de la capacidad de cómputo del procesador ARM7 o la delegación de la tarea al procesador ARM9.

El procesador administra el subsistema de audio a través de un grupo de registros de control mapeados en memoria:

- `SOUND_VOL` (`0x04000500`): Registro de control de volumen maestro de 7 bits y sesgo (*bias*) de la señal analógica.
- `SOUNDxCNT` / `SCHANNEL_CR(x)` (`0x04000400 + x*0x10`): Registro de control individual para el canal x (x \in [0, 15]). Define la activación del canal, el formato de la muestra (PCM8, PCM16, ADPCM), el modo de repetición (*looping* continuo o disparo único *one-shot*), la longitud de la muestra, el volumen y el grado de panorámica.
- Llamadas a la BIOS como `swiChangeSoundBias`: Modifican de forma progresiva el nivel de offset de corriente continua (*DC bias*) en la etapa del DAC. Su ejecución previene transitorios eléctricos y chasquidos de alta frecuencia (*pop noises*) en los altavoces al inicializar o apagar la etapa de amplificación.

Para garantizar la sincronización del audio con la tasa de refresco visual, es necesario examinar los mecanismos de interrupción, temporizadores y sincronización vertical.

## 6. Mecanismos de Interrupción Hardware, Temporizadores y Sincronización Vertical

El comportamiento determinista del software y la sincronización del hardware dependen del sistema de interrupciones (*IRQs*) y de los temporizadores programables. Las interrupciones detienen el hilo de ejecución principal instantáneamente para atender eventos críticos del hardware mediante vectores asignados en la rutina de interrupción de la BIOS.

A continuación, se categorizan las interrupciones primarias del sistema, anotando sus direcciones de control I/O asociadas:

- **Interrupciones Ligadas a Pantalla:**
  - `IRQ_VBLANK`: Se dispara al inicio del período de borrado vertical, cuando el hardware de pantalla finaliza el dibujado activo del cuadro. Es el intervalo seguro para actualizar VRAM, la OAM y la paleta.
  - `IRQ_HBLANK`: Se activa al completar el renderizado de cada línea de escaneo. Permite ejecutar efectos de rasterizado y modificar paletas o registros por línea (*scanline FX*).
  - `IRQ_VCOUNT`: Se dispara cuando el contador de la línea de escaneo actual (almacenado en el registro de solo lectura `REG_VCOUNT` a la dirección `0x04000006`) coincide con el valor programado en el registro de estado del display `REG_DISPSTAT` (`0x04000004`).
- **Interrupciones de Temporizadores (**`**IRQ_TIMER0**` **a** `**IRQ_TIMER3**`**):** Emitidas cuando el registro de datos de un temporizador específico sufre un desbordamiento de 16 bits (pasa de `0xFFFF` a `0x0000`).
- **Interrupciones de Transferencia DMA (**`**IRQ_DMA0**` **a** `**IRQ_DMA3**`**):** Emitidas automáticamente tras completar el recuento total de bloques asignados a un canal DMA específico (`DMAxCNT_H`).
- **Interrupción de Teclado / Entrada (**`**IRQ_KEYS**`**):** Disparada cuando el estado de pulsación de los botones coincide con la máscara de condiciones programada en el registro de control de teclas `REG_KEYCNT` (`0x04000132`).
- **Interrupción de Cartucho / Slot Externo (**`**IRQ_CART**`**):** Emitida al detectarse eventos de inserción, extracción o bus en el puerto de cartucho Slot-2.
- **Interrupciones de Control Global:** Administradas por los registros de activación e identificación de interrupciones: `REG_IE` (`0x04000200`, *Interrupt Enable*), `REG_IF` (`0x04000202`, *Interrupt Flag*) y `REG_IME` (`0x04000208`, *Interrupt Master Enable*).

El ciclo de renderizado de pantalla está sincronizado a una frecuencia de refresco vertical estricta de **60 Hz** (lo que equivale a un período exacto de **16.7 milisegundos** por cuadro). Este ciclo se fragmenta en dos intervalos de operación:

```
|<------------------ Periodo de Cuadro (16.7 ms / 60 Hz) ------------------>|
+------------------------------------------+-------------------------------+
| Fase VDraw (Dibujo Activo)               | Fase VBlank (Inactivo)        |
| Líneas 0 a 159 (GBA) / Líneas 0 a 191(DS)| Líneas 160-227 / Líneas 192-262|
| (VRAM, OAM y Paletas BLOQUEADAS)         | (Transferencias Seguras)      |
+------------------------------------------+-------------------------------+
```

1. **Fase de Dibujo Activo (*****VDraw*****):** El hardware lee la VRAM, OAM y paletas para generar los píxeles en pantalla. Las memorias gráficas se encuentran con restricciones de acceso para la CPU.
2. **Fase de Borrado Vertical (*****VBlank*****):** Durante el intervalo de retorno del haz de escaneo desde la última línea hasta la primera, la pantalla permanece libre de lecturas de dibujado. La CPU ejecuta las transferencias de memoria gráfica, la actualización de la Sombra de OAM y la sincronización de la lógica del juego. Para eliminar el rasgado de imagen (*tearing*), el software ejecuta rutinas de sincronización vertical (*VSync*) o de intercambio de páginas (*page flipping*), suspendiendo la CPU mediante la llamada de ahorro de energía a la BIOS `swiWaitForVBlank`.

El sistema dispone de **4 temporizadores incrementales de 16 bits** por CPU (identificados de 0 a 3). Cada temporizador se alimenta de una fuente de reloj base de 33.514 MHz y se controla mediante dos registros mapeados: `TIMER_DATA` / `TMxCNT_L` (`0x04000100 + x*4`) y `TIMER_CR` / `TMxCNT_H` (`0x04000102 + x*4`). Sus características funcionales se detallan a continuación:

- **Escalado por Prescaler:** La frecuencia de reloj de 33.514 MHz puede dividirse seleccionando prescalers en `TMxCNT_H`: de forma directa por 1 (33.514 MHz), por 64 (523.65 kHz), por 256 (130.91 kHz) o por 1024 (32.72 kHz).
- **Modo en Cascada (*****Cascading Overflow*****):** Activando el bit de cascada en `TMxCNT_H`, el temporizador ignora el reloj base y se incrementa un ciclo únicamente cuando el temporizador de índice inferior (x-1) sufre un desbordamiento (pasa de `0xFFFF` a `0x0000`). Esto permite interconectar dos temporizadores de 16 bits para constituir un contador de 32 bits de alta precisión destinado al perfilado de ciclos (*profiling*) o a la programación de eventos de larga duración.

Comprendidos los componentes físicos, el siguiente apartado consolida las técnicas de software empleadas para replicar esta arquitectura mediante emulación y recompilación.

## 7. Estrategias de Emulación, Recompilación de Software y Portabilidad

La recreación del hardware de Game Boy Advance mediante software de emulación o recompilación binaria exige modelos de ejecución que sinteticen el comportamiento del procesador, el mapa de memoria y la GPU con precisión de ciclos.

Existen dos paradigmas principales para el modelado de ejecución del procesador ARM7TDMI:

```
                       Paradigmas de Ejecución de Código
                                      |
            +-------------------------+-------------------------+
            |                                                   |
            v                                                   v
+-----------------------+                           +-----------------------+
|  Bucle Interpretador  |                           | Recompilación Dinámica|
|  (Fetch-Decode-Exec)  |                           |     (JIT / Static)    |
+-----------------------+                           +-----------------------+
| - Decodificación bit  |                           | - Traducción de bloques|
|   a bit por op-code.  |                           |   ARM/Thumb a x86/ARM64|
| - Precisión ciclo a   |                           | - Máximo rendimiento. |
|   ciclo perfecta.     |                           | - Complejidad de      |
| - Elevado consumo     |                           |   invalidación por    |
|   de CPU en host.     |                           |   código automodificado|
+-----------------------+                           +-----------------------+
```

- **Interpretador Aislado (Bucle** ***Fetch-Decode-Execute*****):** Extrae cada instrucción de 16 bits (Thumb) o 32 bits (ARM) desde la memoria virtual emulada, decodifica los campos de bits del *opcode* mediante tablas de saltos (*jump tables*) o estructuras `switch` complejas y actualiza el estado de los registros emulados. Aunque garantiza el procesamiento preciso ciclo a ciclo y simplifica el manejo de interrupciones, impone una carga computacional elevada sobre la CPU anfitriona.
- **Recompilador Dinámico / Estático (JIT / Traducción Binaria Estática):** Traduce bloques de código básico ARM/Thumb a código máquina nativo de la arquitectura anfitriona (por ejemplo, x86_64 o ARM64). Alcanza un rendimiento superior, pero exige mecanismos para invalidar bloques de código traducidos cuando el programa realiza escrituras en la memoria RAM principal (detección de código automodificable) e intercala comprobaciones de temporización (*timing checks*) para no perder la sincronización con los periféricos de video y sonido.

Para mapear la arquitectura emulada sobre el sistema anfitrión, la infraestructura de emulación debe satisfacer los siguientes requerimientos de diseño:

1. **Gestión de Registros de E/S y Búferes:** Las direcciones de memoria de los registros mapeados (`0x04000000`) deben enlazarse a controladores de lectura y escritura con funciones *callback* explícitas (`readIO32`, `writeIO16`). El emulador debe interceptar las escrituras en registros como `DISPSTAT`, `DMAxCNT_H` o `TIMER_CR` para programar el estado de los componentes periféricos.
2. **Programador de Eventos (*****Event Scheduler*****):** En lugar de verificar el estado de los periféricos tras cada instrucción, el emulador implementa una cola de eventos ordenada por ciclos de reloj. Se calcula el ciclo exacto en el que ocurrirá la próxima interrupción `HBlank`, desbordamiento de temporizador o transferencia DMA, y se avanza el reloj global hasta alcanzarlo, reduciendo las comprobaciones periódicas.

Para agentes de inteligencia artificial y desarrolladores de emuladores o adaptaciones de software (*ports*), se prescriben las siguientes instrucciones técnicas de obligado cumplimiento:

1. **Preservación de Aritmética de Punto Fijo:** No deben sustituirse las operaciones de punto fijo originales por tipos de punto flotante (`float` o `double`) del lenguaje C/C++. Se deben mantener el escalado de enteros y las tablas de búsqueda precalculadas (*Look-Up Tables* - LUT para senos, cosenos y tangentes en formato 8.8 o 1.19) para garantizar la compatibilidad determinista en las operaciones de lógica de juego y en el cálculo de matrices afines.
2. **Alineación de Memoria y Conversión de Endianness:** La emulación debe replicar la alineación de memoria del ARM7. Las lecturas desalineadas de 32 bits en memoria deben aplicar la rotación de datos (*barrel shifter*) correspondiente al hardware ARM original. Si el sistema anfitrión no utiliza la convención *Little-Endian*, se debe implementar una conversión de orden de bytes (*byte-swapping*) mediante llamadas intrínsecas del compilador en cada acceso a la RAM virtual.
3. **Sincronización Estricta de Estados de Espera (*****Waitstates*****):** La CPU emulada debe contabilizar la latencia según la región de memoria accedida. Las lecturas/escrituras en IWRAM consumen 1 ciclo virtual, mientras que los accesos a EWRAM o la ROM de cartucho deben añadir los estados de espera (*waitstates*) programados en el registro `WAITCNT` (`0x04000204`), ajustando las penalizaciones de ciclos en accesos secuenciales (S) y no secuenciales (N).
4. **Sincronización por Línea de Escaneo (*****Line-Buffer Sync*****):** El motor gráfico del emulador debe procesar el cuadro línea por línea (*scanline-based*) en sincronía con el contador `REG_VCOUNT` (`0x04000006`) en lugar de renderizar la pantalla completa en un único bloque al final del cuadro. Este comportamiento es imprescindible para dar soporte a juegos que modifican registros de desplazamiento de fondo (`BGxHOFS`), registros de paleta o matrices afines mediante interrupciones `IRQ_HBLANK` a mitad del dibujado del cuadro.

El cumplimiento estricto de las especificaciones de bus, restricciones de memoria, mapeos de registros y sincronizaciones de rasterizado detallados en este manual constituye la especificación técnica de referencia para el desarrollo de software, la ingeniería inversa y la emulación de ciclo preciso de la arquitectura Game Boy Advance.
