# Los cuatro tipos de documentación: ejemplos y casos límite

Contenido:

1. Cómo decidir el tipo de una página
2. Tutorial
3. Guía práctica
4. Referencia
5. Explicación
6. Casos límite frecuentes
7. Cómo corregir una página que mezcla tipos

## 1. Cómo decidir el tipo de una página

Hazte dos preguntas sobre el lector en ese momento:

- ¿Necesita **actuar** o **saber**?
- ¿Está **aprendiendo** (estudiando) o **trabajando** (con una tarea entre manos)?

|  | Actuar | Saber |
|---|---|---|
| **Aprendiendo** | Tutorial | Explicación |
| **Trabajando** | Guía práctica | Referencia |

Si una página responde a dos cuadrantes a la vez, son dos páginas. Sepáralas y enlázalas.

## 2. Tutorial

**Señales de que está bien:** un solo camino; el lector termina con algo que funciona y lo puede ver; cada paso tiene un resultado visible; las explicaciones ocupan una frase como máximo.

**Errores comunes:**

- Ofrecer alternativas ("también puedes usar X o Y"). El principiante no sabe elegir.
- Explicar la teoría en medio de los pasos.
- Pasos que dependen de algo que el tutorial nunca pidió instalar.
- Comandos que no se probaron.

**Mal** (mezcla explicación y alternativas):

> Para instalar el proyecto puedes usar pip, conda o Poetry. Poetry es el más recomendado porque resuelve dependencias de forma determinista, algo que surgió como respuesta a los problemas del archivo requirements.txt...

**Bien:**

> ### 1. Instala el proyecto
>
> ```bash
> pip install mi-proyecto
> ```
>
> Deberías ver `Successfully installed mi-proyecto`.

## 3. Guía práctica

**Señales de que está bien:** el título es una tarea ("Cómo ..."); empieza directo con los requisitos y los pasos; asume que el lector ya sabe lo básico; enlaza a la referencia en vez de describir todas las opciones.

**Errores comunes:**

- Enseñar conceptos antes de los pasos.
- Cubrir todos los casos posibles en lugar de uno claro.
- Titular con la herramienta ("Usando Docker") en vez de con la tarea ("Cómo desplegar con Docker").
- Repetir el contenido del tutorial.

**Mal:**

> ## Docker
>
> Docker es una plataforma de contenedores que permite empaquetar aplicaciones. Existen muchas opciones de configuración, como `--network`, `--volume`, `--env`...

**Bien:**

> ## Cómo desplegar con Docker
>
> Esta guía despliega el servicio en un servidor con Docker instalado.
>
> 1. Construye la imagen: `docker build -t mi-servicio .`
> 2. Inicia el contenedor: `docker run -d -p 8000:8000 mi-servicio`
>
> Para ver todas las variables de entorno disponibles, consulta la [referencia de configuración](../referencia/configuracion.md).

## 4. Referencia

**Señales de que está bien:** estructura idéntica para cada elemento; neutral y precisa; exhaustiva dentro de su alcance; todos los datos salen del código; se puede consultar sin leerla de principio a fin.

**Errores comunes:**

- Dar opiniones o consejos ("lo ideal es...").
- Contar el porqué de una decisión.
- Mezclar pasos de instalación con la descripción de parámetros.
- Omitir parámetros, valores por defecto o errores que sí existen.
- Copiar la descripción de memoria en vez de leer el código.

**Mal:**

> `timeout`: Te recomendamos usar un valor alto, porque las conexiones lentas suelen fallar. Esto se debe a que históricamente el servidor tardaba en responder...

**Bien:**

> | Nombre | Tipo | Por defecto | Descripción |
> |---|---|---|---|
> | `timeout` | `int` | `30` | Segundos de espera antes de cancelar la conexión. Lanza `TimeoutError` al agotarse. |

## 5. Explicación

**Señales de que está bien:** responde "por qué" o "cómo se relaciona"; cuenta el contexto y las decisiones; presenta alternativas y lo que se sacrificó; puede leerse lejos del teclado.

**Errores comunes:**

- Incluir pasos de instalación o comandos que hay que ejecutar.
- Convertirla en un resumen de la referencia.
- Inventar las razones de una decisión. El código muestra qué se hizo, casi nunca por qué. Si no hay evidencia, se marca con `[VERIFICAR]`.
- Presentar el proyecto como perfecto, sin mencionar los compromisos.

**Mal** (razón inventada):

> El proyecto usa SQLite porque el equipo valoraba la simplicidad y la portabilidad.

Nada en el código ni en el historial dice eso. Es una suposición escrita como hecho.

**Bien:**

> El proyecto guarda los datos en SQLite (ver `src/db.py`). `[VERIFICAR: ¿por qué se eligió SQLite y no un servidor de base de datos?]`
>
> Consecuencia visible en el código: no hay configuración de conexión ni usuario, y el archivo de base de datos vive en el mismo directorio del proyecto.

## 6. Casos límite frecuentes

- **El README.** Es una página de entrada, no un tipo. Debe responder en pocas líneas: qué es el proyecto, cómo instalarlo y ejecutarlo lo mínimo, y a dónde ir después (enlaces a `docs/`). Si crece más allá de eso, mueve el contenido a la página del tipo que corresponda.
- **Docstrings y comentarios del código.** Son referencia (el comentario sobre la función) y explicación (las decisiones dentro). Se tratan según las reglas de comentarios de `SKILL.md`.
- **CHANGELOG.** No es parte de Diátaxis. Déjalo como está y no lo reorganices a menos que el usuario lo pida.
- **CONTRIBUTING.** Suele mezclar guía práctica (cómo contribuir) y explicación (por qué se trabaja así). Sepáralo solo si ambas partes son largas.
- **Preguntas frecuentes (FAQ).** Cada pregunta suele pertenecer a un tipo distinto. Si la FAQ es corta, déjala; si crece, reparte las respuestas y enlaza.
- **Ejemplos de código.** Un ejemplo corto dentro de una referencia es parte de ella. Una carpeta de ejemplos completos es más cercana a tutoriales o guías.

## 7. Cómo corregir una página que mezcla tipos

1. Lee la página y marca cada bloque con el tipo al que pertenece (T, G, R, E).
2. Identifica cuál es el propósito dominante: ese será el tipo de la página.
3. Mueve los bloques de otros tipos a páginas propias, o descártalos si ya existen.
4. En el lugar donde estaba cada bloque movido, deja un enlace a su nueva página.
5. Revisa que no se haya perdido información: lo movido debe seguir existiendo en alguna parte.
