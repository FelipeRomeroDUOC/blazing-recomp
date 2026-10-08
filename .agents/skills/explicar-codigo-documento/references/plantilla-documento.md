# Plantilla del documento de estudio

Usa siempre estas secciones y en este orden. Si una sección no aplica (por ejemplo, no hay conceptos nuevos), déjala con una línea que lo diga en lugar de eliminarla: la estructura predecible es parte del diseño.

## Estructura exacta

````markdown
# Guía de estudio: [nombre del programa o archivo]

> ⏱ Tiempo estimado: [X] min · Nivel: [principiante/intermedio/avanzado] · Lenguaje: [Python/JS/...]

## Cómo usar esta guía

1. Lee las secciones en orden.
2. Cuando llegues a un **Tu turno**, para y explica la respuesta con tus palabras (en voz alta o escribiéndola). Después compárala con la sección *Respuestas*.
3. Puedes parar en cualquier **Pausa** y retomar después.

## Progreso

- [ ] 1. Objetivo del programa
- [ ] 2. Mapa general
- [ ] 3. Conceptos previos
- [ ] 4. Explicación por bloques
- [ ] 5. Recorrido con un ejemplo
- [ ] 6. Preguntas de comprobación
- [ ] 7. Ejercicios de modificación

## 1. Objetivo del programa

Qué problema resuelve, en 2 o 3 frases. Qué recibe (entrada) y qué devuelve o muestra (salida).

## 2. Mapa general

Tabla con una fila por bloque del código:

| Bloque | Qué hace | Líneas |
|--------|----------|--------|

## 3. Conceptos previos

Una mini-definición por concepto, cada una con:
- **Qué es** (una frase).
- **Ejemplo mínimo** (1 a 3 líneas de código).

## 4. Explicación por bloques

### Bloque N: [nombre] (⏱ ~X min)

```[lenguaje]
[fragmento corto]
```

**Qué hace:** …
**Por qué está así:** …
**Resumen del bloque:** una sola frase.

**Tu turno:** una pregunta que pida explicar algo con palabras propias.

☕ *Pausa: aquí puedes parar.*

## 5. Recorrido con un ejemplo

Entrada concreta y tabla paso a paso mostrando cómo cambian las variables:

| Paso | Qué ocurre | Variable A | Variable B |
|------|------------|------------|------------|

## 6. Preguntas de comprobación

3 a 5 preguntas, numeradas. Mezcla:
- una de comprensión ("¿qué hace…?"),
- una de predicción ("¿qué pasa si la entrada es…?"),
- una de razonamiento ("¿por qué se usa… en lugar de…?").

## 7. Ejercicios de modificación

2 o 3 cambios pequeños para probar (cambiar un valor, añadir una condición…). Cada uno indica qué se espera observar.

## Respuestas

Respuestas a los "Tu turno", a las preguntas de comprobación y a los ejercicios, numeradas igual que arriba.

## Glosario

Lista breve de los términos técnicos usados, con su definición en una línea.

## Notas finales (opcional)

Solo si hay algo mejorable en el código. Máximo 3 puntos, separado de la explicación.
````

## Notas de uso

- **Tiempos**: estima el tiempo de cada bloque de forma realista (lectura + Tu turno). Es útil para planificar sesiones cortas.
- **Casillas de progreso**: dejan al usuario marcar por dónde va, útil si estudia en varias sesiones.
- **Preguntas "Tu turno"**: formúlalas de modo que se puedan responder con el contenido del propio bloque, sin conocimientos externos. No deben ser trampas.
- **Código largo**: si el código supera ~200 líneas, genera un documento por parte y un índice al inicio con enlaces o referencias a cada parte.
