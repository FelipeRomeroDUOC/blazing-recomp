#!/usr/bin/env python3
"""
Extractor de Referencia Técnica desde el Código Fuente de Rust (blazing-recomp).

Escanea los archivos en `src/` e inspecciona comentarios de documentación (/// y //!)
junto a las firmas públicas de estructuras, enumeraciones, constantes y funciones.
Genera o valida `docs/referencia/api-modulos-rust.md` asegurando que la documentación
de referencia técnica se mantenga 100% sincronizada con el código fuente.

Uso:
    python scripts/extraer_referencias.py          # Genera/actualiza docs/referencia/api-modulos-rust.md
    python scripts/extraer_referencias.py --check  # Valida que el documento esté al día (retorna 1 si difiere)
"""

import sys
import re
from pathlib import Path


def limpiar_doc(lineas):
    """Limpia los prefijos /// de los docstrings."""
    resultado = []
    for linea_doc in lineas:
        linea_doc = linea_doc.strip()
        if linea_doc.startswith("///"):
            contenido = linea_doc[3:]
            if contenido.startswith(" "):
                contenido = contenido[1:]
            resultado.append(contenido)
        elif linea_doc.startswith("//!"):
            contenido = linea_doc[3:]
            if contenido.startswith(" "):
                contenido = contenido[1:]
            resultado.append(contenido)
    return "\n".join(resultado).strip()


def parsear_archivo_rust(path_archivo):
    """Extrae declaraciones públicas y sus comentarios de documentación."""
    with open(path_archivo, "r", encoding="utf-8") as f:
        lineas = f.readlines()

    modulo_doc = []
    elementos = []  # list of dicts: {tipo, nombre, firma, doc}

    doc_acumulado = []
    i = 0
    total = len(lineas)

    while i < total:
        linea = lineas[i].strip()

        # Documentación de módulo //!
        if linea.startswith("//!"):
            modulo_doc.append(linea)
            i += 1
            continue

        # Documentación de elemento ///
        if linea.startswith("///"):
            doc_acumulado.append(linea)
            i += 1
            continue

        # Si hay atributos #[...], los saltamos pero retenemos doc
        if linea.startswith("#["):
            i += 1
            continue

        # Constante pública
        m_const = re.match(r"^pub(?:\([^)]+\))?\s+const\s+([A-Za-z0-9_]+)\s*:\s*([^=]+)=", linea)
        if m_const:
            nombre = m_const.group(1)
            tipo_const = m_const.group(2).strip()
            elementos.append({
                "tipo": "Constante",
                "nombre": nombre,
                "firma": f"pub const {nombre}: {tipo_const}",
                "doc": limpiar_doc(doc_acumulado),
            })
            doc_acumulado = []
            i += 1
            continue

        # Estructura pública
        m_struct = re.match(r"^pub(?:\([^)]+\))?\s+struct\s+([A-Za-z0-9_]+)", linea)
        if m_struct:
            nombre = m_struct.group(1)
            # Acumular firma completa hasta { o ;
            firma = linea
            while not ("{" in firma or ";" in firma) and i + 1 < total:
                i += 1
                firma += " " + lineas[i].strip()
            elementos.append({
                "tipo": "Struct",
                "nombre": nombre,
                "firma": firma.split("{")[0].strip(),
                "doc": limpiar_doc(doc_acumulado),
            })
            doc_acumulado = []
            i += 1
            continue

        # Enumeración pública
        m_enum = re.match(r"^pub(?:\([^)]+\))?\s+enum\s+([A-Za-z0-9_]+)", linea)
        if m_enum:
            nombre = m_enum.group(1)
            firma = linea.split("{")[0].strip()
            elementos.append({
                "tipo": "Enum",
                "nombre": nombre,
                "firma": firma,
                "doc": limpiar_doc(doc_acumulado),
            })
            doc_acumulado = []
            i += 1
            continue

        # Función pública
        m_fn = re.match(r"^pub(?:\([^)]+\))?\s+(?:unsafe\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)", linea)
        if m_fn:
            nombre = m_fn.group(1)
            firma_partes = [linea]
            # Acumular cabecera hasta { o ;
            while not ("{" in "".join(firma_partes) or ";" in "".join(firma_partes)) and i + 1 < total:
                i += 1
                firma_partes.append(lineas[i].strip())
            firma_completa = " ".join(firma_partes)
            firma_limpia = firma_completa.split("{")[0].strip()
            elementos.append({
                "tipo": "Función",
                "nombre": nombre,
                "firma": firma_limpia,
                "doc": limpiar_doc(doc_acumulado),
            })
            doc_acumulado = []
            i += 1
            continue

        # Si no fue elemento documentado, limpiar doc_acumulado si encontramos código
        if linea and not linea.startswith("//"):
            doc_acumulado = []

        i += 1

    return {
        "archivo": path_archivo.name,
        "doc_modulo": limpiar_doc(modulo_doc),
        "elementos": elementos,
    }


def generar_markdown_referencia(modulos):
    """Genera el contenido Markdown conforme al estándar Diátaxis (tipo: referencia)."""
    md = []
    md.append("---")
    md.append("tipo: referencia")
    md.append("verificado: si")
    md.append("---")
    md.append("")
    md.append("# Referencia Técnica: Módulos y APIs de Rust")
    md.append("")
    md.append("## Descripción general")
    md.append("")
    md.append("Este documento describe con exactitud y neutralidad técnica las estructuras, enumeraciones, funciones públicas y constantes del crate nativo `blazing-recomp`. Es generado y auditado automáticamente desde las anotaciones de documentación (`///` y `//!`) del código fuente en `src/` mediante `scripts/extraer_referencias.py`, garantizando sincronización continua con el código.")
    md.append("")
    md.append("---")
    md.append("")
    md.append("## Índice de Módulos")
    md.append("")
    md.append("| Módulo | Elementos Públicos | Descripción |")
    md.append("| :--- | :--- | :--- |")
    for mod in modulos:
        num_elem = len(mod["elementos"])
        primera_linea = mod["doc_modulo"].split("\n")[0] if mod["doc_modulo"] else "Módulo interno del port."
        slug_anchor = f"módulo-{mod['archivo'].replace('.', '').lower()}"
        md.append(f"| [`{mod['archivo']}`](#{slug_anchor}) | {num_elem} | {primera_linea} |")

    md.append("")
    md.append("---")
    md.append("")

    for mod in modulos:
        md.append(f"## Módulo `{mod['archivo']}`")
        md.append("")
        if mod["doc_modulo"]:
            md.append(mod["doc_modulo"])
            md.append("")

        if not mod["elementos"]:
            md.append("*No expone símbolos públicos directos fuera de declaraciones de submódulos.*")
            md.append("")
            continue

        for elem in mod["elementos"]:
            md.append(f"### `{elem['nombre']}` ({elem['tipo']})")
            md.append("")
            if elem["doc"]:
                md.append(elem["doc"])
                md.append("")
            else:
                md.append("*Sin comentario de documentación en código fuente.*")
                md.append("")
            md.append("**Firma:**")
            md.append("```rust")
            md.append(elem["firma"])
            md.append("```")
            md.append("")

        md.append("---")
        md.append("")

    md.append("## Sincronización y Mantenimiento")
    md.append("")
    md.append("Para regenerar este documento ante cualquier modificación en las firmas o comentarios de `src/`:")
    md.append("```bash")
    md.append("python scripts/extraer_referencias.py")
    md.append("```")
    md.append("")
    md.append("Para validar que la documentación se encuentre estrictamente actualizada (usado en tests y CI):")
    md.append("```bash")
    md.append("python scripts/extraer_referencias.py --check")
    md.append("```")
    md.append("")

    return "\n".join(md)


def main():
    repo_root = Path(__file__).resolve().parent.parent
    src_dir = repo_root / "src"
    output_path = repo_root / "docs" / "referencia" / "api-modulos-rust.md"

    archivos_ordenados = [
        "lib.rs",
        "input.rs",
        "overlay.rs",
        "error_screen.rs",
        "platform.rs",
        "storage.rs",
        "runner.rs",
        "recomp.rs",
        "fps.rs",
        "telemetria.rs",
    ]

    modulos = []
    for nombre in archivos_ordenados:
        p = src_dir / nombre
        if p.exists():
            modulos.append(parsear_archivo_rust(p))

    contenido_generado = generar_markdown_referencia(modulos)

    check_mode = "--check" in sys.argv

    if check_mode:
        if not output_path.exists():
            print(f"ERROR: {output_path} no existe. Ejecuta 'python scripts/extraer_referencias.py' para crearlo.")
            sys.exit(1)
        with open(output_path, "r", encoding="utf-8") as f:
            existente = f.read()
        if existente.strip() != contenido_generado.strip():
            print(f"ERROR: {output_path} está desactualizado respecto al código fuente en src/.")
            print("Ejecuta 'python scripts/extraer_referencias.py' para sincronizar las referencias.")
            sys.exit(1)
        print("OK: Referencia técnica docs/referencia/api-modulos-rust.md sincronizada con el código.")
        sys.exit(0)
    else:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(contenido_generado)
        print(f"Éxito: Generada referencia técnica actualizada en {output_path}")


if __name__ == "__main__":
    main()
