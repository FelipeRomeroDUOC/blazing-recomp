#!/usr/bin/env python3
"""Detecta si unos cambios en el código afectan a algo que la documentación describe.

Es un detector barato y aproximado: no entiende el código, aplica reglas simples sobre
`git diff`. Su trabajo es descartar el ruido (formato, tests, comentarios, archivos
internos) y avisar solo cuando un cambio toca algo que suele estar documentado:

  - Símbolos públicos nuevos, eliminados/renombrados o con la firma modificada
    (funciones, clases, tipos exportados).
  - Opciones de línea de comandos (--opcion) nuevas o eliminadas.
  - Variables de entorno nuevas o eliminadas.
  - Archivos de configuración o dependencias modificados (package.json, pyproject.toml,
    Dockerfile, .env.example, etc.).
  - Archivos renombrados o movidos que la documentación menciona.

Para cada cambio busca en la documentación (docs/, README.md, AGENTS.md, llms.txt) dónde
se menciona. Si se menciona, la prioridad es ALTA (la documentación probablemente quedó
desactualizada); si no, MEDIA (puede faltar documentación).

Además lista, como información aparte, las funciones cuyo cuerpo cambió, para que se
revise si su comentario sigue siendo correcto. Esa lista no cuenta como "relevante".

Limitaciones conocidas: no detecta firmas escritas en varias líneas (aparecerán como
función de cuerpo modificado) y los lenguajes compatibles son Python, JavaScript/TypeScript,
Go, Rust, Java, C# y Ruby. Los resultados son pistas, no veredictos.

Uso:
    python detectar_cambios.py [ruta_del_proyecto] [opciones]

    (sin opciones)     cambios sin confirmar respecto a HEAD, incluidos archivos nuevos
    --staged           solo lo que está en el área de preparación (útil en pre-commit)
    --base REV         compara contra una revisión, por ejemplo: --base main...HEAD
    --diff ARCHIVO     lee un diff ya hecho desde un archivo (o '-' para la entrada estándar)
    --docs DIR         directorio de documentación (por defecto: docs)
    --json             salida en JSON
    --fallar-si-relevante   código de salida 1 si hay cambios relevantes

Código de salida: 0 normal, 1 relevante (solo con --fallar-si-relevante), 2 error.
Solo usa la biblioteca estándar de Python (3.8 o superior) y git.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

# ---------------------------------------------------------------- lenguajes


def _publico_python(nombre, linea):
    return not nombre.startswith("_") or nombre == "__init__"


def _publico_exportado(nombre, linea):
    return "export" in linea


def _publico_go(nombre, linea):
    return nombre[:1].isupper()


def _publico_rust(nombre, linea):
    return re.match(r"\s*pub\b", linea) is not None


def _publico_java(nombre, linea):
    return re.match(r"\s*public\b", linea) is not None


def _publico_ruby(nombre, linea):
    return not nombre.startswith("_")


_JAVA_DEFS = [
    (re.compile(r"^\s*(?:(?:public|protected|private|static|final|abstract|sealed|partial)\s+)*(?:class|interface|enum|record|struct)\s+(\w+)"), "tipo"),
    (re.compile(r"^\s*public\s+(?:(?:static|final|abstract|synchronized|default|virtual|override|async)\s+)*(?:<[^>]+>\s+)?[\w<>\[\],.? ]+?\s+(\w+)\s*\("), "función"),
    (re.compile(r"^\s*public\s+(\w+)\s*\("), "constructor"),
]

LENGUAJES = {
    "python": {
        "ext": {".py"},
        "comentario": ("#",),
        "publico": _publico_python,
        "defs": [
            (re.compile(r"^\s*(?:async\s+)?def\s+(\w+)\s*\("), "función"),
            (re.compile(r"^\s*class\s+(\w+)"), "clase"),
        ],
    },
    "javascript/typescript": {
        "ext": {".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"},
        "comentario": ("//", "/*", "*"),
        "publico": _publico_exportado,
        "defs": [
            (re.compile(r"^\s*(?:export\s+(?:default\s+)?)?(?:async\s+)?function\s*\*?\s*(\w+)\s*[(<]"), "función"),
            (re.compile(r"^\s*(?:export\s+(?:default\s+)?)?(?:abstract\s+)?class\s+(\w+)"), "clase"),
            (re.compile(r"^\s*export\s+(?:const|let|var)\s+(\w+)"), "constante o función"),
            (re.compile(r"^\s*export\s+(?:interface|type|enum)\s+(\w+)"), "tipo"),
            (re.compile(r"^\s*(?:module\.)?exports\.(\w+)\s*="), "exportación"),
        ],
    },
    "go": {
        "ext": {".go"},
        "comentario": ("//", "/*", "*"),
        "publico": _publico_go,
        "defs": [
            (re.compile(r"^func\s+(?:\([^)]*\)\s*)?(\w+)\s*[\[(]"), "función"),
            (re.compile(r"^type\s+(\w+)\s+"), "tipo"),
        ],
    },
    "rust": {
        "ext": {".rs"},
        "comentario": ("//",),
        "publico": _publico_rust,
        "defs": [
            (re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+)*fn\s+(\w+)"), "función"),
            (re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|type)\s+(\w+)"), "tipo"),
        ],
    },
    "java": {
        "ext": {".java"},
        "comentario": ("//", "/*", "*"),
        "publico": _publico_java,
        "defs": _JAVA_DEFS,
    },
    "c#": {
        "ext": {".cs"},
        "comentario": ("//", "/*", "*"),
        "publico": _publico_java,
        "defs": _JAVA_DEFS,
    },
    "ruby": {
        "ext": {".rb"},
        "comentario": ("#",),
        "publico": _publico_ruby,
        "defs": [
            (re.compile(r"^\s*def\s+(?:self\.)?(\w+[?!=]?)"), "función"),
            (re.compile(r"^\s*(?:class|module)\s+(\w+)"), "clase o módulo"),
        ],
    },
}

EXT_A_LENGUAJE = {}
for _nombre, _cfg in LENGUAJES.items():
    for _ext in _cfg["ext"]:
        EXT_A_LENGUAJE[_ext] = _nombre

# ---------------------------------------------------------------- clasificación de archivos

EXT_DOCS = {".md", ".rst", ".adoc", ".txt"}
LOCKFILES = {
    "package-lock.json", "yarn.lock", "pnpm-lock.yaml", "poetry.lock", "pipfile.lock",
    "cargo.lock", "go.sum", "gemfile.lock", "composer.lock", "uv.lock",
}
CONFIG_NOMBRES = {
    "package.json", "pyproject.toml", "setup.py", "setup.cfg", "requirements.txt", "pipfile",
    "cargo.toml", "go.mod", "pom.xml", "build.gradle", "build.gradle.kts", "gemfile",
    "composer.json", "dockerfile", "docker-compose.yml", "docker-compose.yaml", "compose.yml",
    "compose.yaml", "makefile", "justfile", "tox.ini", "environment.yml",
}
DIRS_IGNORADOS = {
    "node_modules", "vendor", "dist", "build", "target", "__pycache__", ".git", ".venv",
    "venv", "env", ".tox", ".idea", ".vscode", "site-packages", "coverage",
}
DIRS_TEST = {"test", "tests", "__tests__", "spec", "specs", "e2e", "testing"}
RE_ARCHIVO_TEST = re.compile(
    r"(^test_.*\.py$|.*_test\.(py|go|rb)$|.*\.(test|spec)\.[a-z]+$|.*(Test|Tests)\.(java|cs)$|.*_spec\.rb$)"
)
RE_ENV_ARCHIVO = re.compile(r"^\.env(\..+)?$", re.I)
PREFIJOS_COMENTARIO_CONFIG = ("#", "//", ";")


def clasificar(ruta, nombre_docs):
    ruta = ruta.replace("\\", "/")
    partes = ruta.split("/")
    nombre = partes[-1].lower()
    ext = os.path.splitext(nombre)[1]
    partes_dir = [p.lower() for p in partes[:-1]]
    if any(p in DIRS_IGNORADOS for p in partes_dir):
        return "otro"
    if (partes[0].lower() == nombre_docs.lower() and len(partes) > 1) or ext in EXT_DOCS:
        return "documentacion"
    if nombre in LOCKFILES:
        return "bloqueo"
    if any(p in DIRS_TEST for p in partes_dir) or RE_ARCHIVO_TEST.match(partes[-1]):
        return "test"
    if (
        nombre in CONFIG_NOMBRES
        or re.match(r"^requirements.*\.txt$", nombre)
        or nombre.startswith("dockerfile")
        or ext == ".csproj"
        or RE_ENV_ARCHIVO.match(nombre)
    ):
        return "config"
    if ext in EXT_A_LENGUAJE:
        return "codigo"
    return "otro"


# ---------------------------------------------------------------- lectura del diff

class ArchivoDiff:
    def __init__(self):
        self.ruta = None
        self.ruta_anterior = None
        self.estado = "modificado"  # modificado | nuevo | eliminado | renombrado
        self.hunks = []             # cada hunk: {"contexto": str, "agregadas": [], "eliminadas": []}
        self.binario = False

    @property
    def ruta_efectiva(self):
        return self.ruta or self.ruta_anterior


RE_CABECERA = re.compile(r"^diff --git a/(.*) b/(.*)$")
RE_HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@ ?(.*)$")


def parsear_diff(texto):
    archivos = []
    actual = None
    hunk = None
    restantes_viejas = 0
    restantes_nuevas = 0

    for linea in texto.split("\n"):
        en_hunk = restantes_viejas > 0 or restantes_nuevas > 0

        if en_hunk:
            if linea.startswith("\\"):
                continue
            if linea.startswith("-") and restantes_viejas > 0:
                hunk["eliminadas"].append(linea[1:])
                restantes_viejas -= 1
                continue
            if linea.startswith("+") and restantes_nuevas > 0:
                hunk["agregadas"].append(linea[1:])
                restantes_nuevas -= 1
                continue
            if linea.startswith(" "):
                restantes_viejas = max(0, restantes_viejas - 1)
                restantes_nuevas = max(0, restantes_nuevas - 1)
                continue
            # cualquier otra cosa: el hunk terminó antes de lo esperado
            restantes_viejas = restantes_nuevas = 0

        m = RE_CABECERA.match(linea)
        if m:
            actual = ArchivoDiff()
            actual.ruta_anterior, actual.ruta = m.group(1), m.group(2)
            archivos.append(actual)
            hunk = None
            continue
        if actual is None:
            continue
        if linea.startswith("new file mode"):
            actual.estado = "nuevo"
        elif linea.startswith("deleted file mode"):
            actual.estado = "eliminado"
        elif linea.startswith("rename from "):
            actual.ruta_anterior = linea[len("rename from "):]
            actual.estado = "renombrado"
        elif linea.startswith("rename to "):
            actual.ruta = linea[len("rename to "):]
            actual.estado = "renombrado"
        elif linea.startswith("Binary files") or linea.startswith("GIT binary patch"):
            actual.binario = True
        elif linea.startswith("--- "):
            valor = linea[4:]
            if valor == "/dev/null":
                actual.estado = "nuevo"
            elif valor.startswith("a/"):
                actual.ruta_anterior = valor[2:]
        elif linea.startswith("+++ "):
            valor = linea[4:]
            if valor == "/dev/null":
                actual.estado = "eliminado"
                actual.ruta = None
            elif valor.startswith("b/"):
                actual.ruta = valor[2:]
        else:
            h = RE_HUNK.match(linea)
            if h:
                restantes_viejas = int(h.group(2)) if h.group(2) is not None else 1
                restantes_nuevas = int(h.group(4)) if h.group(4) is not None else 1
                hunk = {"contexto": h.group(5).strip(), "agregadas": [], "eliminadas": []}
                actual.hunks.append(hunk)
    return archivos


# ---------------------------------------------------------------- obtención del diff con git

class ErrorGit(Exception):
    pass


def _git(raiz, *args, permitir_codigos=(0,)):
    try:
        r = subprocess.run(
            ["git", "-c", "core.quotepath=false"] + list(args),
            cwd=str(raiz), capture_output=True, text=True, encoding="utf-8", errors="replace",
        )
    except FileNotFoundError:
        raise ErrorGit("No se encontró git en el sistema.")
    if r.returncode not in permitir_codigos:
        raise ErrorGit((r.stderr or r.stdout).strip() or f"git {' '.join(args)} falló")
    return r.stdout


OPCIONES_DIFF = [
    "diff", "--no-color", "--no-ext-diff", "-U0", "-w", "--ignore-blank-lines",
    "--src-prefix=a/", "--dst-prefix=b/", "-M",
]


def obtener_diff(args, raiz):
    """Devuelve (texto_del_diff, descripcion_de_la_fuente)."""
    if args.diff:
        if args.diff == "-":
            return sys.stdin.read(), "diff recibido por la entrada estándar"
        return Path(args.diff).read_text(encoding="utf-8", errors="replace"), f"diff leído de {args.diff}"

    try:
        _git(raiz, "rev-parse", "--is-inside-work-tree")
    except ErrorGit:
        raise ErrorGit(f"{raiz} no es un repositorio git (o no se puede leer). Usa --diff para pasar un diff manualmente.")

    if args.staged:
        return _git(raiz, *OPCIONES_DIFF, "--cached"), "git diff --cached (área de preparación)"
    if args.base:
        return _git(raiz, *OPCIONES_DIFF, args.base), f"git diff {args.base}"

    # Por defecto: cambios sin confirmar + archivos nuevos sin seguimiento
    try:
        _git(raiz, "rev-parse", "--verify", "HEAD")
        texto = _git(raiz, *OPCIONES_DIFF, "HEAD")
        fuente = "git diff HEAD (cambios sin confirmar, incluidos archivos nuevos)"
    except ErrorGit:
        texto = _git(raiz, *OPCIONES_DIFF, "--cached")
        fuente = "repositorio sin commits: archivos preparados y nuevos"

    sin_seguimiento = _git(raiz, "ls-files", "--others", "--exclude-standard", "-z").split("\0")
    for ruta in [r for r in sin_seguimiento if r][:500]:
        completa = raiz / ruta
        try:
            if not completa.is_file() or completa.stat().st_size > 1_000_000:
                continue
        except OSError:
            continue
        texto += "\n" + _git(
            raiz, "diff", "--no-index", "--no-color", "-U0", "--src-prefix=a/", "--dst-prefix=b/",
            "--", os.devnull, ruta, permitir_codigos=(0, 1),
        )
    return texto, fuente


# ---------------------------------------------------------------- índice de la documentación

class IndiceDocs:
    def __init__(self, raiz, dir_docs):
        self.archivos = {}
        candidatos = []
        if dir_docs.is_dir():
            candidatos += sorted(dir_docs.rglob("*.md"))
        for nombre in ("README.md", "AGENTS.md", "llms.txt"):
            if (raiz / nombre).is_file():
                candidatos.append(raiz / nombre)
        for ruta in candidatos:
            try:
                rel = ruta.relative_to(raiz).as_posix()
                self.archivos[rel] = ruta.read_text(encoding="utf-8", errors="replace").split("\n")
            except (OSError, ValueError):
                continue

    def buscar(self, termino, como_palabra=True, maximo=4):
        if len(termino) < 2:
            return []
        if como_palabra:
            patron = re.compile(r"(?<![\w-])" + re.escape(termino) + r"(?![\w])")
        else:
            patron = re.compile(re.escape(termino))
        resultados = []
        for ruta, lineas in self.archivos.items():
            for numero, linea in enumerate(lineas, start=1):
                if patron.search(linea):
                    resultados.append(f"{ruta}:{numero}")
                    break
            if len(resultados) >= maximo:
                break
        return resultados


# ---------------------------------------------------------------- análisis

RE_CADENA = re.compile(r'"([^"\n]*)"|\'([^\'\n]*)\'')
RE_OPCION = re.compile(r"(?<![\w-])(--[a-z][a-z0-9]*(?:-[a-z0-9]+)*)")
RE_ENV = [
    re.compile(
        r"""(?:environ(?:\.get)?\s*[\(\[]\s*|getenv\s*\(\s*|GetEnvironmentVariable\s*\(\s*|env::var\s*\(\s*)["']([A-Za-z_][A-Za-z0-9_]*)["']""",
        re.I,
    ),
    re.compile(r"""\bENV\s*(?:\[\s*|\.fetch\(\s*)["']([A-Za-z_][A-Za-z0-9_]*)["']"""),
    re.compile(r"process\.env\.([A-Za-z_][A-Za-z0-9_]*)"),
    re.compile(r"""process\.env\[\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*\]"""),
]
RE_VAR_ENV_ARCHIVO = re.compile(r"^\s*(?:export\s+)?([A-Z][A-Z0-9_]*)\s*=")


def normalizar_firma(linea):
    return re.sub(r"\s+", " ", linea.strip()).rstrip(":{ ").strip()


def es_significativa(linea, prefijos_comentario):
    """False si la línea está vacía o es solo un comentario."""
    t = linea.strip()
    if not t:
        return False
    return not any(t.startswith(p) for p in prefijos_comentario)


def extraer_definicion(linea, cfg):
    for regex, tipo in cfg["defs"]:
        m = regex.match(linea)
        if m:
            return m.group(1), tipo
    return None


def extraer_opciones(lineas):
    encontradas = set()
    for linea in lineas:
        for m in RE_CADENA.finditer(linea):
            contenido = m.group(1) if m.group(1) is not None else m.group(2)
            for o in RE_OPCION.finditer(contenido or ""):
                encontradas.add(o.group(1))
    return encontradas


def extraer_variables_entorno(lineas, es_archivo_env=False):
    encontradas = set()
    for linea in lineas:
        for regex in RE_ENV:
            for m in regex.finditer(linea):
                encontradas.add(m.group(1))
        if es_archivo_env:
            m = RE_VAR_ENV_ARCHIVO.match(linea)
            if m:
                encontradas.add(m.group(1))
    return encontradas


class Analizador:
    def __init__(self, indice, nombre_docs):
        self.indice = indice
        self.nombre_docs = nombre_docs
        self.cambios = []
        self.funciones_modificadas = []
        self.resumen_archivos = defaultdict(int)
        self.docs_modificadas = set()

    def agregar(self, categoria, nombre, archivo, mencionado_en, **extra):
        cambio = {
            "categoria": categoria,
            "nombre": nombre,
            "archivo": archivo,
            "prioridad": "alta" if mencionado_en else "media",
            "documentacion": mencionado_en,
        }
        cambio.update(extra)
        self.cambios.append(cambio)

    def _menciones(self, termino, como_palabra=True):
        encontrados = self.indice.buscar(termino, como_palabra)
        return [
            f"{e} (también cambió en este diff)" if e.rsplit(":", 1)[0] in self.docs_modificadas else e
            for e in encontrados
        ]

    # ------------------------------------------------------------ por archivo
    def analizar(self, archivos):
        for a in archivos:
            ruta = a.ruta_efectiva
            if not ruta:
                continue
            tipo = clasificar(ruta, self.nombre_docs)
            self.resumen_archivos[tipo] += 1
            if tipo == "documentacion":
                self.docs_modificadas.add(ruta.replace("\\", "/"))

        for a in archivos:
            ruta = a.ruta_efectiva
            if not ruta or a.binario:
                continue
            tipo = clasificar(ruta, self.nombre_docs)
            if a.estado == "renombrado" and tipo in ("codigo", "config") and a.ruta_anterior:
                menciones = self._menciones(a.ruta_anterior, como_palabra=False)
                if menciones:
                    self.agregar("archivo movido o renombrado", a.ruta_anterior, ruta, menciones,
                                 antes=a.ruta_anterior, ahora=ruta)
            if tipo == "codigo":
                self._analizar_codigo(a, tipo)
            elif tipo == "config":
                self._analizar_config(a)

    # ------------------------------------------------------------ código
    def _analizar_codigo(self, a, tipo):
        ruta = a.ruta_efectiva
        cfg = LENGUAJES[EXT_A_LENGUAJE[os.path.splitext(ruta)[1].lower()]]
        prefijos = cfg["comentario"]

        agregadas, eliminadas = [], []
        hunks_con_cambios = []
        for h in a.hunks:
            ag = [l for l in h["agregadas"] if es_significativa(l, prefijos)]
            el = [l for l in h["eliminadas"] if es_significativa(l, prefijos)]
            agregadas += ag
            eliminadas += el
            if ag or el:
                hunks_con_cambios.append(h)

        # Símbolos públicos
        quitados, puestos, tipos = defaultdict(set), defaultdict(set), {}
        for lista, destino in ((eliminadas, quitados), (agregadas, puestos)):
            for linea in lista:
                d = extraer_definicion(linea, cfg)
                if d and cfg["publico"](d[0], linea):
                    destino[d[0]].add(normalizar_firma(linea))
                    tipos[d[0]] = d[1]

        nombres_reportados = set()
        for nombre in sorted(set(quitados) | set(puestos)):
            etiqueta = tipos[nombre]
            if nombre in puestos and nombre not in quitados:
                self.agregar(f"{etiqueta} pública nueva", nombre, ruta, self._menciones(nombre),
                             ahora="; ".join(sorted(puestos[nombre])))
                nombres_reportados.add(nombre)
            elif nombre in quitados and nombre not in puestos:
                self.agregar(f"{etiqueta} pública eliminada o renombrada", nombre, ruta, self._menciones(nombre),
                             antes="; ".join(sorted(quitados[nombre])))
                nombres_reportados.add(nombre)
            elif quitados[nombre] != puestos[nombre]:
                self.agregar(f"firma modificada ({etiqueta})", nombre, ruta, self._menciones(nombre),
                             antes="; ".join(sorted(quitados[nombre] - puestos[nombre])),
                             ahora="; ".join(sorted(puestos[nombre] - quitados[nombre])))
                nombres_reportados.add(nombre)

        # Opciones de línea de comandos y variables de entorno
        self._comparar_conjuntos(
            extraer_opciones(eliminadas), extraer_opciones(agregadas), ruta,
            "opción de línea de comandos", como_palabra=True,
        )
        self._comparar_conjuntos(
            extraer_variables_entorno(eliminadas), extraer_variables_entorno(agregadas), ruta,
            "variable de entorno", como_palabra=True,
        )

        # Funciones cuyo cuerpo cambió (informativo)
        if a.estado == "modificado" or a.estado == "renombrado":
            vistas = set()
            for h in hunks_con_cambios:
                d = extraer_definicion(h["contexto"], cfg)
                if d and d[0] not in nombres_reportados and d[0] not in vistas:
                    vistas.add(d[0])
                    self.funciones_modificadas.append({"archivo": ruta, "funcion": d[0]})

    def _comparar_conjuntos(self, antes, despues, archivo, etiqueta, como_palabra):
        for nombre in sorted(despues - antes):
            self.agregar(f"{etiqueta} nueva", nombre, archivo, self._menciones(nombre, como_palabra))
        for nombre in sorted(antes - despues):
            self.agregar(f"{etiqueta} eliminada", nombre, archivo, self._menciones(nombre, como_palabra))

    # ------------------------------------------------------------ configuración
    def _analizar_config(self, a):
        ruta = a.ruta_efectiva
        nombre_base = os.path.basename(ruta)
        agregadas, eliminadas = [], []
        for h in a.hunks:
            agregadas += [l for l in h["agregadas"] if es_significativa(l, PREFIJOS_COMENTARIO_CONFIG)]
            eliminadas += [l for l in h["eliminadas"] if es_significativa(l, PREFIJOS_COMENTARIO_CONFIG)]
        es_env = bool(RE_ENV_ARCHIVO.match(nombre_base))

        if es_env:
            self._comparar_conjuntos(
                extraer_variables_entorno(eliminadas, True), extraer_variables_entorno(agregadas, True),
                ruta, "variable de entorno", como_palabra=True,
            )
            return

        if not agregadas and not eliminadas and a.estado == "modificado":
            return
        estado = {"nuevo": "nuevo", "eliminado": "eliminado"}.get(a.estado, "modificado")
        muestra = [f"+ {l.strip()}" for l in agregadas[:4]] + [f"- {l.strip()}" for l in eliminadas[:4]]
        self.agregar(f"archivo de configuración {estado}", nombre_base, ruta,
                     self._menciones(nombre_base, como_palabra=False),
                     cambios=muestra, lineas_agregadas=len(agregadas), lineas_eliminadas=len(eliminadas))


# ---------------------------------------------------------------- salida

NOMBRES_TIPO = {
    "codigo": "de código", "config": "de configuración", "test": "de tests",
    "documentacion": "de documentación", "bloqueo": "de dependencias bloqueadas", "otro": "otros",
}


def imprimir_texto(fuente, analizador, n_archivos):
    cambios = sorted(analizador.cambios, key=lambda c: (c["prioridad"] != "alta", c["archivo"], c["nombre"]))
    print("Detector de cambios documentables")
    print(f"Fuente: {fuente}")
    partes = [f"{n} {NOMBRES_TIPO[t]}" for t, n in analizador.resumen_archivos.items() if n]
    print(f"Archivos con cambios: {n_archivos}" + (f" ({', '.join(partes)})" if partes else ""))
    print()
    if cambios:
        print(f"RESULTADO: HAY {len(cambios)} CAMBIO(S) QUE PUEDEN AFECTAR A LA DOCUMENTACIÓN")
        print()
        for c in cambios:
            etiqueta = "ALTA " if c["prioridad"] == "alta" else "MEDIA"
            print(f"[{etiqueta}] {c['categoria'].capitalize()}: {c['nombre']}  ({c['archivo']})")
            if c.get("antes"):
                print(f"    Antes: {c['antes']}")
            if c.get("ahora"):
                print(f"    Ahora: {c['ahora']}")
            for linea in c.get("cambios", []):
                print(f"    {linea}")
            if c["documentacion"]:
                print(f"    La documentación lo menciona en: {', '.join(c['documentacion'])}")
            else:
                print("    No aparece en la documentación.")
        print()
    else:
        print("RESULTADO: SIN CAMBIOS QUE AFECTEN A LA DOCUMENTACIÓN")
        print()

    mods = analizador.funciones_modificadas
    if mods:
        print("Funciones cuyo cuerpo cambió (informativo: revisa si su comentario sigue siendo correcto):")
        for m in mods[:30]:
            print(f"  {m['archivo']}: {m['funcion']}")
        if len(mods) > 30:
            print(f"  ... y {len(mods) - 30} más")
        print()
    print("Nota: es un análisis aproximado basado en reglas simples; trata los resultados como pistas.")


def main(argv=None):
    parser = argparse.ArgumentParser(description="Detecta si unos cambios en el código afectan a la documentación.")
    parser.add_argument("proyecto", nargs="?", default=".", help="ruta del proyecto (por defecto, el directorio actual)")
    grupo = parser.add_mutually_exclusive_group()
    grupo.add_argument("--staged", action="store_true", help="solo los cambios preparados (git add)")
    grupo.add_argument("--base", metavar="REV", help="compara contra una revisión, por ejemplo main...HEAD")
    grupo.add_argument("--diff", metavar="ARCHIVO", help="lee un diff desde un archivo ('-' para la entrada estándar)")
    parser.add_argument("--docs", default="docs", help="directorio de documentación (por defecto: docs)")
    parser.add_argument("--json", action="store_true", help="salida en JSON")
    parser.add_argument("--fallar-si-relevante", action="store_true", help="código de salida 1 si hay cambios relevantes")
    args = parser.parse_args(argv)

    raiz = Path(args.proyecto).resolve()
    if not raiz.is_dir():
        print(f"No existe el directorio del proyecto: {raiz}", file=sys.stderr)
        return 2

    try:
        texto, fuente = obtener_diff(args, raiz)
    except (ErrorGit, OSError) as e:
        print(f"Error: {e}", file=sys.stderr)
        return 2

    archivos = parsear_diff(texto)
    indice = IndiceDocs(raiz, raiz / args.docs)
    analizador = Analizador(indice, args.docs)
    analizador.analizar(archivos)

    relevante = bool(analizador.cambios)
    if args.json:
        print(json.dumps(
            {
                "fuente": fuente,
                "archivos_con_cambios": len(archivos),
                "relevante": relevante,
                "cambios": analizador.cambios,
                "funciones_con_cuerpo_modificado": analizador.funciones_modificadas,
            },
            ensure_ascii=False, indent=2,
        ))
    else:
        imprimir_texto(fuente, analizador, len(archivos))

    if relevante and args.fallar_si_relevante:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
