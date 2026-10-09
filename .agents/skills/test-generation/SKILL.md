---
name: test-generation
description: Guía para generar tests unitarios, de integración y de mutación de buena calidad. Úsala siempre que el usuario pida crear, mejorar, revisar o ampliar tests, suites de pruebas, cobertura, mocks, fixtures, pytest, Jest, JUnit, Vitest, Stryker, PIT o mutmut, o cuando mencione "mutation testing", "mutantes", "tests que no detectan errores" o "subir la calidad de los tests", aunque no diga explícitamente qué tipo de test quiere.
---

# Generación de tests: unitarios, integración y mutación

Un test vale lo que vale su capacidad de **fallar cuando el código está mal**. Un test que siempre pasa da una falsa sensación de seguridad. Todo lo de esta skill apunta a eso: tests que detecten errores reales, no que solo suban el porcentaje de cobertura.

## Flujo de trabajo

1. **Entender el contrato** antes de escribir nada (ver abajo).
2. **Descubrir el entorno**: lenguaje, framework de tests, estructura de carpetas, fixtures y helpers existentes, comando para ejecutar los tests. Copiar las convenciones del proyecto; no inventar otras.
3. **Elegir el tipo de test y la estrategia temporal** según lo que se prueba y la tarea (ver tabla y sección de estrategia).
4. **Escribir los tests respetando la secuencia**: antes de tocar el código para bugfixes o refactors; después para casos límite y robustez.
5. **Medir su calidad** con mutación cuando el proyecto lo permita.
6. **Reportar** qué se cubrió, qué no, y cualquier bug encontrado.

## Entender el contrato primero

Si el agente solo lee la implementación, termina escribiendo tests que repiten lo que el código ya hace, incluidos sus bugs. Por eso, antes de testear:

- Buscar el comportamiento esperado en docstrings, README, especificaciones, issues o nombres de funciones.
- Si el contrato no está claro o es ambiguo, **preguntar al usuario** en lugar de adivinar. Un test basado en una suposición incorrecta es peor que no tener test.
- Escribir mentalmente (o en un comentario breve) qué debe pasar con entradas normales, límites y errores.

## Qué tipo de test elegir

| Tipo | Qué prueba | Dependencias externas | Velocidad |
|---|---|---|---|
| Unitario | Una función o clase aislada | Reemplazadas por mocks/stubs/fakes | Milisegundos |
| Integración | Varias piezas juntas (servicio + base de datos, API + cola) | Reales o contenedores de prueba | Segundos |
| Mutación | La calidad de los tests existentes | No aplica | Lenta (minutos) |

Proporción recomendada: muchos unitarios, menos de integración, pocos o ninguno de extremo a extremo. Si el código es lógica pura (cálculos, validaciones, transformaciones), bastan unitarios. Si el riesgo está en cómo se conectan las piezas (consultas SQL, serialización, llamadas HTTP), hace falta integración.

Para detalles de cada tipo y estrategia, leer el archivo correspondiente:

- Unitarios: `references/unitarios.md`
- Integración: `references/integracion.md`
- Mutación: `references/mutacion.md`
- Estrategia por tarea (Antes vs. Después): `references/estrategias-por-tarea.md`

## Estrategia según el tipo de tarea (Antes vs. Después)

El momento en que se escribe un test define si realmente protege contra errores:

- **Al corregir un bug (Test de reproducción):** Escribir primero un test que reproduzca el fallo antes de tocar el código de producción. **Debe fallar en rojo.** Solo cuando falle, corregir el código para que pase a verde.
- **Al refactorizar (Tests de caracterización / regresión):** Identificar o escribir tests que capturen el comportamiento observable actual. **Deben pasar en verde antes de refactorizar** y mantenerse en verde tras los cambios sin modificar sus asserts.
- **Al crear una nueva funcionalidad (Tests de contrato):** Escribir tests de especificación según los requisitos antes o a la par del código (estilo TDD). Estarán en rojo hasta que la función exista.
- **Tras la implementación (Casos límite y robustez):** Blindar con valores frontera (vacíos, nulos, ceros, límites de rango, errores esperados) y verificar solidez con mutación o alterando deliberadamente una condición.

## Reglas que valen para todos los tipos

**Verificar comportamiento, no implementación.** Probar qué devuelve o qué efecto tiene una función, no cómo lo hace internamente. Así los tests sobreviven a refactorizaciones.

**Asserts concretos.** `assert resultado == 42` es útil; `assert resultado is not None` casi nunca lo es. Comprobar el valor exacto, el tipo de error y el mensaje cuando importe.

**Deterministas.** Un test no debe depender de la hora actual, números aleatorios, el orden de ejecución, la red ni el estado que dejó otro test. Fijar semillas, inyectar relojes falsos y limpiar el estado en cada test.

**Un motivo para fallar por test.** Nombres descriptivos que digan el escenario y el resultado esperado, por ejemplo `test_descuento_no_se_aplica_si_el_total_es_menor_al_minimo`. Estructura Arrange / Act / Assert (preparar, ejecutar, verificar).

**No duplicar la lógica del código dentro del test.** Si el test calcula el resultado esperado con la misma fórmula que el código, ambos fallarán juntos. Usar valores esperados escritos a mano.

**Casos límite siempre**: vacío, nulo, cero, negativos, valores máximos, texto con acentos o caracteres especiales, listas de un solo elemento, entradas inválidas y errores esperados.

## Límites que el agente debe respetar

- **No modificar el código de producción para que los tests pasen.** Si un test revela un bug, reportarlo al usuario con el caso que lo expone. Corregirlo solo si el usuario lo pide.
- **No borrar ni debilitar tests existentes** para que la suite quede en verde.
- **No marcar tests como `skip` o `xfail`** sin avisar y explicar por qué.
- **No inventar dependencias**: usar el framework que el proyecto ya tiene. Si hace falta instalar algo (por ejemplo una herramienta de mutación), decirlo antes.
- **No dar por buenos tests que no se ejecutaron.** Siempre correr la suite y mostrar el resultado real.

## Verificación antes de entregar

Antes de decir que terminó, comprobar:

- [ ] Si era un bugfix, se comprobó que el test fallaba antes de modificar el código de producción.
- [ ] Todos los tests nuevos pasan al ejecutarlos.
- [ ] Cada test fallaría si se rompiera la lógica que prueba (probar cambiando temporalmente una condición del código y revisando que falle, o usar mutación).
- [ ] No hay dependencias del orden, la hora ni la red.
- [ ] Los nombres explican el escenario.
- [ ] Se siguieron las convenciones del proyecto.

## Cómo reportar al usuario

Cerrar con un resumen corto: cuántos tests se agregaron y de qué tipo, qué comportamientos quedaron cubiertos, qué quedó fuera y por qué, y si se detectó algún bug. Si el usuario está aprendiendo, explicar en una o dos frases por qué se eligió cada tipo de test; evitar jerga sin definirla.
