# Ajustar la profundidad según el nivel

Si el usuario no indica su nivel, usa **principiante** y escríbelo en la cabecera del documento. Si el usuario dice que ya sabe algo, sube el nivel solo en esa parte.

## Principiante

- Define cada concepto básico (variable, función, bucle, condición, lista, diccionario…) con un ejemplo mínimo.
- Explica la sintaxis ("`def` sirve para crear una función").
- Recorrido de ejemplo detallado: una fila por cada cambio de variable.
- Preguntas de comprobación centradas en "qué hace" y "qué valor tiene".
- Glosario completo.

## Intermedio

- Asume conceptos básicos; define solo los específicos del código (por ejemplo, recursión, decoradores, promesas).
- Explica la intención y las decisiones de diseño (por qué esta estructura de datos).
- Recorrido de ejemplo por vueltas o llamadas, no por línea.
- Preguntas sobre casos borde (entrada vacía, valores repetidos, errores).
- Incluye una nota de complejidad solo si es relevante.

## Avanzado

- Céntrate en arquitectura, flujo de datos y trade-offs.
- Menciona complejidad temporal y espacial, alternativas y posibles problemas.
- Recorrido de ejemplo solo en las partes no triviales.
- Preguntas de diseño y de modificación ("¿cómo cambiarías esto para soportar…?").
- Glosario mínimo o ausente.

## Señales para detectar el nivel

- El usuario dice "soy nuevo", "no sé programar" o pregunta por conceptos muy básicos → principiante.
- El usuario usa vocabulario técnico con soltura o pregunta por "por qué" de diseño → intermedio o avanzado.
- En caso de duda, principiante: es más fácil saltarse lo que ya se sabe que perderse por falta de base.
