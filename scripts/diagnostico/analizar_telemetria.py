#!/usr/bin/env python3
"""
analizar_telemetria.py — Analizador y comparador de registros de telemetría de Blazing Recomp.

Procesa la salida de métricas emitida por el motor en Android (Vulkan / GBA Core)
y genera un informe de rendimiento con percentiles de fotogramas, tiempos de GPU/CPU,
el desglose de ejecución por región de memoria, el reparto de las líneas de la PPU entre
los caminos de composición (`# ppucamino`) y el ranking de los hotspots más demandantes
del intérprete de emulación.
Incluye soporte para contrastar dos sesiones (--comparar) para verificar mejoras AOT.

La línea de segundo (`t=`) ya no se lee con un regex posicional: sus campos los declara
`telemetria/esquema.txt`, el mismo fichero del que escribe el motor (`src/telemetria.rs`).
Así el orden de los campos deja de importar y un campo que el analizador no conoce se
reporta como anomalía en vez de descartarse en silencio.
"""

import argparse
import os
import re
import sys
from collections import Counter, namedtuple
from pathlib import Path

# El esquema vive en la raíz del repositorio; este script está en scripts/diagnostico/.
ESQUEMA_RUTA = Path(__file__).resolve().parents[2] / "telemetria" / "esquema.txt"

# Tipos que el esquema puede declarar, y cómo se convierte cada uno.
TIPOS = ("entero", "entero_s", "real", "real/real", "entero_x_entero")

# Una fila `campo` del esquema.
Campo = namedtuple("Campo", "nombre tipo unidad")


class Esquema:
    """El contrato de la línea de segundo, ya analizado."""

    def __init__(self, campos, orden, versiones, sello_clave, sello_version):
        self.campos = campos              # nombre -> Campo
        self.orden = orden                # nombres en orden canónico de escritura
        self.versiones = versiones        # versión -> lista de nombres que añade
        self.sello_clave = sello_clave
        self.sello_version = sello_version
        self.hereda = {}                  # versión -> versión padre o None

    def campos_de_version(self, version):
        """Nombres acumulados hasta `version`, siguiendo la cadena `hereda`."""
        nombres = []
        actual = version
        while actual is not None:
            nombres = list(self.versiones[actual]) + nombres
            actual = self.hereda.get(actual)
        return nombres


def cargar_esquema(ruta=ESQUEMA_RUTA):
    """Lee y valida `telemetria/esquema.txt`.

    Comprueba los tipos declarados, que no haya nombres repetidos y que cada bloque
    `version` recoja exactamente los `campo` que dice añadir, en el mismo orden.
    """
    ruta = Path(ruta)
    bloques = []
    sello = None

    for numero, bruta in enumerate(
        ruta.read_text(encoding="utf-8", errors="replace").splitlines(), 1
    ):
        linea = bruta.strip()
        if not linea or linea.startswith("#"):
            continue
        celdas = [celda.strip() for celda in linea.split(" | ")]
        clave = celdas[0]

        if clave == "version":
            if len(celdas) < 4:
                raise ValueError(f"línea {numero}: una fila 'version' necesita 4 celdas")
            try:
                version = int(celdas[1])
            except ValueError:
                raise ValueError(f"línea {numero}: la versión '{celdas[1]}' no es un entero")
            hereda_texto = celdas[2].partition(":")[2].strip()
            try:
                hereda = None if hereda_texto == "-" else int(hereda_texto)
            except ValueError:
                raise ValueError(f"línea {numero}: 'hereda' no es un entero: '{hereda_texto}'")
            anade_texto = celdas[3].partition(":")[2]
            anade = [nombre.strip() for nombre in anade_texto.split(",") if nombre.strip()]
            bloques.append(
                {"version": version, "hereda": hereda, "anade": anade, "recogidos": []}
            )

        elif clave == "campo":
            if len(celdas) < 5:
                raise ValueError(f"línea {numero}: una fila 'campo' necesita 5 celdas")
            if not bloques:
                raise ValueError(f"línea {numero}: un 'campo' antes de cualquier 'version'")
            tipo = celdas[2]
            if tipo not in TIPOS:
                raise ValueError(
                    f"línea {numero}: tipo desconocido '{tipo}' en el campo '{celdas[1]}'"
                )
            if any(campo.nombre == celdas[1] for campo in bloques[-1]["recogidos"]):
                raise ValueError(
                    f"línea {numero}: el campo '{celdas[1]}' está repetido en la versión "
                    f"{bloques[-1]['version']}"
                )
            bloques[-1]["recogidos"].append(Campo(celdas[1], tipo, celdas[3]))

        elif clave == "sello":
            if len(celdas) < 3:
                raise ValueError(f"línea {numero}: una fila 'sello' necesita 3 celdas")
            try:
                sello = (celdas[1], int(celdas[2]))
            except ValueError:
                raise ValueError(f"línea {numero}: el sello apunta a una versión que no es un entero")

        else:
            raise ValueError(f"línea {numero}: fila desconocida '{clave}'")

    if not bloques:
        raise ValueError("el esquema no declara ninguna versión")

    for bloque in bloques:
        recogidos = [campo.nombre for campo in bloque["recogidos"]]
        if bloque["anade"] != recogidos:
            raise ValueError(
                f"la versión {bloque['version']} declara añadir {bloque['anade']} "
                f"pero recoge {recogidos}"
            )

    if sello is None:
        raise ValueError("el esquema no declara el sello de versión")
    sello_clave, sello_version = sello

    versiones = {}
    hereda = {}
    for bloque in bloques:
        versiones[bloque["version"]] = [campo.nombre for campo in bloque["recogidos"]]
        hereda[bloque["version"]] = bloque["hereda"]
    if sello_version not in versiones:
        raise ValueError(
            f"el sello apunta a la versión {sello_version}, que el esquema no declara"
        )

    campos = {}
    orden = []
    for bloque in bloques:
        for campo in bloque["recogidos"]:
            campos[campo.nombre] = campo
            orden.append(campo.nombre)

    esquema = Esquema(campos, orden, versiones, sello_clave, sello_version)
    esquema.hereda = hereda
    return esquema


def _entero(texto):
    if not re.fullmatch(r"\d+", texto):
        raise ValueError(texto)
    return int(texto)


def _real(texto):
    if not re.fullmatch(r"[+-]?\d+(?:\.\d+)?", texto):
        raise ValueError(texto)
    return float(texto)


def convertir(texto, tipo):
    """Convierte el texto de un campo al valor tipado que declara el esquema."""
    if tipo == "entero":
        return _entero(texto)
    if tipo == "entero_s":
        if not texto.endswith("s"):
            raise ValueError(texto)
        return _entero(texto[:-1])
    if tipo == "real":
        return _real(texto)
    if tipo == "real/real":
        partes = texto.split("/")
        if len(partes) != 2:
            raise ValueError(texto)
        return (_real(partes[0]), _real(partes[1]))
    if tipo == "entero_x_entero":
        partes = texto.split("x")
        if len(partes) != 2:
            raise ValueError(texto)
        return (_entero(partes[0]), _entero(partes[1]))
    raise ValueError(tipo)


def formatear(valor, tipo):
    """Imprime un valor con la misma forma que el escritor de Rust.

    Es el espejo de `Tipo::formatear` en `src/telemetria.rs`: si los dos cambian
    a la vez, el test de oro detecta la deriva.
    """
    if tipo == "entero":
        return str(valor)
    if tipo == "entero_s":
        return f"{valor}s"
    if tipo == "real":
        return f"{valor:.1f}"
    if tipo == "real/real":
        return f"{valor[0]:.1f}/{valor[1]:.1f}"
    if tipo == "entero_x_entero":
        return f"{valor[0]}x{valor[1]}"
    raise ValueError(tipo)


# Una línea `t=` se reconoce por su token de segundo, esté donde esté: el orden de
# los campos ya no importa, así que tampoco importa dónde caiga `t=`.
_RE_SEGUNDO = re.compile(r"(?:^|\s)t=\d+s(?:\s|$)")


def _es_linea_de_segundo(linea):
    return _RE_SEGUNDO.search(linea) is not None


def parsear_linea(linea, esquema):
    """Convierte una línea `t=` en valores tipados, sello y anomalías.

    Devuelve un diccionario con `valores` (sólo los campos que el esquema conoce),
    `sello` (la versión declarada, o None si la línea no la trae) y `anomalias`.
    Una línea sin sello se lee en modo tolerante: los campos que falten no se
    echan de menos, porque un registro viejo no puede declarar lo que aún no existía.
    """
    crudos = {}
    anomalias = []

    for token in linea.split():
        clave, separador, texto = token.partition("=")
        if not separador:
            anomalias.append(f"token sin clave=valor: {token}")
            continue
        if clave in crudos:
            anomalias.append(f"clave repetida: {clave}")
            continue
        crudos[clave] = texto

    sello = None
    if esquema.sello_clave in crudos:
        texto = crudos.pop(esquema.sello_clave)
        try:
            version = int(texto)
        except ValueError:
            anomalias.append(f"sello inválido: {texto}")
        else:
            if version in esquema.versiones:
                sello = version
            else:
                anomalias.append(f"versión desconocida: {version}")

    valores = {}
    for clave, texto in crudos.items():
        campo = esquema.campos.get(clave)
        if campo is None:
            anomalias.append(f"campo desconocido: {clave}")
            continue
        try:
            valores[clave] = convertir(texto, campo.tipo)
        except ValueError:
            anomalias.append(f"valor inválido en {clave}: {texto}")

    # Sólo una línea sellada se comprueba contra la lista de campos de su versión:
    # en una línea vieja no se puede distinguir «no lo medía» de «se perdió».
    if sello is not None:
        for nombre in esquema.campos_de_version(sello):
            if nombre not in crudos:
                anomalias.append(f"falta el campo {nombre}")

    return {"valores": valores, "sello": sello, "anomalias": anomalias}


def parsear_telemetria(ruta_o_stream):
    esquema = cargar_esquema()
    hotspots = Counter()
    anomalias = Counter()
    # Acumulado por región de memoria: la última línea `# regiones` del archivo contiene
    # los totales de la sesión, así que cada lectura sobrescribe la anterior.
    regiones = {}
    # Reparto de las líneas de la PPU entre caminos de composición (`# ppucamino`), con la
    # misma regla: la última línea del archivo contiene los acumulados de la sesión.
    camino_ppu = {}
    ppu_hw_muestras = []
    # Una entrada por línea de segundo, ya tipada, en el orden del archivo.
    lineas_segundo = []

    lineas = ruta_o_stream.readlines() if hasattr(ruta_o_stream, "readlines") else []

    for linea in lineas:
        linea = linea.strip()
        if not linea:
            continue

        if linea.startswith("# hotspots top="):
            matches = re.findall(r"(0x[0-9A-Fa-f]+)\s*\((\d+)\)", linea)
            for addr, count in matches:
                hotspots[addr.upper()] += int(count)

        elif linea.startswith("# regiones"):
            for nombre, interp, nativos in re.findall(r"(\w+)=(\d+)/(\d+)", linea):
                regiones[nombre.lower()] = (int(interp), int(nativos))

        elif linea.startswith("# ppucamino"):
            # La cola `(rectas=NN%)` repite la clave `rectas` con un porcentaje, así que
            # se corta antes de parsear: si no, el conteo se perdería quedándose el tanto por ciento.
            cuerpo = linea.split("(")[0]
            campos = {k: int(v) for k, v in re.findall(r"(\w+)=(\d+)", cuerpo)}
            if campos:
                camino_ppu = campos

        elif linea.startswith("# ppu_hw"):
            campos = {k: int(v) for k, v in re.findall(r"(\w+)=(\d+)", linea)}
            if campos:
                ppu_hw_muestras.append(campos)

        elif _es_linea_de_segundo(linea):
            leida = parsear_linea(linea, esquema)
            lineas_segundo.append(leida)
            for mensaje in leida["anomalias"]:
                anomalias[mensaje] += 1

    total_seg = len(lineas_segundo)
    if total_seg == 0:
        return None

    def serie(nombre):
        """Valores de un campo, sólo de las líneas que lo traen."""
        return [leida["valores"][nombre] for leida in lineas_segundo if nombre in leida["valores"]]

    def promedio(valores):
        return sum(valores) / len(valores) if valores else 0.0

    fps_muestras = serie("fps")
    fps_activos = [x for x in fps_muestras if x > 5.0]
    prom_fps = promedio(fps_activos)
    min_fps = min(fps_activos) if fps_activos else 0.0
    max_fps = max(fps_activos) if fps_activos else 0.0

    # `ejec` y `dibujo` son pares promedio/máximo: el primer componente es la media
    # del segundo y el segundo es el peor fotograma de ese segundo. El peor fotograma
    # de la sesión es el máximo de los segundos componentes, no de las medias.
    ejec = serie("ejec")
    prom_ejec = promedio([prom for prom, _ in ejec])
    max_ejec = max((prom for prom, _ in ejec), default=0.0)
    max_ejec_fotograma = max((peor for _, peor in ejec), default=0.0)

    dibujo = serie("dibujo")
    prom_dibujo = promedio([prom for prom, _ in dibujo])
    max_dibujo = max((prom for prom, _ in dibujo), default=0.0)
    max_dibujo_fotograma = max((peor for _, peor in dibujo), default=0.0)

    ppu_bg_muestras = serie("ppubg")
    ppu_spr_muestras = serie("ppuspr")
    ppu_comp_muestras = serie("ppucomp")
    present_max_muestras = serie("presentmax")

    nativas_instrucciones = serie("nativos")
    interp_instrucciones = serie("interprete")

    total_nativas = sum(nativas_instrucciones)
    total_interp = sum(interp_instrucciones)
    total_total = total_nativas + total_interp
    ratio_aot = (total_nativas / total_total * 100.0) if total_total > 0 else 0.0

    return {
        "esquema": esquema,
        "lineas": lineas_segundo,
        "anomalias": anomalias,
        "segundos": total_seg,
        "prom_fps": prom_fps,
        "min_fps": min_fps,
        "max_fps": max_fps,
        "prom_ejec": prom_ejec,
        "max_ejec": max_ejec,
        "max_ejec_fotograma": max_ejec_fotograma,
        "prom_ppu": promedio(serie("ppu")),
        "prom_ppubg": promedio(ppu_bg_muestras) if ppu_bg_muestras else None,
        "prom_ppuspr": promedio(ppu_spr_muestras) if ppu_spr_muestras else None,
        "prom_ppucomp": promedio(ppu_comp_muestras) if ppu_comp_muestras else None,
        "prom_apu": promedio(serie("apu")),
        "prom_dibujo": prom_dibujo,
        "max_dibujo": max_dibujo,
        "max_dibujo_fotograma": max_dibujo_fotograma,
        "prom_present": promedio(serie("present")),
        "max_present": max(present_max_muestras) if present_max_muestras else 0.0,
        "total_nativas": total_nativas,
        "total_interp": total_interp,
        "ratio_aot": ratio_aot,
        "hotspots": hotspots,
        "regiones": regiones,
        "camino_ppu": camino_ppu,
        "ppu_hw": {
            "gpu_total": sum(m.get("gpu", 0) for m in ppu_hw_muestras),
            "sw_total": sum(m.get("sw", 0) for m in ppu_hw_muestras),
            "nativa_total": sum(m.get("nativa", 0) for m in ppu_hw_muestras),
            "modo0_total": sum(m.get("modo0", 0) for m in ppu_hw_muestras),
            "win_total": sum(m.get("win", 0) for m in ppu_hw_muestras),
            "blend_total": sum(m.get("blend", 0) for m in ppu_hw_muestras),
            "spr_total_avg": round(sum(m.get("spr_total", 0) for m in ppu_hw_muestras) / len(ppu_hw_muestras), 1) if ppu_hw_muestras else 0.0,
            "spr_8bpp_avg": round(sum(m.get("spr_8bpp", 0) for m in ppu_hw_muestras) / len(ppu_hw_muestras), 1) if ppu_hw_muestras else 0.0,
            "spr_afin_avg": round(sum(m.get("spr_afin", 0) for m in ppu_hw_muestras) / len(ppu_hw_muestras), 1) if ppu_hw_muestras else 0.0,
        } if ppu_hw_muestras else None,
    }


def volcar(d):
    """Vuelca en TSV lo que el lector entendió, para compararlo con el escritor.

    Un dato por fila, separado por tabuladores:

        resumen <TAB> <clave> <TAB> <valor>
        linea <TAB> <índice>
        campo <TAB> <nombre> <TAB> <valor>
        sello <TAB> <versión>
        anomalia <TAB> <mensaje>

    Los `campo` van en el orden canónico del esquema y con el valor impreso por el
    propio formateador del esquema, o sea con el mismo texto que escribió el motor.
    """
    print(f"resumen\tsegundos\t{d['segundos']}")
    print(f"resumen\tprom_fps\t{d['prom_fps']:.1f}")
    print(f"resumen\tprom_ejec\t{d['prom_ejec']:.2f}")
    print(f"resumen\tmax_ejec\t{d['max_ejec']:.2f}")
    print(f"resumen\tmax_ejec_fotograma\t{d['max_ejec_fotograma']:.2f}")
    print(f"resumen\tmax_dibujo\t{d['max_dibujo']:.2f}")
    print(f"resumen\tmax_dibujo_fotograma\t{d['max_dibujo_fotograma']:.2f}")
    print(f"resumen\tanomalias\t{sum(d['anomalias'].values())}")

    esquema = d["esquema"]
    for indice, leida in enumerate(d["lineas"], 1):
        print(f"linea\t{indice}")
        for nombre in esquema.orden:
            if nombre in leida["valores"]:
                campo = esquema.campos[nombre]
                print(f"campo\t{nombre}\t{formatear(leida['valores'][nombre], campo.tipo)}")
        if leida["sello"] is not None:
            print(f"sello\t{leida['sello']}")
        for mensaje in leida["anomalias"]:
            print(f"anomalia\t{mensaje}")


def imprimir_informe(d, top_n=20, titulo="INFORME DE TELEMETRÍA — BLAZING RECOMP"):
    print("=" * 72)
    print(f" {titulo}")
    print("=" * 72)
    print(f"Duración analizada:         {d['segundos']} segundos")
    print(f"Tasa de fotogramas (FPS):   {d['prom_fps']:.1f} FPS (mín: {d['min_fps']:.1f}, máx: {d['max_fps']:.1f})")
    print("-" * 72)
    print("DESGLOSE DE TIEMPOS POR FOTOGRAMA:")
    print(f"  • CPU Emulación (ejec):   {d['prom_ejec']:5.2f} ms  (peor media de segundo: {d['max_ejec']:.2f} ms)")
    print(f"      peor fotograma:       {d['max_ejec_fotograma']:5.2f} ms")
    print(f"  • PPU Software:           {d['prom_ppu']:5.2f} ms")
    if d.get("prom_ppubg") is not None:
        print(f"      - Fondos (BG):        {d['prom_ppubg']:5.2f} ms")
        print(f"      - Sprites (OBJ):      {d['prom_ppuspr']:5.2f} ms")
        print(f"      - Composición/Blend:  {d['prom_ppucomp']:5.2f} ms")
    print(f"  • APU Audio:              {d['prom_apu']:5.2f} ms")
    print(f"  • Render GPU (dibujo):    {d['prom_dibujo']:5.2f} ms  (peor media de segundo: {d['max_dibujo']:.2f} ms)")
    print(f"      peor fotograma:       {d['max_dibujo_fotograma']:5.2f} ms")
    if d.get("max_present"):
        print(f"  • Presentación Swapchain: {d['prom_present']:5.2f} ms  (peor fotograma: {d['max_present']:.2f} ms)")
    else:
        print(f"  • Presentación Swapchain: {d['prom_present']:5.2f} ms")
    print("-" * 72)
    print("COBERTURA DE EJECUCIÓN (AOT vs INTÉRPRETE):")
    print(f"  • Instrucciones Nativas AOT:  {d['total_nativas']:>12,} ({d['ratio_aot']:5.1f}%)")
    print(f"  • Instrucciones Intérprete:   {d['total_interp']:>12,} ({100.0 - d['ratio_aot']:5.1f}%)")
    print("=" * 72)

    anomalias = d.get("anomalias") or {}
    if anomalias:
        print("\nANOMALÍAS DE FORMATO EN LA LÍNEA DE SEGUNDO:")
        for mensaje, veces in anomalias.most_common(10):
            sufijo = f"  (x{veces})" if veces > 1 else ""
            print(f"  • {mensaje}{sufijo}")
        print(
            f"total: {sum(anomalias.values())} anomalía(s) en {d['segundos']} línea(s) de segundo."
        )
        print("El esquema es telemetria/esquema.txt: un campo que no esté ahí sobra o falta en uno de los dos lados.")
        print("=" * 72)

    ppu_hw = d.get("ppu_hw")
    if ppu_hw:
        print("\nCENSO DE HARDWARE GBA Y RUTA DE RENDERIZADO (PPU):")
        print(f"  • Ruta activa:                GPU Shim={ppu_hw['gpu_total']} frames | Fallback SW={ppu_hw['sw_total']} frames")
        print(f"  • PPU nativa:                 {ppu_hw['nativa_total']} frames (dentro de los de GPU)")
        print(f"  • Pantalla Modo 0:            {ppu_hw['modo0_total']} frames")
        print(f"  • Ventanas activas (WIN0/1):  {ppu_hw['win_total']} frames")
        print(f"  • Efectos Blend / Fade:       {ppu_hw['blend_total']} frames")
        print(f"  • Sprites OAM promedio/frame: Total={ppu_hw['spr_total_avg']} | 8bpp={ppu_hw['spr_8bpp_avg']} | Afines={ppu_hw['spr_afin_avg']}")
        print("=" * 72)

    regiones = d.get("regiones") or {}
    if regiones:
        print("\nDESGLOSE DE EJECUCIÓN POR REGIÓN DE MEMORIA (acumulado de la sesión):")
        print(f"{'Región':<8} | {'Pasos intérprete':>18} | {'% del intérprete':>16} | {'Bloques nativos':>15}")
        print("-" * 68)
        total_interp_region = sum(v[0] for v in regiones.values())
        for nombre in ("rom", "iwram", "ewram", "bios"):
            if nombre not in regiones:
                continue
            pasos, nativos = regiones[nombre]
            pct = (pasos / total_interp_region * 100.0) if total_interp_region > 0 else 0.0
            print(f"{nombre:<8} | {pasos:>18,} | {pct:>15.1f}% | {nativos:>15,}")
        print("-" * 68)
        print("Los bloques nativos se cuentan por bloque y los pasos de intérprete por instrucción:")
        print("la división entre ambos da una fracción de eventos de despacho, no de instrucciones.")
        print("=" * 72)
    else:
        print()
        print("Aviso: sin líneas `# regiones` en el log (registros anteriores al 2026-10-12).")
        print("=" * 72)

    camino = d.get("camino_ppu") or {}
    if camino:
        total_lineas = sum(camino.get(k, 0) for k in ("rectas", "mezcla", "ventana", "semi"))
        print("\nREPARTO DE LÍNEAS DE LA PPU POR CAMINO DE COMPOSICIÓN (acumulado de la sesión):")
        print(f"{'Camino':<10} | {'Líneas':>12} | {'% del total':>12}")
        print("-" * 40)
        for nombre in ("rectas", "mezcla", "ventana", "semi"):
            if nombre not in camino:
                continue
            valor = camino[nombre]
            pct = (valor / total_lineas * 100.0) if total_lineas > 0 else 0.0
            print(f"{nombre:<10} | {valor:>12,} | {pct:>11.1f}%")
        print("-" * 40)
        print(f"líneas contadas en la sesión: {total_lineas:,}")
        efectos = [
            (nombre, camino[nombre])
            for nombre in ("inertes", "efecto", "efalpha", "efbrillo", "efoscuro")
            if nombre in camino
        ]
        if efectos:
            print("Efectos de color, sobre las líneas contadas:")
            for nombre, valor in efectos:
                pct = (valor / total_lineas * 100.0) if total_lineas > 0 else 0.0
                print(f"  • {nombre:<9} {valor:>12,} ({pct:5.1f}% de las líneas)")
        print("`rectas` es la vía rápida; el resto son líneas que necesitaron la segunda capa.")
        print("=" * 72)

    hotspots = d["hotspots"]
    if hotspots:
        print(f"\nRANKING DE HOTSPOTS (TOP {top_n} DIRECCIONES EN EL INTÉRPRETE):")
        print(f"{'Dirección':<14} | {'Ticks':>10} | {'% Relativo':>10}")
        print("-" * 40)
        total_ticks = sum(hotspots.values())
        for addr, count in hotspots.most_common(top_n):
            pct = (count / total_ticks * 100.0) if total_ticks > 0 else 0.0
            print(f"{addr:<14} | {count:>10,} | {pct:>9.2f}%")
        print("=" * 72)


def comparar_sesiones(base, nuevo, nombre_base="Antes", nombre_nuevo="Nuevo"):
    print("=" * 72)
    print(" INFORME COMPARATIVO DE TELEMETRÍA — VERIFICACIÓN AOT")
    print("=" * 72)
    print(f"{'Métrica':<28} | {nombre_base:<15} | {nombre_nuevo:<15} | {'Diferencia':<12}")
    print("-" * 72)

    def diff_num(v_ant, v_nue, fmt="{:+.1f}", suf=""):
        delta = v_nue - v_ant
        return f"{fmt.format(delta)}{suf}"

    # FPS
    print(f"{'Tasa de FPS:':<28} | {base['prom_fps']:>5.1f} FPS       | {nuevo['prom_fps']:>5.1f} FPS       | {diff_num(base['prom_fps'], nuevo['prom_fps'], '{:+.1f}', ' FPS')}")
    # CPU ejec
    print(f"{'CPU Emulación (ejec):':<28} | {base['prom_ejec']:>5.2f} ms        | {nuevo['prom_ejec']:>5.2f} ms        | {diff_num(base['prom_ejec'], nuevo['prom_ejec'], '{:+.2f}', ' ms')}")
    # Render GPU
    print(f"{'Render GPU (dibujo):':<28} | {base['prom_dibujo']:>5.2f} ms        | {nuevo['prom_dibujo']:>5.2f} ms        | {diff_num(base['prom_dibujo'], nuevo['prom_dibujo'], '{:+.2f}', ' ms')}")
    # Swapchain
    print(f"{'Presentación Swapchain:':<28} | {base['prom_present']:>5.2f} ms        | {nuevo['prom_present']:>5.2f} ms        | {diff_num(base['prom_present'], nuevo['prom_present'], '{:+.2f}', ' ms')}")
    print("-" * 72)
    # Cobertura AOT
    print(f"{'Cobertura Nativa AOT:':<28} | {base['ratio_aot']:>5.1f} %        | {nuevo['ratio_aot']:>5.1f} %        | {diff_num(base['ratio_aot'], nuevo['ratio_aot'], '{:+.1f}', ' %')}")
    # Instrucciones intérprete
    red_interp = ((nuevo['total_interp'] - base['total_interp']) / base['total_interp'] * 100.0) if base['total_interp'] > 0 else 0.0
    print(f"{'Instrucciones Intérprete:':<28} | {base['total_interp']:>10,}     | {nuevo['total_interp']:>10,}     | {red_interp:+.1f} %")
    print("=" * 72)

    # Estado de los top hotspots del log base en el nuevo log
    print(f"\nESTADO DE LOS TOP HOTSPOTS DE '{nombre_base}' EN '{nombre_nuevo}':")
    print(f"{'Dirección':<14} | {'Ticks ' + nombre_base:>12} | {'Ticks ' + nombre_nuevo:>12} | {'Reducción':<12}")
    print("-" * 56)
    for addr, cnt_base in base["hotspots"].most_common(10):
        cnt_nuevo = nuevo["hotspots"].get(addr, 0)
        red_pct = ((cnt_nuevo - cnt_base) / cnt_base * 100.0) if cnt_base > 0 else 0.0
        estado = f"{red_pct:+.1f} %" if cnt_nuevo > 0 else "ELIMINADO (100% NATIVO)"
        print(f"{addr:<14} | {cnt_base:>12,} | {cnt_nuevo:>12,} | {estado}")
    print("=" * 72)


def main():
    parser = argparse.ArgumentParser(
        description="Analiza y compara archivos de telemetría de rendimiento de Blazing Recomp."
    )
    parser.add_argument(
        "--archivo",
        "-f",
        type=str,
        default=None,
        help="Ruta al archivo de log de telemetría (si se omite, lee de stdin).",
    )
    parser.add_argument(
        "--comparar",
        "-c",
        type=str,
        default=None,
        help="Ruta al archivo base para comparar (genera informe comparativo Antes vs Después).",
    )
    parser.add_argument(
        "--top",
        "-t",
        type=int,
        default=20,
        help="Número de hotspots a mostrar en el ranking (defecto: 20).",
    )
    parser.add_argument(
        "--volcar",
        action="store_true",
        help="Vuelca en TSV lo que el lector entendió (para compararlo con lo que escribió el motor).",
    )
    args = parser.parse_args()

    if args.comparar:
        if not args.archivo:
            print("Error: Para usar --comparar debes especificar también --archivo <nuevo_log>.", file=sys.stderr)
            sys.exit(1)
        if not os.path.isfile(args.comparar):
            print(f"Error: No se encontró el archivo base '{args.comparar}'.", file=sys.stderr)
            sys.exit(1)
        if not os.path.isfile(args.archivo):
            print(f"Error: No se encontró el archivo nuevo '{args.archivo}'.", file=sys.stderr)
            sys.exit(1)

        with open(args.comparar, "r", encoding="utf-8", errors="replace") as fb:
            datos_base = parsear_telemetria(fb)
        with open(args.archivo, "r", encoding="utf-8", errors="replace") as fn:
            datos_nuevo = parsear_telemetria(fn)

        if not datos_base or not datos_nuevo:
            print("Error: No se pudieron extraer muestras válidas de ambos archivos.", file=sys.stderr)
            sys.exit(1)

        comparar_sesiones(
            datos_base,
            datos_nuevo,
            nombre_base=os.path.basename(args.comparar),
            nombre_nuevo=os.path.basename(args.archivo),
        )
        return

    if args.archivo:
        if not os.path.isfile(args.archivo):
            print(f"Error: No se encontró el archivo '{args.archivo}'.", file=sys.stderr)
            sys.exit(1)
        with open(args.archivo, "r", encoding="utf-8", errors="replace") as f:
            d = parsear_telemetria(f)
    else:
        if sys.stdin.isatty():
            parser.print_help()
            sys.exit(1)
        d = parsear_telemetria(sys.stdin)

    if not d:
        print("Aviso: No se encontraron muestras de fotogramas ('t=Xs fps=...') en la entrada.")
    elif args.volcar:
        volcar(d)
    else:
        imprimir_informe(d, top_n=args.top)


if __name__ == "__main__":
    main()
