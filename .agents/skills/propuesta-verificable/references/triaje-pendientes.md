# Pendientes y triaje

Los hallazgos fuera de alcance de las propuestas de nivel 2 se juntan en un solo archivo, `propuestas/PENDIENTES.md`, para que la persona los revise en un único lugar.

## Formato de PENDIENTES.md

```
# Pendientes

| ID | Fecha | Hallazgo (1 línea) | Impacto | Origen | Estado |
|----|-------|--------------------|---------|--------|--------|
| P-001 | AAAA-MM-DD | [texto] | bajo | propuestas/AAAA-MM-DD_x.md | abierto |

Estados: abierto | hecho (enlace a su propuesta) | descartado
```

## Cómo escribir en el archivo

- Cuando una propuesta de nivel 2 pasa a "cerrada", copia sus hallazgos de "Fuera de alcance" a la tabla y anota el ID en la propuesta. Solo agrega filas: no cambies el estado de las existentes.
- Antes de agregar, busca si ya existe un hallazgo equivalente. Si existe, no lo dupliques: márcalo como "reaparece en [propuesta]".
- Una línea por hallazgo, sin explicaciones largas. Si hace falta contexto, enlaza a la propuesta de origen.
- Impacto solo "bajo" o "medio". Si crees que es más alto, no va aquí: sube a "Requiere tu decisión" en el resumen final.

## Triaje

Se ejecuta SOLO cuando la persona declara el cierre de una etapa (por ejemplo, "cerremos etapa" o "hagamos triaje"). Nunca lo inicies por tu cuenta. La razón es que la persona decide qué cuenta como etapa y cuándo quiere dedicarle atención a esto.

Aviso: si hay 10 o más pendientes abiertos, menciónalo en una línea del resumen final ("12 pendientes abiertos"). Nada más.

Al iniciar:
1. Presenta los pendientes abiertos agrupados por impacto y por módulo, sin comentarios nuevos.
2. Espera su respuesta por cada uno: hacer | descartar | esperar.
3. **Hacer:** crea una propuesta nueva con su nivel correspondiente y enlázala en la tabla.
4. **Descartar:** estado "descartado". No se borra, queda registrado.
5. **Esperar:** si un pendiente lleva 2 triajes seguidos en "esperar" sin haber causado problemas, propón descartarlo. La decisión es de la persona.
6. Si la persona menciona que algún pendiente marcado "bajo" resultó importante, regístralo al final del archivo. Sirve para saber si el impacto se está subestimando.
