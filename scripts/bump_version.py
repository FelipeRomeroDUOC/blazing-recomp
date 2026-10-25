#!/usr/bin/env python3
"""scripts/bump_version.py

Herramienta canónica de sincronización y avance de versión de Blazing Recomp.

Garantiza que todos los manifiestos y documentación mantengan versiones idénticas
y cumplan determinísticamente con la fórmula estándar de Android:
    versionCode = MAJOR * 10000 + MINOR * 100 + PATCH

Compuerta humana:
    Por defecto, cualquier incremento requiere confirmación explícita de la
    persona en la consola, a menos que se use la opción `--si` / `-y`.
"""

from __future__ import annotations

import argparse
import datetime
import os
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

try:
    import tomllib
except ImportError:
    # Respaldo para versiones antiguas de Python si aplicara
    import tomli as tomllib  # type: ignore


def obtener_raiz_repo() -> Path:
    """Resuelve la raíz del repositorio de forma relativa al script."""
    return Path(__file__).resolve().parent.parent


def calcular_version_code(version_str: str) -> int:
    """Calcula versionCode conforme a MAJOR * 10000 + MINOR * 100 + PATCH."""
    partes = [int(p) for p in version_str.strip().split(".")]
    if len(partes) != 3:
        raise ValueError(
            f"La versión '{version_str}' no cumple con el formato SemVer MAJOR.MINOR.PATCH"
        )
    return partes[0] * 10000 + partes[1] * 100 + partes[2]


def leer_estado_actual(raiz: Path) -> dict[str, str | int]:
    """Lee y extrae la versión declarada en cada manifiesto del proyecto."""
    estado: dict[str, str | int] = {}

    # 1. Cargo.toml
    cargo_path = raiz / "Cargo.toml"
    if cargo_path.exists():
        with open(cargo_path, "rb") as f:
            datos_cargo = tomllib.load(f)
            estado["cargo_version"] = datos_cargo.get("package", {}).get(
                "version", ""
            )

    # 2. pack.toml
    pack_path = raiz / "pack.toml"
    if pack_path.exists():
        with open(pack_path, "rb") as f:
            datos_pack = tomllib.load(f)
            estado["pack_version"] = datos_pack.get("package", {}).get(
                "version", ""
            )

    # 3. android/AndroidManifest.xml
    manifest_path = raiz / "android" / "AndroidManifest.xml"
    if manifest_path.exists():
        tree = ET.parse(manifest_path)
        root = tree.getroot()
        ns = "{http://schemas.android.com/apk/res/android}"
        estado["android_name"] = root.attrib.get(f"{ns}versionName", "")
        code_raw = root.attrib.get(f"{ns}versionCode", "0")
        try:
            estado["android_code"] = int(code_raw)
        except ValueError:
            estado["android_code"] = 0

    # 4. docs/BACKLOG.md
    backlog_path = raiz / "docs" / "BACKLOG.md"
    if backlog_path.exists():
        contenido = backlog_path.read_text(encoding="utf-8")
        match_ver = re.search(
            r"-\s+\*\*Versión:\*\*\s+`([^`]+)`\s+/\s+`versionCode`\s+(\d+)",
            contenido,
        )
        if match_ver:
            estado["backlog_version"] = match_ver.group(1)
            estado["backlog_code"] = int(match_ver.group(2))

    return estado


def validar_sincronizacion(
    raiz: Path,
) -> tuple[bool, list[str], str, int]:
    """Comprueba que todos los manifiestos coincidan entre sí y con la fórmula Android."""
    estado = leer_estado_actual(raiz)
    errores: list[str] = []

    cargo_ver = str(estado.get("cargo_version", ""))
    pack_ver = str(estado.get("pack_version", ""))
    android_name = str(estado.get("android_name", ""))
    android_code = int(estado.get("android_code", 0))

    if not cargo_ver:
        errores.append("Cargo.toml no declara package.version.")
    if not pack_ver:
        errores.append("pack.toml no declara package.version.")
    if not android_name:
        errores.append("AndroidManifest.xml no declara android:versionName.")

    if cargo_ver and pack_ver and cargo_ver != pack_ver:
        errores.append(
            f"Discrepancia: Cargo.toml ({cargo_ver}) != pack.toml ({pack_ver})"
        )
    if cargo_ver and android_name and cargo_ver != android_name:
        errores.append(
            f"Discrepancia: Cargo.toml ({cargo_ver}) != AndroidManifest.xml ({android_name})"
        )

    code_esperado = 0
    if cargo_ver:
        try:
            code_esperado = calcular_version_code(cargo_ver)
            if android_code != code_esperado:
                errores.append(
                    f"Discrepancia de fórmula: AndroidManifest.xml versionCode ({android_code}) "
                    f"!= esperado ({code_esperado}) para la versión {cargo_ver}."
                )
        except ValueError as e:
            errores.append(str(e))

    # Verificar docs/BACKLOG.md
    if "backlog_version" in estado:
        if estado["backlog_version"] != cargo_ver:
            errores.append(
                f"Discrepancia: docs/BACKLOG.md ({estado['backlog_version']}) != Cargo.toml ({cargo_ver})"
            )
        if estado.get("backlog_code") != code_esperado:
            errores.append(
                f"Discrepancia: docs/BACKLOG.md versionCode ({estado.get('backlog_code')}) != esperado ({code_esperado})"
            )

    es_valido = len(errores) == 0
    return es_valido, errores, cargo_ver, code_esperado


def calcular_siguiente_version(
    version_actual: str, tipo_bump: str, version_explicita: str | None = None
) -> tuple[str, int]:
    """Calcula la siguiente versión SemVer y su correspondiente versionCode."""
    if version_explicita:
        nueva_version = version_explicita.strip().lstrip("v")
        nuevo_code = calcular_version_code(nueva_version)
        return nueva_version, nuevo_code

    partes = [int(p) for p in version_actual.split(".")]
    major, minor, patch = partes[0], partes[1], partes[2]

    if tipo_bump == "patch":
        patch += 1
    elif tipo_bump == "minor":
        minor += 1
        patch = 0
    elif tipo_bump == "major":
        major += 1
        minor = 0
        patch = 0
    else:
        raise ValueError(f"Tipo de incremento no reconocido: {tipo_bump}")

    nueva_version = f"{major}.{minor}.{patch}"
    nuevo_code = calcular_version_code(nueva_version)
    return nueva_version, nuevo_code


def aplicar_cambios(
    raiz: Path,
    version_actual: str,
    nueva_version: str,
    nuevo_code: int,
    mensaje: str | None,
    dry_run: bool = False,
) -> list[str]:
    """Aplica o simula la actualización de versiones en todos los archivos pertinentes."""
    cambios: list[str] = []
    fecha_hoy = datetime.date.today().isoformat()

    # 1. Cargo.toml
    cargo_path = raiz / "Cargo.toml"
    if cargo_path.exists():
        contenido = cargo_path.read_text(encoding="utf-8")
        nuevo_contenido = re.sub(
            r'(version\s*=\s*")[^"]+(")',
            rf"\g<1>{nueva_version}\g<2>",
            contenido,
            count=1,
        )
        if nuevo_contenido != contenido:
            cambios.append(
                f"Cargo.toml: version = \"{version_actual}\" -> \"{nueva_version}\""
            )
            if not dry_run:
                cargo_path.write_text(nuevo_contenido, encoding="utf-8")

    # 2. pack.toml
    pack_path = raiz / "pack.toml"
    if pack_path.exists():
        contenido = pack_path.read_text(encoding="utf-8")
        nuevo_contenido = re.sub(
            r'(version\s*=\s*")[^"]+(")',
            rf"\g<1>{nueva_version}\g<2>",
            contenido,
            count=1,
        )
        if nuevo_contenido != contenido:
            cambios.append(
                f"pack.toml: version = \"{version_actual}\" -> \"{nueva_version}\""
            )
            if not dry_run:
                pack_path.write_text(nuevo_contenido, encoding="utf-8")

    # 3. android/AndroidManifest.xml
    manifest_path = raiz / "android" / "AndroidManifest.xml"
    if manifest_path.exists():
        contenido = manifest_path.read_text(encoding="utf-8")
        nuevo_contenido = re.sub(
            r'android:versionCode="[^"]+"',
            f'android:versionCode="{nuevo_code}"',
            contenido,
            count=1,
        )
        nuevo_contenido = re.sub(
            r'android:versionName="[^"]+"',
            f'android:versionName="{nueva_version}"',
            nuevo_contenido,
            count=1,
        )
        if nuevo_contenido != contenido:
            cambios.append(
                f"android/AndroidManifest.xml: versionCode -> {nuevo_code}, versionName -> \"{nueva_version}\""
            )
            if not dry_run:
                manifest_path.write_text(nuevo_contenido, encoding="utf-8")

    # 4. CHANGELOG.md
    changelog_path = raiz / "CHANGELOG.md"
    if changelog_path.exists():
        contenido = changelog_path.read_text(encoding="utf-8")
        cabecera_nueva = (
            f"## [{nueva_version}] - {fecha_hoy}\n\n"
            f'* `versionName`: "{nueva_version}"\n'
            f"* `versionCode`: {nuevo_code}\n"
            f"* Target principal: `arm64-v8a`, `armeabi-v7a`, `x86_64`\n\n"
        )
        if mensaje:
            cabecera_nueva += (
                f"### Novedades y Correcciones\n* {mensaje.strip()}\n\n"
            )

        patron_sin_publicar = r"(## \[Sin publicar\]\s*\n+---*\s*\n+)"
        if re.search(patron_sin_publicar, contenido):
            nuevo_contenido = re.sub(
                patron_sin_publicar,
                rf"\g<1>{cabecera_nueva}---\n\n",
                contenido,
                count=1,
            )
        else:
            # Si no hay [Sin publicar], insertar antes de la primera versión encontrada
            nuevo_contenido = re.sub(
                r"(## \[\d+\.\d+\.\d+\])",
                rf"{cabecera_nueva}---\n\n\g<1>",
                contenido,
                count=1,
            )

        if nuevo_contenido != contenido:
            cambios.append(
                f"CHANGELOG.md: Incorporada cabecera ## [{nueva_version}] - {fecha_hoy}"
            )
            if not dry_run:
                changelog_path.write_text(nuevo_contenido, encoding="utf-8")

    # 5. docs/BACKLOG.md
    backlog_path = raiz / "docs" / "BACKLOG.md"
    if backlog_path.exists():
        contenido = backlog_path.read_text(encoding="utf-8")
        nuevo_contenido = re.sub(
            r"(\*\*Última actualización:\*\*\s*)\d{4}-\d{2}-\d{2}",
            rf"\g<1>{fecha_hoy}",
            contenido,
            count=1,
        )
        nuevo_contenido = re.sub(
            r"(-\s+\*\*Versión:\*\*\s+`)[^`]+(`\s+/\s+`versionCode`\s+)\d+",
            rf"\g<1>{nueva_version}\g<2>{nuevo_code}",
            nuevo_contenido,
            count=1,
        )
        if nuevo_contenido != contenido:
            cambios.append(
                f"docs/BACKLOG.md: Versión actualizada a {nueva_version} ({nuevo_code}) y fecha a {fecha_hoy}"
            )
            if not dry_run:
                backlog_path.write_text(nuevo_contenido, encoding="utf-8")

    # 6. docs/referencia/CONFIGURACION_PACK.md
    pack_doc = raiz / "docs" / "referencia" / "CONFIGURACION_PACK.md"
    if pack_doc.exists():
        contenido = pack_doc.read_text(encoding="utf-8")
        nuevo_contenido = re.sub(
            r'(version\s*=\s*")[^"]+(")',
            rf"\g<1>{nueva_version}\g<2>",
            contenido,
            count=1,
        )
        nuevo_contenido = re.sub(
            r'(\|\s*`version`\s*\|\s*String\s*\|\s*")[^"]+(")',
            rf"\g<1>{nueva_version}\g<2>",
            nuevo_contenido,
            count=1,
        )
        if nuevo_contenido != contenido:
            cambios.append(
                f"docs/referencia/CONFIGURACION_PACK.md: Alineada referencia a {nueva_version}"
            )
            if not dry_run:
                pack_doc.write_text(nuevo_contenido, encoding="utf-8")

    # 7. docs/explicacion/roadmap-y-versionado.md
    roadmap_doc = raiz / "docs" / "explicacion" / "roadmap-y-versionado.md"
    if roadmap_doc.exists():
        contenido = roadmap_doc.read_text(encoding="utf-8")
        nuevo_contenido = re.sub(
            r"(La versión actual del repositorio es \*\*`)[^`]+(`\*\*)",
            rf"\g<1>{nueva_version}\g<2>",
            contenido,
            count=1,
        )
        if nuevo_contenido != contenido:
            cambios.append(
                f"docs/explicacion/roadmap-y-versionado.md: Actualizada mención a {nueva_version}"
            )
            if not dry_run:
                roadmap_doc.write_text(nuevo_contenido, encoding="utf-8")

    return cambios


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Gestor canónico y seguro de versionado para Blazing Recomp."
    )
    grupo_accion = parser.add_mutually_exclusive_group()
    grupo_accion.add_argument(
        "--check",
        action="store_true",
        help="Verifica la coherencia unánime de versiones entre manifiestos.",
    )
    grupo_accion.add_argument(
        "--patch",
        action="store_true",
        help="Incrementa el componente PATCH (ej. 0.0.3 -> 0.0.4).",
    )
    grupo_accion.add_argument(
        "--minor",
        action="store_true",
        help="Incrementa el componente MINOR (ej. 0.0.3 -> 0.1.0).",
    )
    grupo_accion.add_argument(
        "--major",
        action="store_true",
        help="Incrementa el componente MAJOR (ej. 0.0.3 -> 1.0.0).",
    )
    grupo_accion.add_argument(
        "--version",
        dest="version_explicita",
        type=str,
        help="Define una versión SemVer explícita (ej. 0.1.0).",
    )

    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Simula el incremento y muestra los cambios sin escribir en disco.",
    )
    parser.add_argument(
        "--mensaje",
        "-m",
        type=str,
        help="Mensaje de novedad o corrección para el CHANGELOG.",
    )
    parser.add_argument(
        "--si",
        "-y",
        action="store_true",
        help="Omite la confirmación interactiva de la persona.",
    )

    args = parser.parse_args()
    raiz = obtener_raiz_repo()

    # Si se pide solo chequeo
    if args.check:
        es_valido, errores, version_act, code_act = validar_sincronizacion(raiz)
        if es_valido:
            print(
                f"[OK] Todos los manifiestos están sincronizados en versión {version_act} (versionCode {code_act})."
            )
            return 0
        else:
            print("[ERROR] Se detectaron inconsistencias de versionado:")
            for err in errores:
                print(f"  - {err}")
            return 1

    # Determinar tipo de bump
    tipo_bump = None
    if args.patch:
        tipo_bump = "patch"
    elif args.minor:
        tipo_bump = "minor"
    elif args.major:
        tipo_bump = "major"
    elif args.version_explicita:
        tipo_bump = "custom"
    else:
        # Por defecto si no se pasa nada, mostrar estado actual
        print("Uso: python scripts/bump_version.py [--check | --patch | --minor | --major | --version <X.Y.Z>]")
        print("Para simular sin cambios use --dry-run. Para omitir confirmación use --si.")
        es_valido, _, version_act, code_act = validar_sincronizacion(raiz)
        print(f"\nEstado actual: Versión {version_act} | versionCode {code_act}")
        return 0

    # Validar primero sincronía actual antes de incrementar
    es_valido, errores, version_act, code_act = validar_sincronizacion(raiz)
    if not es_valido and not args.dry_run:
        print("[ERROR] No se puede incrementar porque existen inconsistencias previas:")
        for err in errores:
            print(f"  - {err}")
        print("Por favor resuelva las discrepancias antes de avanzar la versión.")
        return 1

    try:
        nueva_version, nuevo_code = calcular_siguiente_version(
            version_act, tipo_bump, args.version_explicita
        )
    except Exception as e:
        print(f"[ERROR] Error al calcular la nueva versión: {e}")
        return 1

    print("=" * 60)
    print("PLAN DE INCREMENTO DE VERSIÓN - BLAZING RECOMP")
    print("=" * 60)
    print(f"Versión actual:   {version_act}  (versionCode: {code_act})")
    print(f"Nueva versión:    {nueva_version}  (versionCode: {nuevo_code})")
    if args.mensaje:
        print(f"Mensaje release:  {args.mensaje.strip()}")
    print("-" * 60)

    # Simular cambios para visualización
    cambios_previstos = aplicar_cambios(
        raiz,
        version_act,
        nueva_version,
        nuevo_code,
        args.mensaje,
        dry_run=True,
    )
    print(f"Archivos a sincronizar ({len(cambios_previstos)}):")
    for c in cambios_previstos:
        print(f"  • {c}")
    print("=" * 60)

    if args.dry_run:
        print("\n[MODO SIMULACIÓN] No se realizaron modificaciones en disco (--dry-run).")
        return 0

    # Compuerta interactiva humana
    if not args.si:
        try:
            prompt = (
                f"\n¿Desea aplicar este incremento a {nueva_version} (versionCode {nuevo_code})? [s/N]: "
            )
            sys.stdout.write(prompt)
            sys.stdout.flush()
            respuesta = sys.stdin.readline().strip().lower()
            if respuesta not in ("s", "si", "y", "yes"):
                print("[CANCELADO] Operación cancelada por la persona. No se modificó ningún archivo.")
                return 1
        except (KeyboardInterrupt, EOFError):
            print("\n[CANCELADO] Operación interrumpida. No se modificó ningún archivo.")
            return 1

    # Aplicar cambios reales
    aplicar_cambios(
        raiz,
        version_act,
        nueva_version,
        nuevo_code,
        args.mensaje,
        dry_run=False,
    )

    # Validar que tras los cambios la sincronización sea unánime
    es_valido, errores, version_post, code_post = validar_sincronizacion(raiz)
    if not es_valido:
        print("\n[ADVERTENCIA] Los archivos se modificaron pero falló la validación final:")
        for err in errores:
            print(f"  - {err}")
        return 1

    print(
        f"\n[ÉXITO] Versión incrementada satisfactoriamente a {version_post} (versionCode {code_post})."
    )
    print("Todos los manifiestos y documentos han sido sincronizados unánimemente.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
