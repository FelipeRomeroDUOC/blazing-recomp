# Diseño de tests

Cómo derivar y escribir los tests de una propuesta de nivel 2. Se lee al escribir los tests, después de la aprobación.

## De dónde salen los casos

- Parte de los casos de uso: situaciones reales de quien usa la app ("el cliente ve el total de su pedido"). Cada escenario nombra el caso de uso del que sale.
- Cada escenario tiene un tipo:
  - **normal:** el uso esperado.
  - **límite:** vacío, muy grande, valor inesperado.
  - **error:** entrada inválida, y qué debe pasar entonces.
  - **regresión:** comportamiento existente que no debe cambiar.
- Cobertura mínima: uno normal, uno límite y, si el cambio afecta comportamiento que ya existe, uno de regresión. Suma uno de error si hay entradas externas.
- Tope: 3 o 4 escenarios. Si no caben, divide el cambio.
- Cada restricción del bloque "Contexto" se protege con un escenario, o se marca "no testeable" y queda para la revisión humana.

## Cómo escribirlos (después de la aprobación)

- Un comportamiento por test. El nombre es el título del escenario, en una frase.
- Prueba lo observable (resultado, mensaje, estado visible), no la implementación (qué función interna se llamó).
- No copies la lógica del código dentro del test. El valor esperado sale del escenario aprobado, no de ejecutar el código nuevo.
- Usa valores concretos: si el escenario dice 35.7, el test comprueba 35.7.
- Usa el framework y el estilo de tests que ya tiene el proyecto. Para ver el estilo esperado, mira `assets/ejemplo-tests.py` (ilustrativo, en Python): copia el estilo, no los valores.
- Ejecuta cada test antes de editar y anota su "Estado previo":
  - Escenarios nuevos: deben fallar **por la razón correcta** (falta el comportamiento nuevo). Un test que falla por otra razón (error de importación, datos mal armados) tampoco sirve.
  - Escenarios de regresión: deben pasar.
  - Si un test nuevo pasa sin cambios, no mide lo que dice: detente y corrige el test o la propuesta.

## Lo que no se puede testear

Lo visual y lo subjetivo: márcalo "(visible)" o "no testeable" y se lista en "Revisión humana". No finjas un test que no comprueba nada.
