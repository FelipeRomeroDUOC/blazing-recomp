# Propuesta: [nombre corto]

Fecha: AAAA-MM-DD
Nivel: 2
Estado: pendiente

<!-- Estados: pendiente | aprobada | modificada | descartada |
     verificada por agente | revisada por humano | discrepada | cerrada -->

## Propuesta (la escribe el agente antes de editar)

### 1. Qué cambia

[Una frase, desde el punto de vista de quien usa la app, sin nombres de funciones]

### 2. Contexto

Cada línea debe cambiar algo que harías. Prueba: "¿qué haría distinto si
esto fuera falso?". Si la respuesta es "nada", la línea sobra. Máximo ~5
líneas en total.

- Contexto consultado: [archivos y documentos que leíste de verdad; "ninguno" si no leíste ninguno]
- Intención: [qué problema resuelve y cómo se ve el éxito para quien usa la app]
- Restricciones y decisiones previas (máx. 3):
  - [decisión] (origen: [archivo, propuesta o "respuesta humana, AAAA-MM-DD"]) → implica: [qué respetar o qué no hacer] → protegida por: [título del escenario] | no testeable
- Alternativas descartadas (solo si las hubo):
  - [alternativa] → descartada porque [razón]
- Ante imprevistos, prioriza: [X sobre Y]

Reglas:
- Toda restricción lleva origen. Sin origen no es una restricción: va a
  "Suposiciones".
- "→ implica" convierte la decisión en una conducta concreta.
- Cada restricción se protege con un escenario de "Criterios de
  aceptación" o se marca "no testeable"; las "no testeable" quedan en
  "Revisión humana".
- Si la persona respondió una pregunta previa, su respuesta entra aquí
  con "respuesta humana" como origen.
- Si un documento contradice el código, gana el código: repórtalo como
  discrepancia en vez de seguir el documento.

### 3. Ejemplos

- Caso normal: [entrada] → antes: [A] → ahora: [B]
- Caso límite: [entrada] → antes: [A] → ahora: [B]

### 4. Pasos

1. [Qué ocurre, una oración]
2. ...

### 5. Qué más podría verse afectado

[Archivos o funciones concretos, hallados buscando referencias.
Si no hay, escribir "ninguno encontrado" y la búsqueda hecha.]

### 6. Suposiciones

- [Lo que se da por cierto sin comprobar] → prototipar: sí | no
  → si es falso: [cómo cambia el plan]
  → resultado del prototipo: [una línea; se completa tras la aprobación] | no detecta: [límite]

Reglas:
- Máximo 3 suposiciones con "prototipar: sí".
- "prototipar: sí" solo si cumple las tres: se puede probar de forma
  aislada, si fuera falsa el plan cambiaría y probarla es barato.
- Las demás quedan como suposición sin comprobar ("prototipar: no").
- Si la persona respondió "no sé" a una pregunta previa, entra aquí.

### 7. Alcance

- Incluye: [2-3 puntos]
- No incluye: [2-3 puntos]

### 8. Criterios de aceptación (no se modifican sin aprobación)

Cada criterio es un escenario con título en una frase, que también es el
nombre del test que lo comprueba.

Escenario: [título en una frase]
Caso de uso: [situación real de quien usa la app] | Tipo: normal | límite | error | regresión
  Dado que [estado inicial concreto]
  Cuando [acción concreta]
  Entonces [resultado observable con valor concreto]
Test: [ruta o nombre del test; se escribe tras la aprobación]
Estado previo: [se completa al escribir el test: falla | pasa (regresión)]

Reglas:

- El "Entonces" debe incluir un valor o resultado observable.
  Prohibido: "el sistema funciona correctamente".
- Cobertura mínima: uno normal, uno límite y, si el cambio afecta
  comportamiento que ya existe, uno de regresión. Suma uno de error si
  hay entradas externas.
- Los tests se definen aquí y se escriben DESPUÉS de la aprobación (y de
  los prototipos), antes de editar. Deben fallar por la razón correcta;
  los de regresión deben pasar. Si un test nuevo pasa sin cambios, no
  mide lo que dice: detenerse y corregir el test o la propuesta.
- Máximo 3-4 escenarios.
- Criterios que no se puedan comprobar con código (visuales): marcarlos
  "(visible)"; quedan para la revisión humana.

### 9. Cómo lo comprobaré

Una línea por escenario. Sin esto, la comprobación es una caja negra.

- [Título del escenario] → herramienta o comando: [ej. `pytest tests/test_x.py::test_y`]
  → pasa si: [resultado concreto] → no detecta: [límite de esta comprobación]
- [Escenario "(visible)"] → revisión humana: [qué debe mirar la persona]
  → no detecta: [límite]

Reglas:
- Nombra la herramienta o el comando exacto, no "pruebas" en general.
- "No detecta" es obligatorio: toda comprobación tiene límites. Si de
  verdad no ves ninguno, escribe "ninguno que yo vea".
- Antes de proponerla, verifica que la herramienta existe y corre en
  este proyecto.
- Si algo no se puede comprobar con ninguna herramienta, dilo aquí en
  vez de omitirlo.

### 10. Terminado cuando

Se cumplen todos los criterios de aceptación.

### 11. Parar si

2 intentos fallidos seguidos con el mismo criterio. Detenerse, explicar
qué se probó y marcar la propuesta como "discrepada".

### Correcciones previas a presentar

[Qué detectó y corrigió en su autocomprobación, o "ninguna"]

### Cambios tras prototipos (solo si el plan cambió)

- Antes: [lo que decía la propuesta] → Ahora: [lo que dice] → Por qué: [resultado del prototipo]

---

## Resultado (lo completa el agente después de editar)

### Predicho vs. real

- [Escenario]: coincide | difiere | no pudo comprobarse
  Resultado real: [lo que se obtuvo al ejecutar]
  Método: el anunciado | distinto: [cuál y por qué]

### Contexto: ¿se respetó?

- [Restricción] → respetada | no respetada: [qué pasó]
- ¿Se tocó algo que alguna restricción prohibía? sí | no: [qué]
- Imprevistos que surgieron: [cuáles] → criterio de prioridad aplicado: sí | no | no hubo imprevistos
- ¿Reapareció alguna alternativa descartada? sí | no

### Archivos modificados

[Lista real]. ¿Coincide con lo anunciado? sí | no: [cuál]

### Discrepancias

- [Descripción] → error de juicio | de implementación |
  suposición falsa | información faltante | omisión | imprevisto

### Para la próxima vez

[Una línea concreta]

### Fuera de alcance (hallazgos que no se arreglaron)

- [Una línea por hallazgo] | impacto: bajo | medio
  (o "ninguno")

---

## Discrepancia (solo si el estado es "discrepada")

Abierta por: agente | humano
Punto afectado: [escenario o criterio]
Motivo: [qué difiere o qué falla]
Clasificación: error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto
Propuesta de corrección: [enlace al nuevo archivo]

---

## Revisión humana
<!-- El agente SÍ edita esta sección: escribe aquí los escenarios que no se
     pudieron comprobar con código ("(visible)" o "no pudo comprobarse") y las restricciones
     marcadas "no testeable",
     uno por línea, dejando las marcas ✔ | ✘ | ? sin elegir.
     El agente NO elige marca, NO escribe la nota y NO rellena
     "Detectado solo por el humano": eso es solo de la persona.
     Si no queda nada para revisar: "ninguno: todo se comprobó con tests
     ([cuáles])". -->

- [Escenario o punto]: ✔ | ✘ | ?  [nota de la persona, opcional]
- Detectado solo por el humano: [lo completa la persona]
