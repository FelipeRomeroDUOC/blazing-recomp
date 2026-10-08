---
name: diataxis-docs
description: Analiza un proyecto de software, propone un plan de documentación basado en Diátaxis (tutoriales, guías prácticas, referencia, explicación) y, una vez aprobado el plan, escribe la documentación en español. Úsala siempre que el usuario pida documentar un proyecto, crear, ordenar o reorganizar documentación, escribir un README, tutorial, guía, referencia de API, AGENTS.md o llms.txt, revisar si la documentación quedó desactualizada tras cambios en el código, o mencione "Diátaxis", aunque no diga explícitamente que quiere un plan.
---

# Documentación con Diátaxis

## Propósito

Producir documentación que un humano o un agente nuevo pueda usar para entender el proyecto rápido. Esa es la vara de medir: no la cantidad de páginas, sino cuánto tarda alguien en encontrar lo que necesita y confiar en ello.

## Principios

1. **No mezclar tipos de documentación.** Cada página sirve a una sola necesidad del lector. Un tutorial con explicaciones largas o una referencia con pasos de instalación obligan al lector a buscar lo suyo entre cosas que no necesita en ese momento.
2. **No inventar.** Puedes afirmar con seguridad lo que el código, los tests y la configuración demuestran. No puedes saber por qué los autores decidieron algo, ni qué experiencia de aprendizaje es la correcta. Cuando falte información, escribe `[VERIFICAR: qué falta y dónde buscarlo]` en vez de rellenar con suposiciones. Una laguna marcada es útil; una suposición escrita como hecho es dañina.
3. **Plan antes de escribir.** Nunca crees ni modifiques archivos de documentación sin que el usuario haya aprobado un plan. Así evitas llenar el proyecto de páginas que nadie pidió.
4. **Lo mínimo que sirve.** Documenta lo que más ayuda primero. Una página corta y verificada vale más que diez largas y dudosas.
5. **Doble audiencia.** Escribe pensando en humanos y agentes: título descriptivo, el resumen en las primeras líneas, un solo tema por página, enlaces explícitos entre páginas, y metadatos de tipo al inicio (ver "Formato de página").

## Los cuatro tipos

| Tipo | El lector se pregunta | Debe | No debe |
|---|---|---|---|
| **Tutorial** | "Quiero aprender" | Llevar de la mano por un camino que funciona, con resultado visible al final | Explicar el porqué en detalle, ofrecer alternativas, listar opciones |
| **Guía práctica** | "Quiero lograr X" | Dar los pasos para una tarea concreta, asumiendo que el lector ya sabe lo básico | Enseñar conceptos, cubrir todos los casos posibles |
| **Referencia** | "Necesito el dato exacto" | Describir con precisión y de forma neutra (funciones, parámetros, comandos, configuración) | Dar opiniones, explicar el porqué, contar historias |
| **Explicación** | "Quiero entender" | Aclarar contexto, decisiones de diseño, alternativas y compromisos | Dar pasos, listar parámetros |

Antes de escribir una página, pregúntate si el lector está **estudiando o trabajando** y si necesita **acción o conocimiento**. Eso decide el tipo. Si dudas entre dos, probablemente son dos páginas.

Para ejemplos buenos y malos de cada tipo, lee `references/tipos.md` si existe.

## Flujo de trabajo

### Fase 1: Analizar el proyecto

Empieza ejecutando el inventario, que reúne en un informe los hechos básicos (lenguajes, manifiestos, puntos de entrada, comandos, tests, documentación existente):

```bash
python <ruta-de-la-skill>/scripts/inventariar_proyecto.py <ruta-del-proyecto>
```

El inventario solo da hechos y pistas; no decide el tipo de proyecto ni clasifica la documentación. Eso lo haces tú leyendo el contenido.

Después lee, en este orden: README, archivos de configuración y dependencias (por ejemplo `package.json`, `pyproject.toml`, `Cargo.toml`, `Dockerfile`), punto de entrada, API pública, tests y documentación existente. Los tests suelen mostrar el uso real mejor que los comentarios.

Identifica el **tipo de proyecto**, porque determina qué documentación importa más:

| Tipo de proyecto | Prioridad probable |
|---|---|
| Biblioteca / paquete | Referencia de API, guías de uso, un tutorial de primer uso |
| Aplicación de línea de comandos | Referencia de comandos y opciones, guías por tarea |
| Aplicación web o móvil | Instalación y ejecución local, arquitectura (explicación), guía de despliegue |
| API o servicio | Referencia de endpoints, autenticación, guías de integración |
| Proyecto de datos / IA | Reproducibilidad, descripción de datos y modelos, explicación de decisiones |
| Infraestructura / configuración | Guías de operación, referencia de variables, explicación de la arquitectura |

Si el proyecto no encaja claramente, dilo y pregunta al usuario. No fuerces una categoría.

### Fase 2: Inventariar lo existente

Para cada documento que ya exista, anota su tipo real (no el que dice ser), si mezcla tipos, si parece desactualizado respecto al código y si se puede reutilizar. Reutiliza y reorganiza antes de reescribir.

### Fase 3: Proponer el plan

Presenta el plan al usuario con este formato y **detente a esperar su respuesta**:

```
## Resumen del proyecto
[Qué es, qué tipo de proyecto, qué lenguajes/herramientas, en 2-4 líneas]

## Documentación existente
[Lista corta: archivo → tipo real → estado (usable / mezcla tipos / desactualizado)]

## Plan propuesto
| Prioridad | Archivo | Tipo | Qué cubre | Origen de la información |
|---|---|---|---|---|
| 1 | docs/referencia/cli.md | Referencia | Comandos y opciones | Código en src/cli.py |

## Comentarios en el código
[Qué archivos o funciones se comentarán y por qué; o "sin cambios" si ya están bien]

## Lagunas y dudas
[Lo que no puedes saber leyendo el código y necesitas que el usuario confirme]

## Lo que NO documentaré (y por qué)
[Para que el usuario pueda corregir el alcance]
```

Ordena por prioridad. Limita el primer plan a lo esencial (normalmente 3 a 6 páginas); el usuario puede pedir más.

### Fase 4: Escribir

Solo tras la aprobación. Para cada página:

1. Copia la plantilla de su tipo desde `assets/plantillas/` (`tutorial.md`, `guia-practica.md`, `referencia.md` o `explicacion.md`), rellénala y borra todos los comentarios `<!-- PLANTILLA ... -->` y los textos `<entre ángulos>`.
2. Escribe únicamente lo que corresponde a ese tipo.
3. Todo dato verificable debe venir del código o de la configuración, no de tu memoria.
4. Marca con `[VERIFICAR: ...]` lo que no puedas comprobar.
5. Enlaza a otras páginas en vez de repetir contenido. Si un tutorial necesita un concepto, enlaza a la explicación.

### Fase 5: Verificar

Antes de dar por terminado el trabajo:

- Ejecuta los comandos de instalación y los ejemplos de código que puedas ejecutar. Si no puedes, márcalos como no verificados.
- Comprueba que los enlaces internos apunten a archivos que existen.
- Revisa que cada página tenga un solo tipo y que cumpla las reglas de la tabla.
- Si escribiste comentarios en el código, confirma con `git diff` que solo cambiaron comentarios y que ninguna razón quedó sin respaldo (o está marcada con `VERIFICAR`).
- Ejecuta el validador y corrige todos los errores que reporte (los avisos, revísalos y decide):

  ```bash
  python <ruta-de-la-skill>/scripts/validar_docs.py <ruta-del-proyecto>
  ```

  Comprueba metadatos, secciones obligatorias por tipo, enlaces internos, restos de plantilla y coherencia entre `verificado` y las marcas `[VERIFICAR]`. No juzga la calidad de la redacción: eso sigue siendo tu trabajo.
- Entrega un informe breve: qué escribiste, qué quedó marcado con `[VERIFICAR]`, y qué comprobaciones hiciste o no pudiste hacer.

## Cuando te activan por cambios en el código

Si el usuario o un hook te pide revisar la documentación tras cambios, **no hagas un análisis completo del proyecto**. En su lugar:

1. Ejecuta el detector, que descarta el ruido (formato, tests, comentarios) y señala qué cambios tocan algo documentado:

   ```bash
   python <ruta-de-la-skill>/scripts/detectar_cambios.py <ruta-del-proyecto>
   ```

   Por defecto analiza los cambios sin confirmar. Con `--staged` analiza solo lo preparado, y con `--base main...HEAD` compara contra otra rama. Cada cambio sale con prioridad ALTA (la documentación ya lo menciona y probablemente quedó desactualizada) o MEDIA (puede faltar documentación). Es una heurística: verifica leyendo el código antes de actuar.
2. Decide si tocan algo que la documentación describe: firmas públicas, parámetros, opciones de configuración, comandos, requisitos de instalación, comportamiento documentado. Los cambios internos, de formato o de tests que no alteran el comportamiento no requieren documentación.
3. Si ningún cambio es relevante, dilo en una línea y termina. Eso es un resultado válido.
4. Revisa también si los cambios dejaron desactualizados los comentarios de las funciones modificadas (el de encima: entradas, salidas, éxito y fallo; y los de dentro: decisiones que ya no aplican).
5. Si hay cambios relevantes, propone un plan **solo para las páginas y comentarios afectados**, con el mismo formato de la Fase 3. La referencia y las guías prácticas son las que más se desactualizan; los tutoriales y las explicaciones cambian menos.

## Ubicación y estructura

Por defecto, la documentación va en un directorio `docs/` en la **raíz del proyecto**, a menos que el usuario indique otra ubicación. Si el proyecto ya tiene documentación en otro lugar (`documentation/`, `wiki/`, etc.), no la muevas por tu cuenta: menciónalo en el plan y pregunta qué prefiere el usuario.

Estructura por defecto, una carpeta por tipo:

```
docs/
├── tutoriales/
├── guias/
├── referencia/
└── explicacion/
```

Crea solo las carpetas que el plan aprobado necesite. Una carpeta vacía es ruido.

`README.md` y `AGENTS.md` (o `llms.txt`) se quedan en la raíz del proyecto, porque es donde humanos y herramientas los buscan primero. Deben enlazar a `docs/`.

Indica siempre la ruta completa de cada página en el plan (por ejemplo `docs/referencia/cli.md`), para que el usuario vea dónde va a quedar todo antes de aprobar.

## Comentarios en el código

Los comentarios son documentación que vive junto al código, y por eso se desactualizan menos que una página en `docs/`. Siguen la misma lógica de Diátaxis: el comentario sobre la función es **referencia** (qué hace) y los comentarios de dentro son **explicación** (por qué es así). No mezcles ambos.

### Encima de cada función

Describe en lenguaje natural y de forma breve:

- Qué hace, en una frase.
- Qué entra: los datos que recibe y las condiciones que deben cumplir.
- Qué sale: lo que devuelve.
- Cómo se define el éxito y el fallo: cuándo se considera que salió bien, y qué pasa cuando no (qué devuelve o qué error lanza, y si deja algo a medias).
- Efectos secundarios, si los hay (escribe en disco, llama a un servicio, modifica un argumento).

Usa el formato propio del lenguaje (docstring en Python, JSDoc/TSDoc en JavaScript y TypeScript, `///` en Rust, Javadoc en Java) y respeta el estilo que el proyecto ya use. Si la firma ya declara los tipos, no los repitas en el comentario.

```python
def cobrar_pedido(pedido, metodo_pago):
    """Cobra un pedido con el método de pago indicado.

    Recibe un pedido confirmado (con al menos un ítem) y un método de pago válido.
    Devuelve el comprobante del cobro.
    Éxito: el banco aprueba el cobro y el pedido queda marcado como pagado.
    Fallo: lanza PagoRechazado si el banco rechaza el cobro (el pedido no cambia);
    lanza ValueError si el pedido está vacío.
    """
```

### Dentro de la función

Comenta las **decisiones**: por qué el código está escrito de esa manera y no de otra. Por ejemplo, una restricción de un servicio externo, un compromiso entre velocidad y claridad, un orden de pasos que no se puede alterar, un valor que parece arbitrario, o una solución provisional a un problema conocido.

No narres lo que hace cada línea. Eso ya lo dice el código y el comentario sería redundante: además, se desactualiza en cuanto alguien cambia la línea y olvida el comentario.

```python
# Mal: repite lo que ya dice la línea
# Multiplica el monto por 100 y lo redondea
monto_centavos = round(monto * 100)

# Bien: explica la decisión
# La pasarela de pago solo acepta enteros en centavos. Con decimales
# aparecen errores de redondeo en el total cobrado.
monto_centavos = round(monto * 100)
```

Prueba rápida: si borrar el comentario no hace perder nada que el código no diga ya, bórralo. Si el código es confuso y necesita un comentario para entenderse, sugiere al usuario renombrar o simplificar, pero no lo refactorices por tu cuenta.

### Qué es una función trivial (y no se comenta)

Una función es trivial solo si cumple **todos** estos criterios a la vez:

1. **El nombre y la firma lo dicen todo.** Quien lea solo el nombre, los parámetros y el tipo de retorno sabe qué entra y qué sale, sin ambigüedad.
2. **Hace una sola operación.** Asignar, devolver un campo, delegar en otra función sin transformar nada, o una comparación o cálculo elemental. La cantidad de líneas es una pista, no el criterio: una función de dos líneas con una solución provisional no es trivial.
3. **No toma decisiones.** No tiene ramas que codifiquen una regla del negocio, ni valores arbitrarios, ni soluciones provisionales, ni un orden de pasos que importe.
4. **No tiene modos de fallo propios.** No lanza errores, no devuelve valores especiales (`None`, `-1`, cadena vacía) para indicar un problema, y no depende de condiciones que el llamador deba cumplir.
5. **No tiene efectos secundarios.** No escribe en disco, no llama a servicios, no modifica sus argumentos ni estado compartido.

Si falla **cualquiera** de los cinco, no es trivial y lleva su comentario.

Suelen ser triviales: un getter o setter simple, un constructor que solo asigna campos, una función que renombra o delega a otra sin cambiar nada, y un método estándar con comportamiento obvio (por ejemplo, `esta_vacio()` que devuelve `len(items) == 0`).

Nunca son triviales, aunque sean cortas: cualquier función que acceda a red, disco o base de datos, que valide o transforme datos de entrada, que lance o capture errores, que contenga una regla del negocio (por ejemplo, un descuento o un umbral), o que existe por un motivo que el nombre no cuenta.

```python
# Trivial: no se comenta
def nombre_completo(self):
    return f"{self.nombre} {self.apellido}"

# No trivial, aunque sea corta: se comenta
def aplicar_descuento(total, cliente):
    """Devuelve el total con el descuento del cliente aplicado.

    Éxito: devuelve el total reducido. Los clientes sin categoría no tienen descuento.
    Fallo: lanza ValueError si el total es negativo.
    """
    if total < 0:
        raise ValueError("El total no puede ser negativo")
    # El descuento de clientes frecuentes está fijado por contrato en 10%.
    return total * 0.9 if cliente.es_frecuente else total
```

Prueba de desempate: *¿podría alguien equivocarse al usar o modificar esta función sin leer su cuerpo?* Si la respuesta es sí, o dudas, no es trivial.

### No inventar el porqué

El código muestra qué se hace, casi nunca por qué. Escribe una decisión solo si se deduce de algo concreto: el propio código, los tests, el historial de commits, comentarios o documentación existentes, o lo que el usuario te haya dicho. Si no encuentras la razón, escribe `# VERIFICAR: ¿por qué se hace X?` o pregunta. Una justificación que suena razonable pero que inventaste es peor que ninguna, porque el siguiente lector confiará en ella.

### Reglas de seguridad

- **Cambia solo comentarios.** No toques lógica, nombres ni formato. Al terminar, revisa con `git diff` que las únicas diferencias sean comentarios.
- **Incluye los comentarios en el plan** (qué archivos y funciones vas a comentar), igual que el resto de la documentación. Prioriza la API pública y las funciones con lógica poco obvia. No comentes las funciones triviales, tal como se definen arriba.
- **Corrige los comentarios desactualizados.** Si un comentario existente contradice el código, actualízalo o márcalo con `VERIFICAR`; un comentario falso es peor que no tener ninguno. No borres comentarios que expliquen un porqué sin preguntar.
- **Idioma:** español por defecto. Si el proyecto ya comenta de forma consistente en otro idioma, pregunta al usuario cuál prefiere. Los nombres de variables y funciones no se traducen.

## Formato de página

Cada página empieza con este bloque de metadatos, que ayuda a agentes y herramientas a clasificarla:

```markdown
---
tipo: referencia        # tutorial | guia-practica | referencia | explicacion
verificado: no          # si | parcial | no
---
```

Significado de `verificado`:

- `si`: ejecutaste todos los comandos y ejemplos de la página y funcionan como se describe.
- `parcial`: ejecutaste algunos; los demás están marcados con `[VERIFICAR]`.
- `no`: no ejecutaste nada (por ejemplo, una explicación sin comandos, o un entorno donde no pudiste correrlos).

Si cambias el contenido de una página, vuelve a evaluar este campo. Nunca lo dejes en `si` por inercia.

Después va un título claro y una o dos frases que digan para qué sirve la página y a quién.

## Estilo de escritura

- Español neutro, frases cortas, voz activa y un tono directo.
- Términos técnicos estándar se dejan como son (`commit`, `endpoint`, `framework`); si el lector podría no conocerlos, defínelos la primera vez o enlaza a la explicación.
- Comandos y código siempre en bloques de código, con el lenguaje indicado.
- Nada de relleno ("como es bien sabido", "cabe destacar"). Si una frase no ayuda al lector, quítala.
- Un nombre por concepto: no alternes sinónimos para la misma cosa.

## Punto de entrada para agentes

Si el proyecto no lo tiene, propón en el plan un `AGENTS.md` (o `llms.txt`) breve que diga qué es el proyecto, cómo se instala y ejecuta, dónde está cada tipo de documentación y qué convenciones seguir. Debe apuntar a las páginas existentes, no duplicar su contenido.

## Recursos (se cargan solo cuando hacen falta)

- `references/tipos.md`: ejemplos buenos y malos de cada tipo de documentación.
- `assets/plantillas/`: esqueletos de tutorial, guía práctica, referencia y explicación.
- `scripts/inventariar_proyecto.py`: informe de hechos del proyecto y de su documentación existente.
- `scripts/detectar_cambios.py`: detecta si unos cambios afectan algo documentado.
- `scripts/validar_docs.py`: comprueba enlaces, metadatos y estructura de las páginas.

Todos los scripts usan solo la biblioteca estándar de Python 3.8 o superior, aceptan `--help` y `--json` (salida para máquinas), y no modifican ningún archivo del proyecto.

Si Python no está disponible o un script falla, continúa sin él siguiendo las reglas de este documento e informa al usuario de que esa comprobación no se pudo hacer.
