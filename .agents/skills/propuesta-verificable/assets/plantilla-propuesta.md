# Propuesta: [nombre corto]

Fecha: AAAA-MM-DD · Nivel 2 · Estado: pendiente
Viene de: [ruta de la propuesta que corrige] | no corrige ninguna
Hereda sin cambios: [ids como S2, R1, C3] | no aplica

## Para decidir

**Qué cambia:** [una frase, en lo que verá quien usa la app]

**Para qué:** [qué problema resuelve y cómo se ve el éxito]

**Ejemplo:** [entrada] → hoy: [A] → después: [B]

**Qué voy a hacer:**
1. [una frase por paso, máximo 5]

**Qué podría romperse:** [máximo 2, concretos] | nada encontrado (busqué: [qué])

**Qué no voy a tocar:** [1 a 3 puntos]

**Necesito de ti:**
1. [pregunta o decisión]
   Opciones: [A / B / no sé]
   Si no respondes, uso: [opción]
   Cambia el plan si: [qué]

Tendrás que mirar tú: [lo que no se puede comprobar con código] | nada
[o escribir: solo aprobar]

---

## Detalle para verificar

### Suposiciones (lo que doy por cierto sin haberlo comprobado)

S1 · [afirmación]
  Si es falso: [cómo cambia el plan]
  Se prueba antes de editar: sí ([por qué esta]) | no
  Resultado de la prueba: [se completa tras la aprobación]

### Reglas del proyecto que respeto (máximo 3)

R1 · [regla]
  Fuente: [archivo, propuesta o "respuesta humana, AAAA-MM-DD"]
  Por eso: [qué no haré]
  Se comprueba con: [C#] | solo tú (no hay test)

### Comprobaciones (criterios de aceptación; no se cambian sin aprobación)

C1 · [título en una frase] — [normal | extremo | entrada inválida | no debe cambiar]
  Situación real: [qué hace la persona que usa la app]
  Dado que [estado inicial concreto]
  Cuando [acción concreta]
  Entonces [resultado observable, con un valor concreto]
  Test: [ruta; se escribe tras la aprobación] · Estado previo: [falla | pasa]
  Se ejecuta con: `[comando exacto]`
  No cubre: [límite de esta comprobación]

### Archivos

Tocaré: [lista]
Podrían verse afectados: [lista, con la búsqueda hecha] | ninguno encontrado

### Evidencia

E1 · [afirmación sobre el código] — origen: [comando ejecutado | archivo y línea]

### Decisiones de diseño (solo si las hubo)

Alternativa descartada: [cuál] — porque [razón]
Ante imprevistos, prioriza: [X sobre Y]

### Antes de presentar, corregí

[qué detecté y corregí en la autocomprobación] | nada

### Cambios tras pruebas previas (solo si el plan cambió)

Antes: [lo que decía] · Ahora: [lo que dice] · Por qué: [resultado de la prueba]

---

## Resultado (lo completa el agente después de editar)

**Veredicto:** coincide con lo predicho | difiere en: [qué]

**Por comprobación:**
C1 · coincide | difiere | no pudo comprobarse
  Resultado real: [lo que se obtuvo al ejecutar]
  Método: el anunciado | distinto: [cuál y por qué]

**Reglas:**
R1 · respetada | no respetada: [qué pasó]
¿Se tocó algo que una regla prohibía? sí | no: [qué]
¿Hubo imprevistos? no | sí: [cuáles y qué prioridad se aplicó]
¿Reapareció una alternativa descartada? sí | no

**Archivos modificados:** [lista real]
¿Coinciden con los anunciados? sí | no: [cuál]

**Discrepancias:**
D1 · [descripción] — [error de juicio | de implementación | suposición falsa | información faltante | omisión | imprevisto]

**Para la próxima vez:** [una línea concreta]

**Fuera de alcance:** [una línea por hallazgo, con impacto bajo | medio] | ninguno

---

## Evaluación (solo si una suposición resultó falsa o hay una discrepancia)

Abierta por: agente | persona · Fecha: AAAA-MM-DD

**Qué se supuso:** S# · [texto]

**Qué se observó:** [el hecho] — evidencia: [comando y salida | archivo y línea]

**¿Está bien comprobado?** [qué hice para descartar que la prueba esté mal] | no pude

**Estado del proyecto ahora:** [archivos editados y tests escritos, según git status o ls] | sin editar

**Qué sigue valiendo y qué no:**
Paso 1 · sigue valiendo | hay que ajustar | deja de valer | no se sabe — [por qué]
C1 · [estado] — [por qué]
R1 · [estado] — [por qué]
S2 · [estado] — [por qué]

**Resumen de la lista:** [N] sigue valiendo · [N] hay que ajustar · [N] deja de valer · [N] no se sabe · total [N]

**Opciones:**
A · [qué se haría] — implica: [qué pasa con lo ya editado y con la propuesta]
B · [...]

**Recomiendo:** [letra] porque [razón basada en la lista de arriba]

**La alternativa prevista en "Si es falso" depende de:** [otras suposiciones sin comprobar] | nada más

**Decisión de la persona:** [se anota aquí lo que responda en el chat]

---

## Revisión humana

- [punto que solo la persona puede comprobar]: ✔ | ✘ | ?
- Detectado solo por el humano: [lo completa la persona]
