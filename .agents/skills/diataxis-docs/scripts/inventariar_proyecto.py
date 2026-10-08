#!/usr/bin/env python3
"""Inventario rápido de un proyecto: hechos verificables para la Fase 1 y 2 de la skill.

Reúne en un solo informe lo que el agente necesita saber antes de planificar:

  - Lenguajes presentes (archivos y líneas), gestores de paquetes y manifiestos.
  - Posibles puntos de entrada, comandos declarados (scripts de package.json, pyproject).
  - Tests, integración continua, contenedores.
  - Pistas sobre el tipo de proyecto (web/API, CLI, datos/IA, infraestructura, paquete).
  - Documentación existente: ruta, tamaño, tipo declarado, secciones, marcas [VERIFICAR].
  - Observaciones evidentes (no hay README, no hay docs/, páginas sin metadatos).

Importante: solo informa de hechos y pistas. NO clasifica la documentación en tipos ni
decide el tipo de proyecto: eso lo juzga el agente leyendo el contenido.

Uso:
    python inventariar_proyecto.py [ruta_del_proyecto] [--docs docs] [--json]

Solo usa la biblioteca estándar de Python (3.8 o superior).
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

try:  # Python 3.11+
    import tomllib
except ImportError:  # pragma: no cover
    tomllib = None

DIRS_IGNORADOS = {
    ".git", "node_modules", "vendor", "dist", "build", "target", "__pycache__", ".venv", "venv",
    "env", ".tox", ".idea", ".vscode", ".mypy_cache", ".pytest_cache", ".next", "coverage",
    "site-packages", ".gradle", ".terraform", "bin", "obj",
}
MAX_ARCHIVOS = 20000
MAX_BYTES_LECTURA = 1_000_000

EXT_LENGUAJE = {
    ".py": "Python", ".js": "JavaScript", ".jsx": "JavaScript", ".mjs": "JavaScript", ".cjs": "JavaScript",
    ".ts": "TypeScript", ".tsx": "TypeScript", ".go": "Go", ".rs": "Rust", ".java": "Java",
    ".kt": "Kotlin", ".cs": "C#", ".rb": "Ruby", ".php": "PHP", ".c": "C", ".h": "C/C++",
    ".cpp": "C++", ".hpp": "C++", ".swift": "Swift", ".sh": "Shell", ".sql": "SQL",
    ".html": "HTML", ".css": "CSS", ".scss": "CSS", ".ipynb": "Notebook de Jupyter",
    ".tf": "Terraform", ".lua": "Lua", ".dart": "Dart", ".r": "R", ".scala": "Scala",
}
LENGUAJES_SIN_LINEAS = {"Notebook de Jupyter"}

MANIFIESTOS = {
    "package.json": "Node.js (npm/yarn/pnpm)",
    "pyproject.toml": "Python (pip/poetry/uv...)",
    "setup.py": "Python (setuptools)",
    "setup.cfg": "Python (setuptools)",
    "requirements.txt": "Python (pip)",
    "pipfile": "Python (pipenv)",
    "cargo.toml": "Rust (cargo)",
    "go.mod": "Go (módulos)",
    "pom.xml": "Java (maven)",
    "build.gradle": "Java/Kotlin (gradle)",
    "build.gradle.kts": "Java/Kotlin (gradle)",
    "gemfile": "Ruby (bundler)",
    "composer.json": "PHP (composer)",
    "makefile": "Make",
    "justfile": "Just",
    "dockerfile": "Contenedor Docker",
    "docker-compose.yml": "Docker Compose",
    "docker-compose.yaml": "Docker Compose",
    "compose.yml": "Docker Compose",
    "compose.yaml": "Docker Compose",
    "tox.ini": "Tox",
    ".env.example": "Variables de entorno de ejemplo",
    "chart.yaml": "Helm",
}
PUNTOS_ENTRADA = {
    "main.py", "app.py", "__main__.py", "manage.py", "cli.py", "wsgi.py", "asgi.py", "server.py",
    "index.js", "index.ts", "server.js", "server.ts", "app.js", "app.ts", "main.js", "main.ts",
    "main.go", "main.rs", "lib.rs", "program.cs", "main.java", "application.java", "main.rb",
}
DIRS_TEST = {"test", "tests", "__tests__", "spec", "specs", "e2e"}
RE_ARCHIVO_TEST = re.compile(
    r"(^test_.*\.py$|.*_test\.(py|go|rb)$|.*\.(test|spec)\.[a-z]+$|.*(Test|Tests)\.(java|cs)$|.*_spec\.rb$)"
)

PISTAS_DEPENDENCIAS = {
    "aplicación web o API": {
        "flask", "django", "fastapi", "starlette", "aiohttp", "express", "koa", "fastify", "next",
        "nuxt", "react", "vue", "svelte", "@nestjs/core", "@angular/core", "rails", "sinatra",
        "laravel/framework", "github.com/gin-gonic/gin", "github.com/gofiber/fiber", "actix-web",
        "axum", "rocket", "spring-boot-starter-web",
    },
    "herramienta de línea de comandos": {
        "click", "typer", "argparse", "commander", "yargs", "oclif", "github.com/spf13/cobra",
        "clap", "docopt", "thor",
    },
    "datos / IA": {
        "pandas", "numpy", "scipy", "scikit-learn", "sklearn", "torch", "tensorflow", "keras",
        "transformers", "jupyter", "matplotlib", "langchain", "anthropic", "openai", "polars",
        "xgboost", "lightgbm",
    },
}
FRAMEWORKS_TEST = {
    "pytest", "unittest", "nose", "jest", "vitest", "mocha", "jasmine", "playwright", "cypress",
    "rspec", "minitest", "phpunit", "junit", "testng",
}

RE_ENCABEZADO = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")


# ---------------------------------------------------------------- utilidades

def leer(ruta):
    try:
        if ruta.stat().st_size > MAX_BYTES_LECTURA:
            return ""
        return ruta.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def git(raiz, *args):
    try:
        r = subprocess.run(
            ["git"] + list(args), cwd=str(raiz), capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=15,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None
    return r.stdout.strip() if r.returncode == 0 else None


def recorrer(raiz):
    """Devuelve la lista de rutas (relativas, con /) de los archivos del proyecto."""
    archivos = []
    for dirpath, dirnames, filenames in os.walk(raiz):
        dirnames[:] = sorted(d for d in dirnames if d not in DIRS_IGNORADOS)
        for nombre in sorted(filenames):
            completa = Path(dirpath) / nombre
            archivos.append(completa.relative_to(raiz).as_posix())
            if len(archivos) >= MAX_ARCHIVOS:
                return archivos, True
    return archivos, False


def contar_lineas(texto):
    return texto.count("\n") + (1 if texto and not texto.endswith("\n") else 0)


def frontmatter(texto):
    lineas = texto.split("\n")
    if not lineas or lineas[0].strip() != "---":
        return {}
    datos = {}
    for linea in lineas[1:]:
        if linea.strip() == "---":
            return datos
        limpio = re.sub(r"\s+#.*$", "", linea).strip()
        if ":" in limpio:
            k, v = limpio.split(":", 1)
            datos[k.strip()] = v.strip().strip("\"'")
    return {}


# ---------------------------------------------------------------- lectura de manifiestos

def info_package_json(ruta):
    try:
        datos = json.loads(leer(ruta))
    except ValueError:
        return {}
    deps = {}
    for clave in ("dependencies", "devDependencies", "peerDependencies"):
        deps.update(datos.get(clave) or {})
    return {
        "nombre": datos.get("name"),
        "main": datos.get("main"),
        "bin": datos.get("bin"),
        "scripts": datos.get("scripts") or {},
        "dependencias": {d.lower() for d in deps},
    }


def info_pyproject(ruta):
    texto = leer(ruta)
    info = {"nombre": None, "scripts": {}, "dependencias": set()}
    if tomllib:
        try:
            datos = tomllib.loads(texto)
        except Exception:
            datos = None
        if datos is not None:
            proyecto = datos.get("project", {})
            poetry = datos.get("tool", {}).get("poetry", {})
            info["nombre"] = proyecto.get("name") or poetry.get("name")
            info["scripts"] = dict(proyecto.get("scripts") or poetry.get("scripts") or {})
            for dep in proyecto.get("dependencies") or []:
                info["dependencias"].add(re.split(r"[\s<>=!~;\[]", dep, maxsplit=1)[0].lower())
            for dep in (poetry.get("dependencies") or {}):
                info["dependencias"].add(dep.lower())
            return info
    # Alternativa sin tomllib: lectura aproximada por secciones
    seccion = ""
    for linea in texto.split("\n"):
        s = linea.strip()
        m = re.match(r"^\[(.+)\]$", s)
        if m:
            seccion = m.group(1)
            continue
        kv = re.match(r"^([\w.\-]+)\s*=\s*(.+)$", s)
        if kv and seccion in ("project.scripts", "tool.poetry.scripts"):
            info["scripts"][kv.group(1)] = kv.group(2).strip("\"'")
        elif kv and seccion in ("project", "tool.poetry") and kv.group(1) == "name":
            info["nombre"] = kv.group(2).strip("\"'")
        elif kv and seccion == "tool.poetry.dependencies":
            info["dependencias"].add(kv.group(1).lower())
    return info


def dependencias_requirements(ruta):
    deps = set()
    for linea in leer(ruta).split("\n"):
        linea = linea.strip()
        if linea and not linea.startswith(("#", "-")):
            deps.add(re.split(r"[\s<>=!~;\[]", linea, maxsplit=1)[0].lower())
    return deps


def dependencias_cargo(ruta):
    deps, dentro = set(), False
    for linea in leer(ruta).split("\n"):
        s = linea.strip()
        if s.startswith("["):
            dentro = s in ("[dependencies]", "[dev-dependencies]")
            continue
        if dentro and "=" in s and not s.startswith("#"):
            deps.add(s.split("=", 1)[0].strip().lower())
    return deps


def dependencias_go(ruta):
    deps = set()
    for m in re.finditer(r"^\s*(?:require\s+)?([\w.\-]+\.[\w.\-]+/[\w./\-]+)\s+v", leer(ruta), re.MULTILINE):
        deps.add(m.group(1).lower())
    return deps


# ---------------------------------------------------------------- documentación existente

def resumir_documento(raiz, rel):
    texto = leer(raiz / rel)
    meta = frontmatter(texto)
    h2, en_codigo, bloques = [], False, 0
    for linea in texto.split("\n"):
        if re.match(r"^\s{0,3}(`{3,}|~{3,})", linea):
            if not en_codigo:
                bloques += 1
            en_codigo = not en_codigo
            continue
        if en_codigo:
            continue
        m = RE_ENCABEZADO.match(linea)
        if m and len(m.group(1)) == 2 and len(h2) < 10:
            h2.append(m.group(2))
    return {
        "ruta": rel,
        "lineas": contar_lineas(texto),
        "tipo_declarado": meta.get("tipo"),
        "verificado": meta.get("verificado"),
        "secciones": h2,
        "bloques_de_codigo": bloques,
        "marcas_verificar": len(re.findall(r"\[VERIFICAR", texto)),
    }


# ---------------------------------------------------------------- inventario

def inventariar(raiz, nombre_docs):
    archivos, truncado = recorrer(raiz)
    inv = {"proyecto": raiz.name, "ruta": str(raiz), "truncado": truncado, "total_archivos": len(archivos)}

    # Git
    if git(raiz, "rev-parse", "--is-inside-work-tree") == "true":
        ultimo = git(raiz, "log", "-1", "--format=%cs|%s")
        inv["git"] = {
            "rama": git(raiz, "rev-parse", "--abbrev-ref", "HEAD"),
            "commits": git(raiz, "rev-list", "--count", "HEAD"),
            "ultimo_commit": ultimo,
        }
    else:
        inv["git"] = None

    # Lenguajes
    n_archivos, n_lineas = Counter(), Counter()
    for rel in archivos:
        lenguaje = EXT_LENGUAJE.get(os.path.splitext(rel)[1].lower())
        if lenguaje:
            n_archivos[lenguaje] += 1
            if lenguaje not in LENGUAJES_SIN_LINEAS:
                n_lineas[lenguaje] += contar_lineas(leer(raiz / rel))
    inv["lenguajes"] = [
        {"lenguaje": l, "archivos": n, "lineas": n_lineas.get(l)} for l, n in n_archivos.most_common(8)
    ]

    # Manifiestos
    manifiestos, ci = {}, []
    for rel in archivos:
        base = os.path.basename(rel).lower()
        profundidad = rel.count("/")
        if base in MANIFIESTOS and profundidad <= 2:
            manifiestos[rel] = MANIFIESTOS[base]
        if base.endswith(".csproj") and profundidad <= 2:
            manifiestos[rel] = "C# (.NET)"
        if rel.startswith(".github/workflows/") or base in (".gitlab-ci.yml", "jenkinsfile", ".travis.yml", "azure-pipelines.yml"):
            ci.append(rel)
    inv["manifiestos"] = manifiestos
    inv["integracion_continua"] = ci

    # Dependencias, scripts y pistas
    dependencias, scripts, bin_declarado = set(), {}, False
    for rel in manifiestos:
        base = os.path.basename(rel).lower()
        ruta = raiz / rel
        if base == "package.json":
            info = info_package_json(ruta)
            dependencias |= info.get("dependencias", set())
            for nombre, cmd in (info.get("scripts") or {}).items():
                scripts[f"npm:{nombre}"] = cmd
            if info.get("bin"):
                bin_declarado = True
                scripts["npm:bin"] = json.dumps(info["bin"], ensure_ascii=False)
        elif base == "pyproject.toml":
            info = info_pyproject(ruta)
            dependencias |= info["dependencias"]
            for nombre, cmd in info["scripts"].items():
                scripts[f"python:{nombre}"] = str(cmd)
                bin_declarado = True
        elif base.startswith("requirements") and base.endswith(".txt"):
            dependencias |= dependencias_requirements(ruta)
        elif base == "cargo.toml":
            dependencias |= dependencias_cargo(ruta)
        elif base == "go.mod":
            dependencias |= dependencias_go(ruta)
    inv["comandos_declarados"] = dict(list(scripts.items())[:15])

    pistas = []
    for etiqueta, claves in PISTAS_DEPENDENCIAS.items():
        encontradas = sorted(dependencias & claves)
        if encontradas:
            pistas.append(f"{etiqueta}: dependencias {', '.join(encontradas[:6])}")
    if bin_declarado:
        pistas.append("herramienta de línea de comandos: declara comandos ejecutables (bin o scripts del paquete)")
    if n_archivos.get("Notebook de Jupyter"):
        pistas.append(f"datos / IA: hay {n_archivos['Notebook de Jupyter']} notebook(s) de Jupyter")
    if n_archivos.get("Terraform") or any(os.path.basename(r).lower() == "chart.yaml" for r in archivos):
        pistas.append("infraestructura: hay archivos de Terraform o charts de Helm")
    if any(os.path.basename(r).lower() in ("dockerfile", "docker-compose.yml", "compose.yml") for r in manifiestos):
        pistas.append("se despliega o ejecuta en contenedores (Docker)")
    if any(os.path.basename(r).lower() in ("setup.py", "setup.cfg") for r in manifiestos) or "pyproject.toml" in [os.path.basename(r).lower() for r in manifiestos]:
        pistas.append("paquete Python instalable (tiene pyproject.toml o setup.py)")
    inv["pistas_tipo_proyecto"] = pistas

    # Puntos de entrada
    entradas = [
        rel for rel in archivos
        if os.path.basename(rel).lower() in PUNTOS_ENTRADA and rel.count("/") <= 3
    ]
    for d in ("cmd", "bin"):
        if (raiz / d).is_dir():
            entradas.append(d + "/")
    inv["posibles_puntos_de_entrada"] = entradas[:15]

    # Tests
    archivos_test = [
        rel for rel in archivos
        if any(p.lower() in DIRS_TEST for p in rel.split("/")[:-1]) or RE_ARCHIVO_TEST.match(os.path.basename(rel))
    ]
    inv["tests"] = {
        "archivos": len(archivos_test),
        "frameworks_probables": sorted(dependencias & FRAMEWORKS_TEST),
    }

    # Documentación existente
    docs_rel = [
        rel for rel in archivos
        if rel.lower().endswith((".md", ".rst", ".adoc")) or os.path.basename(rel).lower() == "llms.txt"
    ]
    entrada = [r for r in docs_rel if "/" not in r]
    en_docs = [r for r in docs_rel if r.split("/")[0].lower() == nombre_docs.lower() and "/" in r]
    otros = [r for r in docs_rel if r not in entrada and r not in en_docs]
    inv["documentacion"] = {
        "directorio_docs_existe": (raiz / nombre_docs).is_dir(),
        "raiz": [resumir_documento(raiz, r) for r in entrada if r.lower().endswith((".md", ".rst", ".adoc"))],
        "docs": [resumir_documento(raiz, r) for r in en_docs[:60]],
        "otros_en_el_proyecto": otros[:30],
        "llms_txt": any(os.path.basename(r).lower() == "llms.txt" for r in docs_rel),
    }

    # Observaciones
    nombres_raiz = {os.path.basename(r).lower() for r in entrada}
    obs = []
    if not any(n.startswith("readme") for n in nombres_raiz):
        obs.append("No hay README en la raíz.")
    if not inv["documentacion"]["directorio_docs_existe"]:
        obs.append(f"No existe el directorio {nombre_docs}/.")
    if "agents.md" not in nombres_raiz:
        obs.append("No hay AGENTS.md en la raíz.")
    sin_meta = [d["ruta"] for d in inv["documentacion"]["docs"] if not d["tipo_declarado"] and os.path.basename(d["ruta"]).lower() not in ("readme.md", "index.md")]
    if sin_meta:
        obs.append(f"{len(sin_meta)} página(s) de {nombre_docs}/ sin metadatos de tipo (no siguen el formato de la skill).")
    if archivos_test == []:
        obs.append("No se detectaron tests.")
    if truncado:
        obs.append(f"El proyecto supera {MAX_ARCHIVOS} archivos: el inventario es parcial.")
    inv["observaciones"] = obs
    return inv


# ---------------------------------------------------------------- salida

def imprimir_texto(inv):
    print(f"Inventario del proyecto: {inv['proyecto']}")
    print(f"Ruta: {inv['ruta']}  ({inv['total_archivos']} archivos revisados)")
    g = inv["git"]
    if g:
        extra = ""
        if g["ultimo_commit"]:
            fecha, _, asunto = g["ultimo_commit"].partition("|")
            extra = f"; último commit {fecha}: {asunto}"
        print(f"Git: sí, rama {g['rama']}, {g['commits']} commits{extra}")
    else:
        print("Git: no es un repositorio git")

    print("\nLenguajes:")
    if inv["lenguajes"]:
        for l in inv["lenguajes"]:
            lineas = f", {l['lineas']} líneas" if l["lineas"] is not None else ""
            print(f"  - {l['lenguaje']}: {l['archivos']} archivo(s){lineas}")
    else:
        print("  (no se reconocieron lenguajes de programación)")

    print("\nManifiestos y herramientas:")
    for ruta, desc in inv["manifiestos"].items():
        print(f"  - {ruta}: {desc}")
    if not inv["manifiestos"]:
        print("  (ninguno)")
    if inv["integracion_continua"]:
        print("  Integración continua: " + ", ".join(inv["integracion_continua"][:5]))

    print("\nPosibles puntos de entrada:")
    for e in inv["posibles_puntos_de_entrada"] or ["(no se detectaron por nombre)"]:
        print(f"  - {e}")

    if inv["comandos_declarados"]:
        print("\nComandos declarados en los manifiestos:")
        for nombre, cmd in inv["comandos_declarados"].items():
            print(f"  - {nombre}: {cmd}")

    t = inv["tests"]
    marcos = f" (frameworks probables: {', '.join(t['frameworks_probables'])})" if t["frameworks_probables"] else ""
    print(f"\nTests: {t['archivos']} archivo(s){marcos}")

    print("\nPistas sobre el tipo de proyecto (no son un veredicto):")
    for p in inv["pistas_tipo_proyecto"] or ["(sin pistas claras; léelo y decide, o pregunta al usuario)"]:
        print(f"  - {p}")

    d = inv["documentacion"]
    print("\nDocumentación existente:")
    todos = d["raiz"] + d["docs"]
    if todos:
        for doc in todos:
            tipo = doc["tipo_declarado"] or "sin tipo declarado"
            marcas = f", {doc['marcas_verificar']} [VERIFICAR]" if doc["marcas_verificar"] else ""
            print(f"  - {doc['ruta']}: {doc['lineas']} líneas, {tipo}, {doc['bloques_de_codigo']} bloque(s) de código{marcas}")
            if doc["secciones"]:
                print(f"      secciones: {' | '.join(doc['secciones'][:6])}")
    else:
        print("  (ninguna)")
    if d["otros_en_el_proyecto"]:
        print("  Otros documentos en el proyecto: " + ", ".join(d["otros_en_el_proyecto"][:8]))
    if d["llms_txt"]:
        print("  Existe llms.txt")

    if inv["observaciones"]:
        print("\nObservaciones:")
        for o in inv["observaciones"]:
            print(f"  - {o}")


def main(argv=None):
    parser = argparse.ArgumentParser(description="Inventario de un proyecto para planificar su documentación.")
    parser.add_argument("proyecto", nargs="?", default=".", help="ruta del proyecto (por defecto, el directorio actual)")
    parser.add_argument("--docs", default="docs", help="directorio de documentación (por defecto: docs)")
    parser.add_argument("--json", action="store_true", help="salida en JSON")
    args = parser.parse_args(argv)

    raiz = Path(args.proyecto).resolve()
    if not raiz.is_dir():
        print(f"No existe el directorio del proyecto: {raiz}", file=sys.stderr)
        return 2
    inv = inventariar(raiz, args.docs)
    if args.json:
        print(json.dumps(inv, ensure_ascii=False, indent=2, default=list))
    else:
        imprimir_texto(inv)
    return 0


if __name__ == "__main__":
    sys.exit(main())
