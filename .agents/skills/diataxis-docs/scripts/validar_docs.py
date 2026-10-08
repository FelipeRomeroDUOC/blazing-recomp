#!/usr/bin/env python3
"""Valida la documentación Diátaxis de un proyecto.

Qué comprueba (solo cosas con respuesta exacta; la calidad de la redacción
la juzga el agente, no este script):

  - Que cada página de docs/ tenga metadatos (`tipo` y `verificado`) con valores válidos.
  - Que el `tipo` coincida con la carpeta (docs/tutoriales -> tutorial, etc.).
  - Que la página tenga las secciones obligatorias de su tipo.
  - Que los enlaces internos apunten a archivos (y secciones) que existen.
  - Que `verificado: si` no conviva con marcas [VERIFICAR] pendientes.
  - Que no queden restos de la plantilla (comentarios PLANTILLA, títulos <entre ángulos>).
  - Que los bloques de código indiquen su lenguaje (aviso).
  - Que README.md y AGENTS.md (si existen) enlacen a docs/ (aviso).

Las páginas llamadas README.md o index.md dentro de docs/ se tratan como índices:
solo se comprueban sus enlaces, no sus metadatos.

Uso:
    python validar_docs.py [ruta_del_proyecto] [--docs docs] [--json] [--estricto]

Código de salida: 0 sin errores, 1 con errores, 2 si no se pudo revisar.
Solo usa la biblioteca estándar de Python (3.8 o superior).
"""

import argparse
import json
import re
import sys
import unicodedata
from pathlib import Path
from urllib.parse import unquote

TIPOS = {"tutorial", "guia-practica", "referencia", "explicacion"}
VERIFICADO = {"si", "parcial", "no"}

CARPETA_A_TIPO = {
    "tutoriales": "tutorial",
    "guias": "guia-practica",
    "referencia": "referencia",
    "explicacion": "explicacion",
}

SECCIONES_REQUERIDAS = {
    "tutorial": ["Antes de empezar", "Pasos", "Resultado"],
    "guia-practica": ["Requisitos", "Pasos", "Comprobar el resultado"],
    "referencia": ["Descripción general"],
    "explicacion": ["Contexto", "Idea central", "Alternativas y compromisos"],
}

PAGINAS_INDICE = {"readme.md", "index.md"}

RE_ENLACE = re.compile(r"!?\[[^\]]*\]\(\s*<?([^)\s>]+)>?(?:\s+\"[^\"]*\")?\s*\)")
RE_ENLACE_REF = re.compile(r"^\s{0,3}\[[^\]]+\]:\s*<?(\S+?)>?\s*$", re.MULTILINE)
RE_ENCABEZADO = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
RE_ESQUEMA = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.\-]*:")
RE_APERTURA_CERCA = re.compile(r"^\s{0,3}(`{3,}|~{3,})(.*)$")


# ---------------------------------------------------------------- utilidades

def normalizar(texto):
    """Minúsculas, sin tildes, sin signos: para comparar títulos."""
    texto = unicodedata.normalize("NFKD", texto)
    texto = "".join(c for c in texto if not unicodedata.combining(c))
    texto = re.sub(r"[`*_:¿?¡!.]", "", texto.lower())
    return re.sub(r"\s+", " ", texto).strip()


def slug(texto):
    """Ancla de un título, al estilo de GitHub."""
    texto = re.sub(r"[`*]", "", texto).strip().lower()
    texto = re.sub(r"[^\w\- ]", "", texto)
    return texto.replace(" ", "-")


def quitar_comentarios_html(texto):
    """Elimina <!-- ... --> conservando los saltos de línea (los números de línea no cambian)."""
    return re.sub(
        r"<!--.*?-->",
        lambda m: "\n" * m.group(0).count("\n"),
        texto,
        flags=re.DOTALL,
    )


def procesar_codigo(texto):
    """Devuelve (texto_sin_bloques_de_codigo, cercas_sin_lenguaje).

    Los bloques de código se reemplazan por líneas vacías para conservar la numeración.
    `cercas_sin_lenguaje` es la lista de números de línea donde se abre un bloque sin lenguaje.
    """
    salida = []
    sin_lenguaje = []
    cerca = None  # (caracter, longitud) mientras estamos dentro de un bloque
    for num, linea in enumerate(texto.split("\n"), start=1):
        m = RE_APERTURA_CERCA.match(linea)
        if cerca is None:
            if m:
                cerca = (m.group(1)[0], len(m.group(1)))
                if not m.group(2).strip():
                    sin_lenguaje.append(num)
            else:
                salida.append(linea)
                continue
            salida.append("")
        else:
            if m and m.group(1)[0] == cerca[0] and len(m.group(1)) >= cerca[1] and not m.group(2).strip():
                cerca = None
            salida.append("")
    return "\n".join(salida), sin_lenguaje


def quitar_codigo_en_linea(texto):
    return re.sub(r"`[^`\n]*`", lambda m: " " * len(m.group(0)), texto)


def leer_frontmatter(texto):
    """Devuelve (dict | None, linea_donde_termina). None si no hay bloque de metadatos."""
    lineas = texto.split("\n")
    if not lineas or lineas[0].strip() != "---":
        return None, 0
    datos = {}
    for i in range(1, len(lineas)):
        linea = lineas[i]
        if linea.strip() == "---":
            return datos, i + 1
        sin_comentario = re.sub(r"\s+#.*$", "", linea).strip()
        if ":" in sin_comentario:
            clave, valor = sin_comentario.split(":", 1)
            datos[clave.strip()] = valor.strip().strip("\"'")
    return None, 0  # nunca se cerró


def encabezados(texto_limpio):
    """Lista de (nivel, texto, linea) de los títulos Markdown."""
    resultado = []
    for num, linea in enumerate(texto_limpio.split("\n"), start=1):
        m = RE_ENCABEZADO.match(linea)
        if m:
            resultado.append((len(m.group(1)), m.group(2), num))
    return resultado


def anclas_de(ruta, cache):
    if ruta in cache:
        return cache[ruta]
    try:
        texto = ruta.read_text(encoding="utf-8", errors="replace")
    except OSError:
        cache[ruta] = set()
        return cache[ruta]
    limpio = quitar_comentarios_html(procesar_codigo(texto)[0])
    usados = {}
    anclas = set()
    for _, titulo, _ in encabezados(limpio):
        base = slug(titulo)
        n = usados.get(base, 0)
        anclas.add(base if n == 0 else f"{base}-{n}")
        usados[base] = n + 1
    cache[ruta] = anclas
    return anclas


# ---------------------------------------------------------------- revisión

class Revisor:
    def __init__(self, raiz, docs_dir):
        self.raiz = raiz
        self.docs_dir = docs_dir
        self.problemas = []
        self.pendientes_verificar = 0
        self.cache_anclas = {}

    def rel(self, ruta):
        try:
            return ruta.relative_to(self.raiz).as_posix()
        except ValueError:
            return ruta.as_posix()

    def agregar(self, nivel, ruta, linea, mensaje):
        self.problemas.append(
            {"nivel": nivel, "archivo": self.rel(ruta), "linea": linea, "mensaje": mensaje}
        )

    # ------------------------------------------------------------ enlaces
    def revisar_enlaces(self, ruta, texto_limpio):
        """Comprueba los enlaces internos. Devuelve los destinos que resuelven a archivos existentes."""
        destinos_validos = []
        sin_codigo = quitar_codigo_en_linea(texto_limpio)
        encontrados = []
        for num, linea in enumerate(sin_codigo.split("\n"), start=1):
            for m in RE_ENLACE.finditer(linea):
                encontrados.append((num, m.group(1)))
        for m in RE_ENLACE_REF.finditer(sin_codigo):
            num = sin_codigo.count("\n", 0, m.start()) + 1
            encontrados.append((num, m.group(1)))

        for num, linea in enumerate(sin_codigo.split("\n"), start=1):
            if re.search(r"\]\(\s*<[^>]*>[^)]*\)", linea):
                self.agregar("error", ruta, num, "enlace sin rellenar de la plantilla: (<...>)")

        for num, destino in encontrados:
            if RE_ESQUEMA.match(destino):
                continue  # http:, https:, mailto:, etc.
            ruta_parte, _, ancla = destino.partition("#")
            if not ruta_parte:
                destino_abs = ruta
            elif ruta_parte.startswith("/"):
                destino_abs = self.raiz / unquote(ruta_parte).lstrip("/")
            else:
                destino_abs = ruta.parent / unquote(ruta_parte)
            try:
                existe = destino_abs.exists()
            except OSError:
                existe = False
            if not existe:
                self.agregar("error", ruta, num, f"enlace roto: {destino}")
                continue
            destinos_validos.append(destino_abs.resolve())
            if ancla and destino_abs.is_file() and destino_abs.suffix.lower() == ".md":
                if unquote(ancla).lower() not in anclas_de(destino_abs, self.cache_anclas):
                    self.agregar("aviso", ruta, num, f"la sección #{ancla} no existe en {self.rel(destino_abs.resolve())}")
        return destinos_validos

    # ------------------------------------------------------------ páginas
    def revisar_pagina(self, ruta):
        texto = ruta.read_text(encoding="utf-8", errors="replace")
        sin_comentarios = quitar_comentarios_html(texto)
        sin_codigo, cercas_sin_lenguaje = procesar_codigo(texto)
        limpio = quitar_comentarios_html(sin_codigo)

        self.revisar_enlaces(ruta, limpio)

        es_indice = ruta.name.lower() in PAGINAS_INDICE
        if es_indice:
            return

        # Restos de plantilla
        for m in re.finditer(r"<!--\s*PLANTILLA", texto):
            linea = texto.count("\n", 0, m.start()) + 1
            self.agregar("error", ruta, linea, "quedó un comentario de plantilla sin borrar (<!-- PLANTILLA ...)")
        for nivel, titulo, linea in encabezados(limpio):
            if nivel <= 2 and titulo.startswith("<") and titulo.endswith(">"):
                self.agregar("error", ruta, linea, f"título sin rellenar de la plantilla: {titulo}")

        # Metadatos
        meta, fin_meta = leer_frontmatter(texto)
        if meta is None:
            self.agregar("error", ruta, 1, "faltan los metadatos al inicio (bloque --- con `tipo` y `verificado`)")
            tipo = None
            verificado = None
        else:
            tipo = meta.get("tipo")
            verificado = meta.get("verificado")
            if tipo is None:
                self.agregar("error", ruta, 1, "falta el campo `tipo` en los metadatos")
            elif tipo not in TIPOS:
                self.agregar("error", ruta, 1, f"`tipo` inválido: '{tipo}' (válidos: {', '.join(sorted(TIPOS))})")
            if verificado is None:
                self.agregar("error", ruta, 1, "falta el campo `verificado` en los metadatos")
            elif verificado not in VERIFICADO:
                self.agregar("error", ruta, 1, f"`verificado` inválido: '{verificado}' (válidos: {', '.join(sorted(VERIFICADO))})")

        # Carpeta y tipo
        try:
            partes = ruta.relative_to(self.docs_dir).parts
        except ValueError:
            partes = ()
        if len(partes) > 1 and partes[0] in CARPETA_A_TIPO and tipo in TIPOS:
            esperado = CARPETA_A_TIPO[partes[0]]
            if tipo != esperado:
                self.agregar(
                    "error", ruta, 1,
                    f"está en docs/{partes[0]}/ (tipo '{esperado}') pero declara tipo '{tipo}'",
                )

        # Secciones obligatorias
        h2 = [normalizar(t) for n, t, _ in encabezados(limpio) if n == 2]
        if tipo in SECCIONES_REQUERIDAS:
            for requerida in SECCIONES_REQUERIDAS[tipo]:
                objetivo = normalizar(requerida)
                if not any(h.startswith(objetivo) for h in h2):
                    self.agregar("error", ruta, None, f"falta la sección '## {requerida}' (obligatoria en tipo '{tipo}')")

        # Un único título principal
        h1 = [e for e in encabezados(limpio) if e[0] == 1]
        if len(h1) == 0:
            self.agregar("aviso", ruta, None, "no tiene título principal (# Título)")
        elif len(h1) > 1:
            self.agregar("aviso", ruta, h1[1][2], "tiene más de un título principal (#)")

        # Marcas [VERIFICAR] y coherencia con `verificado`
        marcas = len(re.findall(r"\[VERIFICAR", sin_comentarios))
        self.pendientes_verificar += marcas
        if verificado == "si" and marcas:
            self.agregar("error", ruta, None, f"declara `verificado: si` pero tiene {marcas} marca(s) [VERIFICAR] pendiente(s)")
        if verificado == "parcial" and not marcas:
            self.agregar("aviso", ruta, None, "declara `verificado: parcial` pero no hay ninguna marca [VERIFICAR] que indique qué falta")

        # Bloques de código sin lenguaje
        for linea in cercas_sin_lenguaje:
            self.agregar("aviso", ruta, linea, "bloque de código sin lenguaje indicado (por ejemplo ```bash)")

    def revisar_entrada(self, ruta, hay_paginas_docs):
        """README.md / AGENTS.md de la raíz: enlaces válidos y enlace a docs/."""
        texto = ruta.read_text(encoding="utf-8", errors="replace")
        limpio = quitar_comentarios_html(procesar_codigo(texto)[0])
        destinos = self.revisar_enlaces(ruta, limpio)
        if hay_paginas_docs:
            enlaza = any(self._dentro_de_docs(d) for d in destinos)
            if not enlaza:
                self.agregar("aviso", ruta, None, "no enlaza a ninguna página de docs/")

    def _dentro_de_docs(self, destino):
        try:
            destino.relative_to(self.docs_dir.resolve())
            return True
        except ValueError:
            return False


def paginas_markdown(docs_dir):
    paginas = []
    for ruta in sorted(docs_dir.rglob("*.md")):
        partes = ruta.relative_to(docs_dir).parts
        if any(p.startswith(".") for p in partes):
            continue
        paginas.append(ruta)
    return paginas


# ---------------------------------------------------------------- salida

def imprimir_texto(revisor, n_paginas):
    por_archivo = {}
    for p in revisor.problemas:
        por_archivo.setdefault(p["archivo"], []).append(p)
    for archivo in sorted(por_archivo):
        print(archivo)
        for p in sorted(por_archivo[archivo], key=lambda x: (x["linea"] or 0)):
            etiqueta = "ERROR" if p["nivel"] == "error" else "AVISO"
            ubicacion = f"línea {p['linea']}: " if p["linea"] else ""
            print(f"  {etiqueta}  {ubicacion}{p['mensaje']}")
        print()
    errores = sum(1 for p in revisor.problemas if p["nivel"] == "error")
    avisos = sum(1 for p in revisor.problemas if p["nivel"] == "aviso")
    print(
        f"Resumen: {n_paginas} página(s) revisada(s), {errores} error(es), {avisos} aviso(s), "
        f"{revisor.pendientes_verificar} marca(s) [VERIFICAR] pendiente(s)."
    )
    if errores == 0 and avisos == 0:
        print("Todo en orden.")


def main(argv=None):
    parser = argparse.ArgumentParser(description="Valida la documentación Diátaxis de un proyecto.")
    parser.add_argument("proyecto", nargs="?", default=".", help="ruta del proyecto (por defecto, el directorio actual)")
    parser.add_argument("--docs", default="docs", help="directorio de documentación relativo al proyecto (por defecto: docs)")
    parser.add_argument("--json", action="store_true", help="salida en JSON, pensada para agentes y herramientas")
    parser.add_argument("--estricto", action="store_true", help="los avisos también hacen fallar (código de salida 1)")
    args = parser.parse_args(argv)

    raiz = Path(args.proyecto).resolve()
    docs_dir = (raiz / args.docs).resolve()
    if not raiz.is_dir():
        print(f"No existe el directorio del proyecto: {raiz}", file=sys.stderr)
        return 2
    if not docs_dir.is_dir():
        print(f"No existe el directorio de documentación: {docs_dir}", file=sys.stderr)
        return 2

    revisor = Revisor(raiz, docs_dir)
    paginas = paginas_markdown(docs_dir)
    for ruta in paginas:
        revisor.revisar_pagina(ruta)

    hay_paginas = any(p.name.lower() not in PAGINAS_INDICE for p in paginas)
    for nombre in ("README.md", "AGENTS.md"):
        ruta = raiz / nombre
        if ruta.is_file():
            revisor.revisar_entrada(ruta, hay_paginas)

    errores = sum(1 for p in revisor.problemas if p["nivel"] == "error")
    avisos = sum(1 for p in revisor.problemas if p["nivel"] == "aviso")

    if args.json:
        print(json.dumps(
            {
                "paginas_revisadas": len(paginas),
                "errores": errores,
                "avisos": avisos,
                "verificar_pendientes": revisor.pendientes_verificar,
                "problemas": revisor.problemas,
            },
            ensure_ascii=False, indent=2,
        ))
    else:
        imprimir_texto(revisor, len(paginas))

    if errores or (args.estricto and avisos):
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
