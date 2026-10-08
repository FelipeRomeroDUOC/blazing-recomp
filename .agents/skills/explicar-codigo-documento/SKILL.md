---
name: explicar-codigo-documento
description: Genera un documento de estudio (Markdown por defecto; Word o PDF si el usuario lo pide) que explica un código paso a paso para que el usuario lo estudie y lo entienda por sí mismo, con estilo rubber-duck. El documento incluye momentos "Tu turno" donde el usuario explica el código con sus propias palabras antes de ver la respuesta, de modo que el usuario hace de "pato". Úsalo siempre que el usuario pida explicar un código para aprender, hacer una guía de estudio, apuntes o un resumen didáctico de un script, función, archivo o proyecto, o diga cosas como "no entiendo este código", "explícame cómo funciona esto", "quiero estudiar este programa" o "hazme una guía de este archivo", incluso si no menciona la palabra "documento". No lo uses si el usuario solo quiere corregir un error, escribir código nuevo o refactorizar sin explicación.
---

# Explicar código con un documento de estudio (estilo rubber-duck)

## Qué hace este skill

Convierte un código en un **documento para estudiar**. La idea del rubber-duck (el pato de goma) es que uno entiende mejor un código cuando lo explica en voz alta. Aquí el usuario es el pato: el documento no solo explica, también le pide que explique con sus palabras en puntos concretos ("Tu turno"). Así el usuario aprende de forma activa y no solo leyendo.

Por qué importa: un documento que solo parafrasea el código línea por línea se lee y se olvida. Un documento con estructura predecible, pasos cortos y ejercicios de explicación se puede estudiar, retomar y repasar.

## Flujo de trabajo

### 1. Leer todo el código antes de escribir

- Si el usuario subió archivos, léelos desde `/mnt/user-data/uploads/` (usa el skill `file-reading` si hace falta). Si pegó el código en el chat, úsalo directamente.
- Identifica: propósito general, entradas y salidas, bloques lógicos (funciones, bucles, condiciones, estructuras de datos), dependencias y conceptos que el usuario necesitará conocer.
- Si el código es muy grande (más de ~200 líneas o varios archivos), divide el documento en partes (Parte 1, Parte 2…) con un índice, en lugar de hacer un solo documento enorme.

### 2. Decidir el nivel y el contexto

- Por defecto, asume nivel **principiante** y dilo en el documento. Es más seguro explicar de más que dar por sabido algo que no se sabe.
- Si el nivel del usuario o el origen del código cambian mucho el resultado (por ejemplo, es un examen vs. código de un proyecto propio), haz **una sola pregunta corta**. Si no es crítico, no preguntes: asume y avanza.
- Para ajustar la profundidad, consulta `references/niveles.md`.

### 3. Redactar con la plantilla

- Usa la estructura de `references/plantilla-documento.md`. Es fija a propósito: un documento con siempre las mismas secciones se vuelve predecible y es más fácil de recorrer.
- Aplica las reglas de `references/redaccion-accesible.md` en cada sección. Son la parte que más afecta a si el documento se puede estudiar sin perderse.
- Si necesitas un modelo de cómo queda, mira `assets/ejemplo-documento.md`.
- Escribe el documento en el idioma del usuario. Los nombres de variables, funciones y palabras clave del lenguaje se dejan tal cual.

### 4. Verificar antes de entregar

- Si puedes ejecutar el código de forma segura (sin red, sin borrar ni modificar archivos, sin dependencias desconocidas), ejecútalo con el ejemplo del "Recorrido" para confirmar que los valores de la tabla de seguimiento son correctos. Un valor mal calculado en un documento de estudio confunde mucho.
- Si no puedes ejecutarlo, traza el ejemplo a mano con cuidado y revisa cada fila.
- Comprueba que las respuestas a las preguntas de comprobación son correctas y que cada término técnico se definió antes de usarse.

### 5. Entregar

- Guarda el documento en `/mnt/user-data/outputs/` con un nombre descriptivo (por ejemplo `guia-estudio-calculadora.md`).
- Formato: Markdown por defecto. Si el usuario pide Word o PDF, usa el skill correspondiente (`docx` o `pdf`) con la misma estructura.
- Presenta el archivo con `present_files` y deja un mensaje muy breve: qué contiene, cuánto tiempo aproximado de estudio lleva y por dónde empezar. No repitas el contenido del documento en el chat.

## Reglas de contenido

- **Citar fragmentos, no pegar el archivo entero.** Cada bloque cita solo las líneas que explica (idealmente menos de 15 líneas).
- **Explicar el porqué, no solo el qué.** Para cada bloque: qué hace y por qué está escrito así.
- **Definir antes de usar.** Cada término técnico se explica la primera vez que aparece, con un ejemplo mínimo si ayuda.
- **Mantener el estilo rubber-duck.** Cada bloque termina con un "Tu turno": una pregunta que invita al usuario a explicar con sus palabras. Las respuestas van en una sección final para que no las vea sin querer.
- **Una idea por párrafo y un tema por sección.** Cada sección debe poder leerse de una sentada, en pocos minutos.

## Qué evitar

- Parafrasear el código línea por línea ("esta línea crea una variable") sin explicar la intención.
- Secciones largas, sin título o sin resumen.
- Lenguaje figurado o expresiones ambiguas. Una analogía solo se usa si se marca como analogía y se aclara dónde deja de ser válida.
- Dar por sabido un concepto que no se definió.
- Sermones sobre buenas prácticas que el usuario no pidió. Si algo del código es mejorable, déjalo en una nota breve al final, separada de la explicación.
- Cambiar la estructura del documento de una vez a otra sin motivo.
