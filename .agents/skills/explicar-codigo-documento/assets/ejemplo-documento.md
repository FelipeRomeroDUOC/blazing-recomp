# Guía de estudio: contar palabras

> ⏱ Tiempo estimado: 15 min · Nivel: principiante · Lenguaje: Python

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

Este programa cuenta cuántas veces aparece cada palabra en un texto.

- **Entrada:** un texto, por ejemplo `"hola mundo hola"`.
- **Salida:** un diccionario con cada palabra y su número de apariciones, por ejemplo `{'hola': 2, 'mundo': 1}`.

## 2. Mapa general

| Bloque | Qué hace | Líneas |
|--------|----------|--------|
| 1. Preparar | Crea un diccionario vacío para guardar los conteos | 2 |
| 2. Recorrer | Separa el texto en palabras y pasa por cada una | 3 |
| 3. Contar | Suma 1 al conteo de la palabra actual | 4 |
| 4. Devolver | Entrega el diccionario terminado | 5 |

El código completo:

```python
def contar_palabras(texto):
    conteo = {}
    for palabra in texto.lower().split():
        conteo[palabra] = conteo.get(palabra, 0) + 1
    return conteo
```

## 3. Conceptos previos

**Función**
- Qué es: un bloque de código con nombre que recibe datos y devuelve un resultado.
- Ejemplo mínimo:
  ```python
  def doble(n):
      return n * 2
  ```

**Diccionario**
- Qué es: una estructura que guarda pares *clave → valor*. Se busca un valor a partir de su clave.
- Ejemplo mínimo:
  ```python
  edades = {"Ana": 20, "Luis": 25}
  ```

**Bucle `for`**
- Qué es: repite un bloque de código una vez por cada elemento de una secuencia.
- Ejemplo mínimo:
  ```python
  for letra in "abc":
      print(letra)
  ```

## 4. Explicación por bloques

### Bloque 1: preparar el diccionario (⏱ ~2 min)

```python
conteo = {}
```

**Qué hace:** crea un diccionario vacío llamado `conteo`.
**Por qué está así:** hace falta un lugar donde guardar los resultados. Empieza vacío porque aún no se ha contado nada.
**Resumen del bloque:** `conteo` es el lugar donde se irán anotando las palabras y sus cantidades.

**Tu turno:** ¿Qué crees que pasaría si `conteo` no se creara antes del bucle? Explica por qué.

☕ *Pausa: aquí puedes parar.*

### Bloque 2: recorrer las palabras (⏱ ~4 min)

```python
for palabra in texto.lower().split():
```

**Qué hace:** convierte el texto a minúsculas con `lower()`, lo separa en una lista de palabras con `split()` y repite el bloque una vez por cada palabra. En cada vuelta, `palabra` contiene la palabra actual.
**Por qué está así:** `lower()` hace que "Hola" y "hola" cuenten como la misma palabra. `split()` corta el texto por los espacios.
**Resumen del bloque:** el bucle visita cada palabra del texto, ya en minúsculas, una a una.

**Tu turno:** Si el texto es `"Sol sol"`, ¿cuántas vueltas da el bucle y qué valor tiene `palabra` en cada una?

☕ *Pausa: aquí puedes parar.*

### Bloque 3: sumar al conteo (⏱ ~5 min)

```python
conteo[palabra] = conteo.get(palabra, 0) + 1
```

**Qué hace:** busca cuántas veces se ha visto `palabra` hasta ahora y guarda ese número más 1.
**Por qué está así:** `conteo.get(palabra, 0)` devuelve el valor guardado para `palabra`. Si la palabra aún no está en el diccionario, devuelve `0` en lugar de dar error. Así la primera vez que aparece una palabra su conteo pasa a ser `0 + 1 = 1`.
**Resumen del bloque:** cada vez que aparece una palabra, su contador sube en 1, empezando desde 0 si es nueva.

**Tu turno:** ¿Qué devuelve `conteo.get("gato", 0)` si `conteo` es `{"perro": 3}`? ¿Y si `conteo` es `{"gato": 2}`?

☕ *Pausa: aquí puedes parar.*

### Bloque 4: devolver el resultado (⏱ ~1 min)

```python
return conteo
```

**Qué hace:** entrega el diccionario a quien llamó a la función.
**Por qué está así:** el diccionario se llena dentro de la función; sin `return`, quien la llamó no recibiría nada.
**Resumen del bloque:** `return` sale de la función y entrega el resultado final.

**Tu turno:** ¿Dónde se ejecuta `return conteo`: dentro del bucle o después de terminarlo? ¿Cómo lo sabes?

## 5. Recorrido con un ejemplo

Entrada: `contar_palabras("hola mundo hola")`

Tras `lower().split()` queda la lista `["hola", "mundo", "hola"]`.

| Paso | Qué ocurre | `palabra` | `conteo.get(palabra, 0)` | `conteo` después |
|------|------------|-----------|--------------------------|------------------|
| Inicio | Se crea el diccionario | — | — | `{}` |
| Vuelta 1 | Primera palabra | `"hola"` | `0` | `{"hola": 1}` |
| Vuelta 2 | Segunda palabra | `"mundo"` | `0` | `{"hola": 1, "mundo": 1}` |
| Vuelta 3 | Tercera palabra | `"hola"` | `1` | `{"hola": 2, "mundo": 1}` |
| Fin | Se devuelve | — | — | `{"hola": 2, "mundo": 1}` |

## 6. Preguntas de comprobación

1. ¿Qué hace `texto.lower().split()` con `"Hola Mundo"`?
2. ¿Qué devuelve la función si el texto es una cadena vacía `""`?
3. ¿Por qué se usa `conteo.get(palabra, 0)` en lugar de `conteo[palabra]`?

## 7. Ejercicios de modificación

1. Quita `.lower()` y ejecuta con `"Hola hola"`. Debería aparecer `"Hola"` y `"hola"` como palabras distintas.
2. Cambia `+ 1` por `+ 2`. Cada palabra debería contar el doble.

## Respuestas

**Tu turno (Bloque 1):** Daría un error, porque `conteo[palabra] = ...` y `conteo.get(...)` necesitan que `conteo` exista antes de usarse.

**Tu turno (Bloque 2):** Da 2 vueltas. En la primera `palabra` es `"sol"` y en la segunda también `"sol"`, porque `lower()` pasó "Sol" a minúsculas.

**Tu turno (Bloque 3):** Con `{"perro": 3}` devuelve `0`, porque "gato" no está. Con `{"gato": 2}` devuelve `2`.

**Tu turno (Bloque 4):** Después de terminar el bucle: `return` está sin sangría extra, al mismo nivel que `for`, fuera de su bloque.

**Preguntas de comprobación:**
1. Produce la lista `["hola", "mundo"]`.
2. Devuelve `{}`: `"".split()` da una lista vacía, el bucle no se ejecuta y se devuelve el diccionario vacío.
3. Porque `conteo[palabra]` da error si la palabra aún no existe; `get` con `0` evita ese error y permite empezar a contar.

**Ejercicios:**
1. Resultado esperado: `{'Hola': 1, 'hola': 1}`.
2. Resultado esperado con `"hola mundo hola"`: `{'hola': 4, 'mundo': 2}`.

## Glosario

- **Función:** bloque de código con nombre que recibe datos y devuelve un resultado.
- **Diccionario:** estructura que guarda pares clave → valor.
- **Bucle `for`:** repite código una vez por cada elemento de una secuencia.
- **Método:** función que pertenece a un objeto, como `lower()` o `split()` en un texto.
