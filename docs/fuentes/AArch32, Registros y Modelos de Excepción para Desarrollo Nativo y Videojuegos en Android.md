# Manual de Arquitectura de Ejecución ARMv8-A: Estados AArch64/AArch32, Registros y Modelos de Excepción para Desarrollo Nativo y Videojuegos en Android

## 1. Fundamentos de los Estados de Ejecución en ARMv8-A (AArch64 vs. AArch32)

La introducción de la arquitectura ARMv8-A representó una transformación de fondo en la ingeniería de sistemas embebidos y de cómputo móvil. Para los arquitectos de motores de juego y desarrolladores de librerías nativas en C/C++ sobre el Android NDK, dominar la dualidad entre los estados de ejecución AArch64 y AArch32 resulta fundamental para maximizar el rendimiento computacional, mitigar latencias en el Bucle de Juego (*Game Loop*) y garantizar una transición fluida entre sistemas de 32 y 64 bits. Esta dualidad arquitectónica no constituye un mero mecanismo de compatibilidad hacia atrás; define la manera en que el procesador administra el ancho de banda del subsistema de memoria, ejecuta código vectorizado en el *pipeline*, maneja el espacio de direccionamiento virtual y procesa algoritmos matemáticos intensivos como la simulación física, la cinemática inversa y la transformación de vértices en tiempo real.

### 1.1 Modos de Ejecución y Conjuntos de Instrucciones

La arquitectura ARMv8-A formaliza dos estados operativos fundamentales a nivel de hardware: **AArch64** y **AArch32**.

- **AArch64**: Es el estado de ejecución nativo de 64 bits. Implementa exclusivamente el conjunto de instrucciones **A64**, caracterizado por instrucciones de longitud fija de 32 bits con una codificación regular y simétrica. Proporciona acceso directo al banco ampliado de registros de 64 bits y a un espacio de direccionamiento virtual masivo, optimizando la capacidad del compilador para programar instrucciones dinámicamente y reducir la presión sobre los registros (*register pressure*).
- **AArch32**: Es el estado de ejecución de 32 bits destinado a conservar compatibilidad binaria estricta con la arquitectura ARMv7-A previa. En este estado, el procesador puede ejecutar dos conjuntos de instrucciones distintos: **A32** (el conjunto tradicional ARM con instrucciones de 32 bits de ancho) y **T32** (tecnología Thumb-2, que intercala instrucciones de 16 y 32 bits para optimizar la densidad de código en memoria).

| Estado de Ejecución | Ancho de Registros General | Conjuntos de Instrucciones Compatibles | Ancho de Punteros | Límite de Espacio de Direccionamiento Virtual                                                                  |
| ------------------- | -------------------------- | -------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------- |
| **AArch64**         | 64 bits                    | A64                                    | 64 bits           | Hasta 2^{48} bytes (48 bits / 256 TB por espacio, extensible mediante configuraciones del grano de traducción) |
| **AArch32**         | 32 bits                    | A32, T32 (Thumb-2)                     | 32 bits           | 4 GB (2^{32} bytes, limitado por el ancho de puntero de 32 bits)                                               |

### 1.2 Reglas Operativas y Mecanismos para el Cambio de Estado

El paso de un estado de ejecución a otro está sujeto a restricciones arquitectónicas severas del hardware, diseñadas para preservar el aislamiento de procesos y evitar inconsistencias en el estado del procesador:

1. **Mecanismo de Cambio de Estado mediante Niveles de Excepción**: La conmutación entre AArch64 y AArch32 **solo** puede materializarse mediante un cambio explícito en el Nivel de Excepción (*Exception Level*, EL). Un motor de ejecución pasa de AArch32 a AArch64 cuando se genera una excepción que es capturada por un Nivel de Excepción superior configurado en AArch64. Por el contrario, la transición de AArch64 a AArch32 solo se produce cuando el procesador ejecuta una instrucción de retorno de excepción (`ERET`) hacia un Nivel de Excepción inferior configurado explícitamente para operar en AArch32.
2. **Imposibilidad Técnica de Interworking Directo**: A diferencia de lo que ocurre en AArch32 entre los conjuntos A32 y T32 (donde la instrucción `BX` realiza un salto e *interworking* cambiando el bit de estado T), es técnicamente imposible realizar llamadas directas o entrelazar código de 32 bits y 64 bits dentro del mismo espacio de proceso o Nivel de Excepción. Una función C/C++ compilada para A64 no puede invocar de forma directa una subrutina en A32/T32; cualquier interacción requiere un cambio de contexto conducido por el sistema operativo mediante llamadas al sistema o trampas de excepción.
3. **Restricciones Jerárquicas del Sistema**: Un sistema operativo de 64 bits que se ejecuta en AArch64 dentro de `EL1` puede albergar simultáneamente aplicaciones de usuario de 64 bits (AArch64) y de 32 bits (AArch32) en `EL0`. Sin embargo, la jerarquía arquitectónica impone una regla infranqueable: **un sistema operativo de 32 bits (AArch32 en** `**EL1**`**) no puede hospedar ni ejecutar aplicaciones de 64 bits (AArch64 en** `**EL0**`**)**, del mismo modo que un hipervisor de 32 bits en `EL2` no puede gestionar sistemas operativos huéspedes de 64 bits.

### 1.3 Impacto Estratégico en Ports Nativos de Android y Juegos ("Capa So What?")

El salto de motores de juego (como Unity, Unreal Engine o motores propietarios C++) desde AArch32 hacia AArch64 altera de manera substancial la eficiencia computacional y la distribución del consumo energético en dispositivos Android:

- **Ganancias Directas de Rendimiento**:
  - *Duplicación del Banco de Registros*: AArch64 expone 31 registros de propósito general de 64 bits en comparación con los 16 registros de 32 bits de AArch32 (de los cuales 3 estaban reservados de forma nativa para el *Program Counter*, *Stack Pointer* y *Link Register*). Esto elimina la necesidad de volcar datos temporales a la pila (*stack spilling*), permitiendo que los algoritmos de sombreado por software (*software rasterization*), culling de oclusión y evaluación de árboles de comportamiento de IA mantengan sus variables calientes directamente en los registros.
  - *Aritmética A64 Nativa de 64 bits*: Las operaciones con datos de 64 bits (`long long`, punteros de 64 bits, enteros de precisión doble) se ejecutan en un único ciclo de reloj mediante instrucciones nativas A64. En AArch32, estas operaciones requerían pares de registros (`R0:R1`), instrucciones concatenadas de propagación de acarreo (`ADDS`/`ADC`) ymúltiples ciclos de canalización.
  - *Paso de Parámetros Extendido (AAPCS64)*: La convención de llamadas de AArch64 permite pasar hasta 8 parámetros por registro (`X0-X7`) para enteros/punteros y 8 para punto flotante/SIMD (`V0-V7`). Esto reduce drásticamente la latencia de función en subrutinas hiper-frecuentes como el cálculo de matrices de transformación matemática en el bucle principal de renderizado.
- **Compromisos Técnicos e Impacto en la Memoria**:
  - *Expansión de Punteros y Huella de Memoria*: En AArch64, todo puntero consumirá 8 bytes en lugar de 4. En estructuras de datos densas del motor —tales como grafos de escena (*scene graphs*), listas enlazadas de componentes ECS (*Entity Component System*) o buffers de nodos de renderizado—, la huella de memoria para almacenar la topología del grafo se duplica.
  - *Presión sobre las Cachés de Datos (L1D/L2)*: La expansión de punteros reduce la densidad de información empaquetada dentro de una línea de caché (habitualmente de 64 bytes). Si el motor no reorganiza sus datos mediante arreglos de estructuras (*Structure of Arrays*, SoA) en lugar de estructuras de arreglos (*Array of Structures*, AoS), la tasa de fallos de caché de datos (*cache misses*) se incrementará, provocando paradas en la canalización (*pipeline stalls*) por latencia de DRAM que merman el presupuesto de cuadro (*frame budget*) a 60 o 120 FPS.

Comprender la separación estructural entre los estados de ejecución y sus consecuencias operativas exige estudiar detalladamente la organización interna del banco de registros que el procesador pone a disposición en cada estado.

## 2. Arquitectura de Registros: Propósito General, Especiales y de Sistema

El diseño del banco de registros en la arquitectura ARMv8-A reorganiza la interacción entre la CPU y los compiladores de C/C++. Un hito fundamental en AArch64 es la **eliminación del acceso directo al Program Counter (**`**PC**`**) como un registro de propósito general**. En arquitecturas de 32 bits como ARMv7, el `PC` era el registro `R15`, lo que permitía escrituras directas sobre el `PC` mediante operaciones aritméticas o de carga (`MOV PC, LR` o `LDR PC, [SP]`), dificultando enormemente la predicción de saltos (*branch prediction*) y complicando el diseño de procesadores superescalares con ejecución fuera de orden (*Out-of-Order execution*). En AArch64, el `PC` es un registro especial independiente, lo que simplifica la canalización de hardware y elimina cuellos de botella en la fase de decodificación.

### 2.1 Registros de Propósito General (X0-X30 / W0-W30) y Mapeo Inter-Estado

En el estado AArch64, el procesador expone 31 registros generales de 64 bits, denominados `X0` a `X30`.

- **Visión Wn vs. Xn**: Cada registro de 64 bits `Xn` puede ser accedido a través de un alias de 32 bits denominado `Wn`, el cual representa exactamente los 32 bits menos significativos (bits [31:0]) del registro `Xn` correspondiente.
  - *Lectura*: Leer desde un registro `Wn` retorna el valor contenido en los 32 bits inferiores de `Xn`, ignorando los 32 bits superiores [63:32].
  - *Comportamiento Estricto de Escritura*: **Toda escritura en un registro** `**Wn**` **pone a cero automáticamente los 32 bits superiores (bits [63:32]) del registro** `**Xn**` **asociado.** Si el registro `X0` contiene `0xFFFFFFFFFFFFFFFF` y se ejecuta la instrucción `MOV W0, #1`, el valor final de `X0` será `0x0000000000000001`. Este comportamiento fue diseñado específicamente para eliminar dependencias parciales de registros (*partial register stalls*) en microarquitecturas superescalares.
- **El Registro Zero (**`**XZR**` **/** `**WZR**`**)**: El índice 31 en las codificaciones de instrucciones A64 representa el Registro Cero. Cuando se lee `XZR` (64 bits) o `WZR` (32 bits), el valor devuelto es siempre cero absoluto. Toda escritura dirigida a `XZR`/`WZR` es ignorada y descartada por el hardware. Esto permite simplificar el conjunto de instrucciones al evitar la necesidad de opcodes dedicados para operaciones comunes (por ejemplo, la instrucción `NOP` se codifica frecuentemente como `MOV XZR, XZR`, e inicializar regiones de memoria a cero se realiza mediante instrucciones de par de almacenamiento `STP XZR, XZR, [X0]`).
- **Mapeo Inter-Estado (Conmutación Física AArch64** **\rightarrow** **AArch32)**: Cuando el procesador conmuta su ejecución al estado AArch32, los registros de 32 bits de ARMv7 se asocian de manera directa y transparente sobre los registros físicos subyacentes de AArch64. Los registros mapeados (*banked registers*) de los modos privilegiados de AArch32 se vinculan físicamente al espacio no utilizado directamente por la ejecución AArch32 estándar (`X16-X30`), asegurando que el estado del procesador se conserve sin necesidad de volcados excesivos a memoria.

| Registro de AArch32 / Modo  | Registro Físico Mapeado en AArch64 | Descripción del Contenido Preservado                          |
| --------------------------- | ---------------------------------- | ------------------------------------------------------------- |
| `R0` - `R12` (User/Sys)     | `W0` - `W12`                       | Bits [31:0] de los registros generales `X0-X12`.              |
| `SP` (`R13`) / `LR` (`R14`) | `W13` / `W14`                      | Stack Pointer y Link Register del modo User/System.           |
| `R8_fiq` - `R12_fiq`        | `W24` - `W28`                      | Registros bancados dedicados para interrupciones rápidas FIQ. |
| `SP_fiq` / `LR_fiq`         | `W29` / `W30`                      | Stack Pointer y Link Register del modo FIQ.                   |
| `SP_irq` / `LR_irq`         | `W16` / `W17`                      | Stack Pointer y Link Register del modo IRQ.                   |
| `SP_svc` / `LR_svc`         | `W18` / `W19`                      | Stack Pointer y Link Register del modo Supervisor (SVC).      |
| `SP_abt` / `LR_abt`         | `W20` / `W21`                      | Stack Pointer y Link Register del modo Abort (ABT).           |
| `SP_und` / `LR_und`         | `W22` / `W23`                      | Stack Pointer y Link Register del modo Undefined (UND).       |
| `SP_hyp` / `LR_hyp`         | `W15` / `R14` (AArch32)            | Stack Pointer y Link Register del modo Hipervisor (HYP).      |
| `SP_mon` / `LR_mon`         | `W15` / `W14` (según contexto)     | Stack Pointer y Link Register del modo Monitor (MON).         |

*Nota: Durante la ejecución en AArch32, los 32 bits superiores (bits [63:32]) de todos los registros* `*X0-X30*` *de AArch64 se vuelven inaccesibles pero mantienen su estado intacto hasta el retorno a AArch64.*

### 2.2 Registros Especiales y Procesamiento de Estado (PSTATE)

AArch64 descarta el uso del registro global CPSR (*Current Program Status Register*) presente en ARMv7 y lo sustituye por una abstracción denominada **PSTATE** (*Processor State*). PSTATE representa un conjunto de campos independientes de estado del procesador a los que se puede acceder individualmente mediante instrucciones `MSR`/`MRS` o de forma implícita mediante banderas de condición.

```
                  Estructura de Campos de PSTATE en AArch64
+---+---+---+---+---+---+---+---+---+---+-----+----+----+
| N | Z | C | V | SS| IL| D | A | I | F | nRW | EL | SP |
+---+---+---+---+---+---+---+---+---+---+-----+----+----+
```

- **Banderas de Condición Aritmética (ALU)**:
  - `N` (*Negative*): Se establece en 1 si el resultado de una operación ALU es negativo.
  - `Z` (*Zero*): Se establece en 1 si el resultado de la operación es cero.
  - `C` (*Carry*): Se establece en 1 si la operación genera un acarreo de salida (*carry-out*) en adiciones o un no-prestado en sustracciones.
  - `V` (*oVerflow*): Se establece en 1 si la operación provoca un desbordamiento aritmético en complemento a dos.
- **Campos de Control y Máscaras de Excepción**:
  - `SS` (*Software Step*): Utilizado por depuradores para habilitar el paso a paso de instrucciones.
  - `IL` (*Illegal Execution State*): Bit de estado ilegal; si se establece en 1, cualquier intento de ejecutar una instrucción genera una excepción de estado ilegal.
  - `D`, `A`, `I`, `F` (*Interrupt Mask Bits*): Máscaras de depuración (`D`), errores asincrónicos del sistema SError (`A`), interrupciones IRQ (`I`) e interrupciones rápidas FIQ (`F`). Si el bit es 1, la interrupción correspondiente está enmascarada (deshabilitada).
- **Campos de Estado Operativo**:
  - `nRW` (*Execution State*): **Indica el estado de ejecución actual. Un valor de** `**0**` **denota ejecución en AArch64; un valor de** `**1**` **denota ejecución en AArch32.**
  - `EL` (*Exception Level*): Almacena el Nivel de Excepción actual (valores `00` para EL0, `01` para EL1, `10` para EL2, y `11` para EL3).
  - `SP` (*Stack Pointer Selector*): Bit selector de pila. Un valor `0` indica el uso de `SP_EL0`; un valor `1` indica el uso del Stack Pointer propio del nivel actual (`SP_ELn`).

Entre los **Registros Especiales** clave de AArch64 destacan:

- **Stack Pointer (**`**SP**` **/** `**SP_ELn**`**)**: AArch64 ofrece Stack Pointers bancados e independientes para cada Nivel de Excepción (`SP_EL0`, `SP_EL1`, `SP_EL2`, `SP_EL3`).
- **Program Counter (**`**PC**`**)**: Mantiene la dirección de memoria de la instrucción en ejecución. No es direccionable como registro fuente o destino en operaciones aritméticas o de carga general (`ADD`, `MOV`). Solo puede modificarse implícitamente mediante instrucciones de salto (`B`, `BL`, `BR`), retornos (`RET`, `ERET`) o direccionamiento relativo al PC (`ADR`, `ADRP`).
- **Exception Link Register (**`**ELR_ELn**`**)**: Almacena la dirección de retorno preferida a la cual debe volver la ejecución cuando se completa el procesamiento de una excepción en el nivel `ELn`.
- **Saved Program Status Register (**`**SPSR_ELn**`**)**: Almacena de forma atómica una copia completa de PSTATE inmediatamente antes de tomar una excepción hacia `ELn`. Al ejecutar `ERET`, el procesador restaura PSTATE copiando `SPSR_ELn` de vuelta a los campos operativos.

### 2.3 Registros de Control del Sistema (SCTLR, TTBR, TCR, VBAR)

La configuración del procesador, la gestión de memoria MMU y el control de dispositivos en AArch64 se realizan a través de Registros de Sistema dedicados, accedidos mediante las instrucciones `MRS` (copiar de registro de sistema a registro general `Xn`) y `MSR` (escribir valor de registro general `Xn` en registro de sistema). La nomenclatura incluye el sufijo `_ELn`, especificando el nivel mínimo de privilegio requerido para su acceso:

- `**SCTLR_EL1/2/3**` **(System Control Register)**: Controla las funciones arquitectónicas esenciales del núcleo en el Nivel de Excepción correspondiente. Administra la activación de la MMU (bit `M`), la habilitación de las cachés de datos e instrucciones (bits `C` e `I`), el control de alineación de la pila (bits `SA` y `SA0`), y la configuración de *Endianness* (bit `EE` para el nivel actual y `E0E` para EL0 en data accesses).
- `**TTBR0_EL1**` **y** `**TTBR1_EL1**` **(Translation Table Base Registers)**: En AArch64, el espacio de direcciones virtuales en `EL0/EL1` se divide explícitamente en dos regiones:
  - `TTBR0_EL1`: Almacena la dirección física base de la tabla de traducción para el espacio de direcciones virtuales inferior (*User Space* / `0x0000000000000000` a `0x0000FFFFFFFFFFFF`), accesible por aplicaciones en `EL0` y el Kernel.
  - `TTBR1_EL1`: Almacena la dirección física base de la tabla de traducción para el espacio de direcciones virtuales superior (*Kernel Space* / `0xFFFF000000000000` a `0xFFFFFFFFFFFFFFFF`), reservado exclusivamente para el Kernel en `EL1`. Esta división física evita la necesidad de descargar las entradas de la TLB (*Translation Lookaside Buffer*) del espacio de usuario durante las llamadas al sistema o cambios de contexto.
- `**TCR_EL1**` **(Translation Control Register)**: Determina el tamaño del direccionamiento virtual útil, el tamaño de página o grano de traducción (4 KB, 16 KB o 64 KB) y los atributos de compartibilidad de memoria para los recorridos de tablas de páginas (*page table walks*).
- `**VBAR_EL1/2/3**` **(Vector Base Address Register)**: Contiene la dirección virtual base de la tabla de vectores de excepción para el Nivel de Excepción respectivo.

### 2.4 Registros SIMD y de Punto Flotante (NEON / V0-V31)

AArch64 incluye un banco ampliado de **32 registros SIMD y de Punto Flotante de 128 bits**, denominados `V0` a `V31`.

A diferencia de la arquitectura AArch32 —donde dos registros `D` de 64 bits se empaquetaban consecutivamente dentro de un registro `Q` de 128 bits, creando complejas dependencias cruzadas en el hardware—, en AArch64 **el mapeo es directo, vertical y no solapado entre registros distintos**. Las vistas escalares representan simplemente la porción de bits menos significativos del mismo registro vectorial `Vn`:

```
Estructura Mapeada y Directa del Banco Vectorial AArch64 (V0-V31)
127                                                            0
+--------------------------------------------------------------+
|                         V0 (128 bits)                        |
+--------------------------------------------------------------+
|               Q0 (128 bits / Quadword)                       |
+-----------------------------------------------+--------------+
                                                | D0 (64 bits) |
                                                +-------+------+
                                                        | S0   |
                                                        +--+---+
                                                           |H0 |
                                                           +---+
                                                           |B0 |
                                                           +---+
```

- `Q0-Q31`: Vista completa de 128 bits (Vector o escalar amplio).
- `D0-D31`: Bits [63:0] (Punto flotante de doble precisión de 64 bits o enteros de 64 bits).
- `S0-S31`: Bits [31:0] (Punto flotante de precisión simple de 32 bits o enteros de 32 bits).
- `H0-H31`: Bits [15:0] (Punto flotante de media precisión de 16 bits o enteros de 16 bits).
- `B0-B31`: Bits [7:0] (Entero de 8 bits).

**Comparativa Arquitectónica de Mapeo (AArch64 vs. AArch32)**: En AArch32, la instrucción `Q0` agrupaba físicamente a `D0` y `D1`, y `D0` agrupaba a `S0` y `S1`. Por ende, modificar el registro escalar `S1` en AArch32 alteraba los bits superiores de `D0` y destruía parcialmente el contenido de `Q0`. Esto creaba dependencias de bucle (*loop-carried dependencies*) que impedían a los compiladores autovectorizar el código eficientemente. En AArch64, `S0` es los 32 bits inferiores de `V0`, y `S1` es los 32 bits inferiores de `V1`. Modificar `S1` no afecta a `V0` en absoluto.

**Optimizaciones Clave para Motores de Renderizado y Física**:

1. **Reducción Vectorial Cruzada (*****Cross-Lane Operations*****)**: AArch64 añade instrucciones nativas de reducción que procesan horizontalmente todos los elementos dentro de un único registro vectorial `Vn` y devuelven un resultado escalar en un solo ciclo (e.g., `ADDV` para suma acumulada, `FMINNM`/`FMAXNM` para búsqueda instantánea del mínimo/máximo respetando la norma IEEE 754-2008). Esto elimina decenas de instrucciones de desempaquetado (*unpacking*) y permutación (*shuffling*) al calcular iluminaciones, productos escalares o límites de cajas de colisión (*AABB Bounding Boxes*).
2. **Operaciones Fusión Multiplicación-Adición Nativa (**`**FMADD**`**)**: Realizan la operación R = A \times B + C con un único paso de redondeo al final, incrementando la precisión matemática y acelerando el cálculo de transformaciones matriciales 4\times4, algoritmos de piel animada (*skeletal animation*) y física de partículas.

La disponibilidad y los privilegios de acceso a todos estos registros están gobernados estrictamente por la jerarquía de los Niveles de Excepción.

## 3. Modelo de Niveles de Excepción (EL) y Gestión de Privilegios

La arquitectura ARMv8-A implementa un modelo de protección jerárquico basado en cuatro **Niveles de Excepción** (*Exception Levels*), codificados numéricamente desde `EL0` hasta `EL3`. Este diseño impone un aislamiento estricto de software, garantizando que el código de aplicaciones no privilegiado no pueda interferir con las operaciones críticas del kernel, los hipervisores o el firmware del sistema.

```
          Jerarquía de Niveles de Excepción en ARMv8-A
+---------------------------------------------------------------+
| Nivel de Excepción   Grado de Privilegio   Mapeo Típico       |
+---------------------------------------------------------------+
|       EL3          |      PL3        | Secure Monitor / FW    |
|       EL2          |      PL2        | Hipervisor (KVM/Android|
|       EL1          |      PL1        | Kernel Linux / Android |
|       EL0          |      PL0        | App C/C++ (Android NDK)|
+---------------------------------------------------------------+
```

### 3.1 Jerarquía de Privilegios

Los Niveles de Excepción estructuran la ejecución en el ecosistema Android de la siguiente forma:

- `**EL0**` **(Usuario / No Privilegiado - PL0)**: Nivel donde se ejecutan las aplicaciones de usuario y el código binario nativo en C/C++ de los motores de juego mediante el NDK. No posee acceso a instrucciones privilegiadas de configuración ni a los registros del sistema `_EL1/2/3`.
- `**EL1**` **(Kernel del SO / Privilegiado - PL1)**: Nivel donde opera el Kernel de Linux/Android. Administra las tablas de paginación de la MMU, gestiona los controladores de dispositivos nativos (*drivers* de GPU, sonido, almacenamiento) y procesa excepciones provenientes de `EL0`.
- `**EL2**` **(Hipervisor / Virtualización - PL2)**: Nivel diseñado para la ejecución de monitores de máquinas virtuales e hipervisores (como KVM en Android). Administra la traducción de memoria de segunda etapa (*Stage 2 Translation*) para aislar sistemas operativos huéspedes.
- `**EL3**` **(Firmware / Secure Monitor - PL3)**: Máximo nivel de privilegio dentro del procesador. Ejecuta el *Secure Monitor* y el firmware de arranque inicial (*Bootloader*), administrando la conmutación entre los mundos de seguridad TrustZone.

**Reglas Invariables para el Cambio de Nivel**:

- **Tomar una excepción**: Incrementa el Nivel de Excepción o lo mantiene igual (la ejecución se desplaza obligatoriamente hacia un nivel numéricamente mayor o igual, e.g., de `EL0` a `EL1`).
- **Retornar de una excepción (**`**ERET**`**)**: Reduce el Nivel de Excepción o lo mantiene igual (la ejecución se desplaza hacia un nivel numéricamente menor o igual, e.g., de `EL1` a `EL0`).
- **Ausencia de Excepciones en** `**EL0**`: El nivel `EL0` no dispone de vectores de excepción propios; cualquier excepción producida en `EL0` debe ser atrapada y procesada en `EL1` o superior.

### 3.2 Clasificación de Excepciones: Sincrónicas vs. Asincrónicas

En ARMv8-A, las excepciones se dividen en dos categorías operativas:

1. **Excepciones Sincrónicas**: Son provocadas directamente por la ejecución o intento de ejecución de una instrucción en la canalización de la CPU. La dirección de retorno almacenada en `ELR_ELn` indica exactamente la instrucción que originó la excepción o la instrucción inmediatamente posterior.
   - *Tipos comunes*: Abortos de memoria MMU (fallos de traducción de página, violaciones de permiso de lectura/escritura), fallos de alineación de dirección en el `PC` o `SP`, instrucciones no asignadas (*Undefined Instructions*) y traps de instrucciones privilegiadas.
   - *Instrucciones Generadoras de Excepciones (Llamadas al Sistema)*:
     - `SVC` (*Supervisor Call*): Permite al código de usuario en `EL0` solicitar servicios al Kernel en `EL1`.
     - `HVC` (*Hypervisor Call*): Permite al Kernel en `EL1` solicitar servicios al Hipervisor en `EL2`.
     - `SMC` (*Secure Monitor Call*): Permite a software en `EL1/EL2` invocar los servicios del *Secure Monitor* en `EL3`.
2. **Excepciones Asincrónicas**: Ocurren independientemente del flujo de instrucciones en curso y son causadas por eventos externos al núcleo de la CPU.
   - `IRQ` (*Interrupt Request*): Interrupciones físicas de prioridad estándar generadas por periféricos (e.g., la GPU notificando la finalización del renderizado de un *frame* o el controlador táctil de la pantalla).
   - `FIQ` (*Fast Interrupt Request*): Interrupciones físicas de alta prioridad orientadas a transferencias de baja latencia.
   - `SError` (*System Error*): Errores asincrónicos del bus del sistema, provocados comúnmente por volcados diferidos de escrituras corruptas en cachés hacia la RAM.

### 3.3 Registros del Símbolo de Excepción y Retorno

Cuando el procesador atrapa una excepción sincrónica o un error del sistema en el nivel `ELn`, actualiza atómicamente tres registros clave de diagnóstico:

- `**ESR_ELn**` **(Exception Syndrome Register)**: Almacena el motivo de la excepción. Su campo `EC` (*Exception Class*, bits [31:26]) identifica la naturaleza del fallo (e.g., `0b100100` para Data Abort desde un EL inferior, `0b010101` para ejecución de `SVC`). El campo `ISS` (*Instruction Specific Syndrome*, bits [24:0]) suministra detalles específicos como el tipo de acceso a memoria que falló o el número inmediato codificado en la instrucción `SVC`.
- `**FAR_ELn**` **(Fault Address Register)**: Guarda la dirección virtual exacta que provocó el fallo en las excepciones de tipo aborto de instrucciones o aborto de datos. Si un puntero corrupto en C++ intenta acceder a `0x0000000000000008`, `FAR_EL1` almacenará dicho valor.
- `**ELR_ELn**` **(Exception Link Register)**: Contiene la dirección virtual de retorno preferida donde la CPU debe reanudar la ejecución tras invocar la instrucción `ERET`.

Este modelo de manejo de excepciones formaliza los límites operacionales para la división de dominios de seguridad mediante la tecnología ARM TrustZone.

## 4. Seguridad y Aislamiento de Dominios: Mundo Seguro vs. Mundo No Seguro (TrustZone)

La tecnología ARM TrustZone es una extensión de seguridad implementada a nivel de hardware que divide al procesador en dos entornos operacionales aislados: el **Mundo No Seguro** (*Normal World*) y el **Mundo Seguro** (*Secure World*). Esta separación física garantiza que ni siquiera un ataque con privilegios totales de Kernel en `EL1` dentro del Mundo No Seguro pueda comprometer los activos críticos del sistema (tales como claves criptográficas de DRM, datos bancarios, sensores biométricos o credenciales de la plataforma).

### 4.1 Arquitectura Hardware TrustZone y el Secure Monitor (`EL3`)

El aislamiento entre ambos mundos se propaga por todo el SoC mediante el bus del sistema (AMBA AXI), el cual incluye un bit de señalización de seguridad en cada canal de lectura y escritura.

```
          Topología de Mundos en ARM TrustZone

   MUNDO NO SEGURO                     MUNDO SEGURO
  (Normal World / Rich OS)            (Trusted OS / Firmware)
+-------------------------+         +-------------------------+
| EL0: Aplicaciones/Juegos|         | EL0: Apps Confiables    |
|      Android NDK        |         |      (Trusted Apps)     |
+-------------------------+         +-------------------------+
| EL1: Kernel Android     |         | EL1: Trusted OS         |
|      (Linux Kernel)     |         |      (OP-TEE, etc.)     |
+-------------------------+         +-------------------------+
| EL2: Hipervisor (KVM)   |         | (Nivel EL2 no existe    |
| (Solo en Normal World)  |         |  en Mundo Seguro)       |
+-------------------------+---------+-------------------------+
|                    EL3: Secure Monitor                      |
+-------------------------------------------------------------+
```

El **Secure Monitor**, ejecutado en el nivel `EL3` bajo el estado AArch64, es la pasarela central y única encargada de orquestar la conmutación entre el Mundo No Seguro y el Mundo Seguro. La transición solo puede ser provocada cuando software en `EL1/EL2` ejecuta una instrucción `SMC` (*Secure Monitor Call*) o cuando el hardware redirige una interrupción física específicamente asignada al dominio seguro hacia `EL3`.

### 4.2 Mapeo de Niveles de Excepción en Ambos Mundos

La topología de Niveles de Excepción en la arquitectura ARMv8-A se distribuye de manera asimétrica entre ambos mundos:

- **Mundo No Seguro (Normal World)**:
  - `EL0`: Entorno de ejecución de aplicaciones Android de usuario y librerías C/C++ de motores de renderizado.
  - `EL1`: Entorno privilegiado del Kernel de Linux/Android.
  - `EL2`: Entorno de hipervisor para virtualización.
- **Mundo Seguro (Secure World)**:
  - `EL0`: Entorno para Aplicaciones Confiables (*Trusted Applications* / TAs), como módulos de procesamiento DRM de video o autenticación biométrica.
  - `EL1`: Entorno del Sistema Operativo Confiable (*Trusted OS*, e.g., OP-TEE o QSEE).
  - `EL2`: **Ausente**. En la arquitectura descrita no existe una capa de hipervisor dentro del Mundo Seguro.
  - `EL3`: Nivel supremo asignado al *Secure Monitor* y al firmware de bajo nivel de la plataforma.

### 4.3 Impacto en el Desarrollo de Juegos y Agentes de IA ("Capa So What?")

Las llamadas al dominó Seguro introducen sobrecostos de rendimiento críticos que deben gestionarse en proyectos nativos:

- **Latencia de Conmutación de Entorno**: Invocar una instrucción `SMC` requiere purgar las etapas de precaptura de la canalización de la CPU, guardar la totalidad del banco de registros generales en la pila del Monitor, cambiar el estado de canal del bus AXI a Seguro y actualizar los contextos de MMU en `EL3`. Este cambio de contexto puede consumir órdenes de magnitud más ciclos de reloj que una simple llamada al sistema `SVC`.
- **Regla de Arquitectura para Motores de Juego**: Los subsistemas de renderizado, cálculo de físicas o agentes de IA en motores de juego **jamás deben ejecutar llamadas** `**SMC**` **ni depender de validaciones criptográficas o llamadas a servicios del Mundo Seguro dentro del Bucle de Juego (*****Game Loop*****)**. Las comprobaciones de licencias por DRM, la desencripción de assets de audio/video o el intercambio de claves de red deben aislarse estrictamente durante las pantallas de carga o fases de inicialización, evitando así micro-paradas (*micro-stuttering*) o caídas en la tasa de cuadros por segundo.

Saber interactuar con el modelo de ejecución y los niveles de privilegio del hardware permite aplicar normas rigurosas de optimización a nivel de código C/C++ bajo la arquitectura AArch64.

## 5. Guía Práctica de Portabilidad y Bajo Nivel para Desarrollo Nativo y Android

Migrar o desarrollar motores de juego y componentes de bajo nivel C/C++ en AArch64 requiere alinearse estrictamente con la convención de llamadas oficial de la arquitectura, gestionar la expansión de tipos de datos e implementar primitivas de atomicidad y sincronización de memoria de forma adecuada.

### 5.1 Convención de Llamadas (AAPCS64) y Manejo de Registros

El estándar *Procedure Call Standard for the ARM 64-bit Architecture* (AAPCS64) rige el uso y conservación de registros entre funciones subrutinas:

- **Registros de Parámetros y Resultados (**`**X0-X7**` **/** `**V0-V7**`**)**: Se emplean para pasar los primeros 8 parámetros escalares o punteros (`X0-X7`) y los primeros 8 parámetros SIMD/Punto Flotante (`V0-V7`). Los valores de retorno de funciones también se devuelven en `X0`/`V0` (o `X0-X1` para tipos que ocupan hasta 16 bytes).
- **Registro de Resultado Indirecto (**`**X8**`**)**: Contiene la dirección de memoria reservada por el llamador (*caller*) cuando una función retorna una estructura de datos mayor a 16 bytes (`struct` devuelta por valor).
- **Registros Temporales Salvados por el Llamador (**`**X9-X15**` **/** ***Caller-saved*****)**: Registros de trabajo temporal. Si la función llamadora necesita preservar sus datos a través de una llamada a otra subrutina, debe guardarlos manualmente en su propio marco de pila (*stack frame*).
- **Registros de Arañamiento Interno del Linker (**`**X16-X17**` **/** `**IP0-IP1**`**)**: Registros rascadores intra-procedimiento (*Intra-procedure-call scratch registers*). Son utilizados por el *linker* para insertar código intermedio de salto denominado **Veneers**.
- **Registro de Plataforma (**`**X18**`**)**: Reservado para usos específicos del sistema operativo (en Android, almacena punteros de estado por hilo o *Shadow Call Stack*).
- **Registros Salvados por el Subprograma (**`**X19-X28**` **/** ***Callee-saved*****)**: Registros que deben ser preservados obligatoriamente por la función llamada. Si una subrutina los utiliza, debe respaldarlos en la pila en su prólogo y restaurarlos en su epílogo. Para el banco SIMD (`V8-V15`), la norma AAPCS64 exige preservar **únicamente la mitad inferior de 64 bits (los bits [63:0] de cada registro** `**Dn**`**)**; los 64 bits superiores de `V8-V15` son calificados como *caller-saved*.
- **Frame Pointer (**`**X29**` **/ FP)**: Registro que apunta a la base del marco de pila actual, esencial para el rastreo de llamadas (*stack unwinding*) en tareas de depuración y perfilado de rendimiento.
- **Link Register (**`**X30**` **/ LR)**: Guarda la dirección de retorno a la cual debe volver la ejecución tras completar la subrutina actual.

```
                  Estructura del Stack Frame en AAPCS64
                  +-----------------------------------+
                  | Parámetros excedentes en pila     |
                  +-----------------------------------+
                  | Link Register (X30 / LR)          |
  Frame Pointer ->+-----------------------------------+
     (X29 / FP)   | Frame Pointer anterior (X29)      |
                  +-----------------------------------+
                  | Registros callee-saved (X19-X28)  |
                  +-----------------------------------+
                  | Variables locales y temporales    |
  Stack Pointer ->+-----------------------------------+ (Alineado estrictamente
     (SP)                                               a límites de 16 bytes)
```

#### Alerta de Rendimiento: Latencia de Veneers en Bucles del Motor

Cuando un objetivo de salto incondicional mediante la instrucción `B` o `BL` excede el rango de direccionamiento codificable en la instrucción (\pm128\text{ MB}), el *linker* inserta un **Veneer** (una pequeña secuencia de código intermedia que carga la dirección completa de 64 bits en `X16`/`X17` y efectúa un salto indirecto `BR X16`). **Si un bucle crítico de física o renderizado efectúa llamadas a funciones ubicadas en otras secciones de memoria a través de Veneers, provocará múltiples paradas en la canalización (*****pipeline flushes*****) y contaminación masiva en la caché de instrucciones (I-Cache).** Se debe estructurar la ubicación del código en memoria asegurando que las funciones llamadas en bucles densos permanezcan dentro del radio de \pm128\text{ MB}.

#### Regla Invariable de Alineación de Pila

En AArch64, la CPU realiza una comprobación física de alineación del *Stack Pointer*. **El registro** `**SP**` **debe estar estrictamente alineado a límites de 16 bytes (128 bits) en todo momento en que sea utilizado como dirección base para accesos a memoria.** Si se ejecuta una instrucción `LDR` o `STR` utilizando un `SP` desalineado (por ejemplo, con bits `SP[3:0] != 0b0000`), el hardware disparará de inmediato una excepción sincrónica de alineación (*Stack Alignment Fault*).

### 5.2 Modelo de Datos LP64 y Errores Comunes de Portabilidad C/C++

El entorno binario nativo de 64 bits para Android se rige bajo el modelo de datos **LP64**. En este modelo, los tipos `int` se mantienen en 32 bits, mientras que los tipos `long` y los **punteros** se expanden a 64 bits.

| Tipo de Dato C/C++        | AArch32 (ILP32 / ABI)          | AArch64 (LP64 / ABI)           |
| ------------------------- | ------------------------------ | ------------------------------ |
| `char`                    | 8 bits (sin signo por defecto) | 8 bits (sin signo por defecto) |
| `short`                   | 16 bits                        | 16 bits                        |
| `int`                     | 32 bits                        | 32 bits                        |
| `long`                    | 32 bits                        | **64 bits**                    |
| `long long`               | 64 bits                        | 64 bits                        |
| `pointer` (e.g., `void*`) | 32 bits                        | **64 bits**                    |
| `size_t` / `ptrdiff_t`    | 32 bits                        | **64 bits**                    |

#### Errores Críticos de Portabilidad y Ejemplos de Código

##### 1. Cast Prohibido de Puntero a Entero de 32 bits

```cpp
// ERROR CRÍTICO EN AArch64: Truncamiento de dirección virtual
void* nodeAddress = getSceneNode();
int ptrValue = (int)nodeAddress; // TRUNCA LOS 32 BITS SUPERIORES (BOOM!)
void* corruptPtr = (void*)ptrValue;

// SOLUCIÓN CORRECTA:
uintptr_t safePtrValue = (uintptr_t)nodeAddress; // Mantiene los 64 bits completos
```

##### 2. Aritmética de Punteros e Índices de Arreglos

Usar `int` (32 bits con signo) para recorrer arreglos masivos de vértices o buffers de texturas que superen la frontera de los 2 GB provoca desbordamientos numéricos. Se debe emplear sistemáticamente `size_t` para indexación de arreglos y `ptrdiff_t` para la diferencia de punteros.

##### 3. Penalizaciones por Extensión de Signo (`SXTW`) en Bucles Internos

Mezclar índices enteros con signo de 32 bits (`int`) con punteros de 64 bits en expresiones de direccionamiento obliga al compilador a promover implícitamente el índice de 32 bits a 64 bits mediante la instrucción `SXTW` (*Sign-Extend Word*).

```cpp
// C++: Mezcla de int (32-bit con signo) con puntero de 64-bit
void ProcessParticles(float* __restrict positions, int count) {
    for (int i = 0; i < count; i++) {
        positions[i] *= 0.98f;
    }
}
```

El compilador emitirá el siguiente ensamblador A64 dentro del bucle interno:

```assembly
.Lloop:
    sxtw    x3, w2                // Inyección extra de ciclos: Sign-extend de 'i' (W2) a X3
    ldr     s0, [x0, x3, lsl #2]  // Direccionamiento base X0 + (X3 << 2)
    fmul    s0, s0, s1
    str     s0, [x0, x3, lsl #2]
    add     w2, w2, #1
    cmp     w2, w1
    b.lt    .Lloop
```

**Optimización**: Declarar el índice del bucle como `size_t` o `size_t i` elimina la instrucción `SXTW`, ahorrando latencia en cada iteración del bucle de física o partículas.

### 5.3 Accesos a Memoria, Alineación e Instrucciones de Barrera

En AArch64, el hardware soporta accesos a memoria no alineados (*unaligned memory accesses*) para operaciones escalares estándar de carga y almacenamiento (`LDR`/`STR`) dirigidas a regiones de memoria marcadas como **Normal Memory**. El procesador gestiona el acceso mediante múltiples ciclos de bus transparentes. Sin embargo, existen excepciones absolutas donde el desalineamiento provoca fallos fatales de alineación (*Alignment Fault*):

1. **Operaciones Exclusivas (**`**LDXR**`**/**`**STXR**`**) y de Ordenamiento (**`**LDAR**`**/**`**STLR**`**)**: Deben estar alineadas obligatoriamente a la dimensión natural de sus datos (e.g., accesos de 64 bits alineados a direcciones múltiplos de 8 bytes).
2. **Memoria de Dispositivos (*****Device Memory*****)**: Todo acceso a regiones mapeadas de hardware e/s o registros MMIO debe ser naturalmente alineado.

#### Instrucciones de Barrera de Memoria

AArch64 utiliza un modelo de memoria débilmente ordenado (*Weakly-Ordered Memory Model*). Las lecturas y escrituras pueden reordenarse en la canalización para maximizar el rendimiento, excepto cuando se insertan barreras explícitas:

- `**DMB**` **(Data Memory Barrier)**: Garantiza que todas las operaciones de memoria (lecturas y escrituras) anteriores en orden de programa sean observadas globalmente antes de que se emita cualquier acceso posterior. No detiene la ejecución de instrucciones no relacionadas con memoria.
- `**DSB**` **(Data Synchronization Barrier)**: Suspende la ejecución de *todas* las instrucciones subsiguientes hasta que las operaciones de memoria previas, el mantenimiento de cachés y las invalidaciones de la TLB hayan completado de forma absoluta.
- `**ISB**` **(Instruction Synchronization Barrier)**: Vacía completamente la canalización del procesador y los búferes de precaptura, obligando a volver a buscar las instrucciones desde la caché o memoria RAM. Es obligatoria tras la modificación dinámica de código (e.g., compiladores JIT en motores de juego) o cambios en tablas de paginación MMU.

#### Semántica de Adquisición y Liberación (*Load-Acquire / Store-Release*)

ARMv8-A introduce barreras de memoria unidireccionales optimizadas para los modelos de memoria multihilo de C++11, C11 y Java:

- `**LDAR**` **(Load-Acquire)**: Lectura con semántica de adquisición. Ningún acceso a memoria (lectura o escritura) posterior en orden de programa puede ser reordenado por encima de la instrucción `LDAR`.
- `**STLR**` **(Store-Release)**: Escritura con semántica de liberación. Ningún acceso a memoria previo en orden de programa puede ser reordenado por debajo de la instrucción `STLR`.

```
       Modelo de Sincronización Unidireccional en AArch64

             Lectura Load-Acquire (LDAR)
      -----------------------------------
        [Op. Memoria Posterior 1] \
        [Op. Memoria Posterior 2]  } Ninguna lectura/escritura posterior
        [Op. Memoria Posterior 3] /  puede reordenarse por encima de LDAR

      ===================================

        [Op. Memoria Previa 1]    \
        [Op. Memoria Previa 2]     } Ninguna lectura/escritura previa
        [Op. Memoria Previa 3]    /  puede reordenarse por debajo de STLR
      -----------------------------------
             Escritura Store-Release (STLR)
```

Reemplazar secuencias pesadas de `DMB ISH` por instrucciones pareadas `LDAR`/`STLR` elimina barreras globales de bus, acelerando drásticamente los algoritmos multihilo de cola de trabajos (*Job System*) en motores de juego.

#### Sincronización Atómica y Monitores Exclusivos de Hardware

La implementación de tipos atómicos (`std::atomic`), *spinlocks* y algoritmos libres de bloqueo (*lock-free*) se efectúa mediante las instrucciones pareadas de carga y almacenamiento exclusivo: `LDXR` y `STXR`.

##### Monitores Exclusivos Locales y Globales

El hardware de la CPU ARMv8-A gestiona la atomicidad mediante dos tipos de monitores de estado:

- **Monitor Exclusivo Local**: Supervisa las direcciones de memoria cuando el hilo ejecuta en un solo núcleo.
- **Monitor Exclusivo Global**: Un circuito dentro del interconector de coherencia entre núcleos (*Cache Coherent Interconnect*) que rastrea las direcciones de memoria compartidas marcadas como exclusivas entre múltiples núcleos de la CPU.

Cuando se ejecuta `LDXR`, el monitor exclusivo registra la dirección de memoria y cambia al estado *Exclusive*. Al ejecutar `STXR`, la CPU verifica el estado del monitor: si permanece en estado *Exclusive*, la escritura se efectúa con éxito en la memoria, devuelve `0` en un registro general de estado y limpia el monitor. Si otro núcleo modificó la misma dirección o si ocurrió un cambio de contexto, el monitor pasa a estado *Open*, la escritura falla, `STXR` devuelve `1` y la operación debe reintentarse dentro de un bucle.

##### Prohibición Absoluta entre Parejas `LDXR` y `STXR`

Para evitar condiciones de bloqueo infinito (*livelock*) a nivel de hardware, **está estrictamente prohibido colocar instrucciones de acceso a memoria adicionales (**`**LDR**`**/**`**STR**`**), cambios en registros del sistema o mantenimiento de cachés entre la instrucción** `**LDXR**` **y su correspondiente** `**STXR**`. Colocar accesos a memoria intermedios destruirá el estado del monitor exclusivo, haciendo que `STXR` falle sistemáticamente y provocando un cuelgue definitivo en el *spinlock* del motor de juegos.

```assembly
// Ejemplo de Implementación Atómica de Spinlock en AArch64 (Correcta)
.global acquire_spinlock
acquire_spinlock:
    mov     w1, #1                  // Valor 1 para indicar 'bloqueado'
.Lretry:
    ldaxr   w2, [x0]                // Carga exclusiva con semántica Acquire (LDAXR)
    cbnz    w2, .Lretry             // Si w2 != 0, el candado está ocupado, reintentar
    stxr    w3, w1, [x0]            // Intento de escritura exclusiva de 1
    cbnz    w3, .Lretry             // Si w3 == 1, la escritura falló (monitor borrado), reintentar
    ret                             // Candado adquirido con éxito

.global release_spinlock
release_spinlock:
    stlr    wzr, [x0]               // Escritura Store-Release con valor 0 (STLR)
    ret                             // Libera candado y notifica ordenamiento de memoria
```
