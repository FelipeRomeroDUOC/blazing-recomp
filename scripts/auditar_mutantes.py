#!/usr/bin/env python3
"""
Auditor de Mutación para Blazing Recomp.

Aplica mutaciones intencionales representativas en la lógica pura (operadores relacionales,
lógicos, aritméticos, constantes y retornos) en src/input.rs, src/overlay.rs y src/error_screen.rs.
Para cada mutante, ejecuta `cargo test --lib` y verifica si la suite detecta y elimina el mutante.
Restaura siempre el archivo a su estado original.
"""

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

MUTANTES = [
    # --- src/input.rs ---
    {
        "id": "M01",
        "archivo": "src/input.rs",
        "desc": "Operador relacional en TouchArea::contains (<= a <)",
        "original": "dx * dx + dy * dy <= self.radius * self.radius",
        "mutado": "dx * dx + dy * dy < self.radius * self.radius",
    },
    {
        "id": "M02",
        "archivo": "src/input.rs",
        "desc": "Inversión lógica en TouchController::touch_down (!self.touch_enabled)",
        "original": "if !self.touch_enabled {",
        "mutado": "if self.touch_enabled {",
    },
    {
        "id": "M03",
        "archivo": "src/input.rs",
        "desc": "Descarte de puntero repetido en TouchController::touch_down",
        "original": "if self.pressed.iter().any(|(id, _)| *id == pointer_id) {",
        "mutado": "if false {",
    },
    {
        "id": "M04",
        "archivo": "src/input.rs",
        "desc": "Constante de máscara KEYINPUT en TouchController::key_mask (0x03FF a 0x01FF)",
        "original": "!pressed_bits & 0x03FF",
        "mutado": "!pressed_bits & 0x01FF",
    },
    {
        "id": "M05",
        "archivo": "src/input.rs",
        "desc": "Inversión de condición en TouchController::is_pressed (any a all)",
        "original": "self.pressed.iter().any(|(_, b)| *b == button)",
        "mutado": "!self.pressed.is_empty() && self.pressed.iter().all(|(_, b)| *b == button)",
    },
    {
        "id": "M06",
        "archivo": "src/input.rs",
        "desc": "Colisión de bitmask en Button::key_bit (B a 0x0001 en lugar de 0x0002)",
        "original": "Button::B => 0x0002,",
        "mutado": "Button::B => 0x0001,",
    },
    {
        "id": "M07",
        "archivo": "src/input.rs",
        "desc": "Arm de cruceta colapsado en layout (0.55 a 0.15, invade deadzone)",
        "original": "let arm = dpad_radius * 0.55;",
        "mutado": "let arm = dpad_radius * 0.15;",
    },
    {
        "id": "M08",
        "archivo": "src/input.rs",
        "desc": "Anulación de ángulo de inclinación SP (16.0 a 0.0)",
        "original": "let tilt = 16.0f32.to_radians();",
        "mutado": "let tilt = 0.0f32.to_radians();",
    },

    # --- src/overlay.rs ---
    {
        "id": "M09",
        "archivo": "src/overlay.rs",
        "desc": "Opacidad en Landscape en build_globals (0.65 a 0.95)",
        "original": "crate::input::Orientation::Landscape => 0.65,",
        "mutado": "crate::input::Orientation::Landscape => 0.95,",
    },
    {
        "id": "M10",
        "archivo": "src/overlay.rs",
        "desc": "Carcasa shell en Portrait en build_globals (1.0 a 0.0)",
        "original": "shell: match orientation {\n            crate::input::Orientation::Portrait => 1.0,",
        "mutado": "shell: match orientation {\n            crate::input::Orientation::Portrait => 0.0,",
    },
    {
        "id": "M11",
        "archivo": "src/overlay.rs",
        "desc": "Alineación de uniforms en Globals::pack (intercambio opacity y shell)",
        "original": "[self.res_x, self.res_y, self.opacity, self.shell]",
        "mutado": "[self.res_x, self.res_y, self.shell, self.opacity]",
    },
    {
        "id": "M12",
        "archivo": "src/overlay.rs",
        "desc": "Asignación de glifo invertida en button_glyph (Up a ArrowDown)",
        "original": "Up => Glyph::ArrowUp,",
        "mutado": "Up => Glyph::ArrowDown,",
    },
    {
        "id": "M13",
        "archivo": "src/overlay.rs",
        "desc": "Alineación uniforme en ElemParams::pack (pressed forzado a 0.0)",
        "original": "self.pressed,",
        "mutado": "0.0,",
    },

    # --- src/error_screen.rs ---
    {
        "id": "M14",
        "archivo": "src/error_screen.rs",
        "desc": "Preferencia invertida en select_surface_format_and_alpha (!f.is_srgb() a f.is_srgb())",
        "original": ".find(|f| !f.is_srgb())",
        "mutado": ".find(|f| f.is_srgb())",
    },
    {
        "id": "M15",
        "archivo": "src/error_screen.rs",
        "desc": "Desbordamiento de memoria sobre padding en ErrorScreenRenderer::render (width a stride)",
        "original": "for x in 0..width {",
        "mutado": "for x in 0..stride {",
    },
    {
        "id": "M16",
        "archivo": "src/error_screen.rs",
        "desc": "Eliminación de fallback seguro en select_surface_format_and_alpha (unwrap directo)",
        "original": ".unwrap_or(wgpu::TextureFormat::Rgba8Unorm);",
        "mutado": ".unwrap();",
    },
]


def test_baseline():
    """Verifica que la suite pase en verde antes de mutar."""
    print("Verificando línea base (suite en verde)...")
    res = subprocess.run(
        ["cargo", "test", "--lib"],
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if res.returncode != 0:
        print("ERROR: La suite de pruebas falla antes de aplicar mutantes:")
        print(res.stdout)
        print(res.stderr)
        sys.exit(1)
    print("Línea base OK: Suite 100% en verde.\n")


def ejecutar_mutacion():
    test_baseline()

    total = len(MUTANTES)
    eliminados = 0
    supervivientes = []

    print(f"Iniciando auditoría sobre {total} mutantes dirigidos...\n" + "=" * 70)

    for m in MUTANTES:
        m_id = m["id"]
        rel_path = m["archivo"]
        desc = m["desc"]
        original = m["original"]
        mutado = m["mutado"]

        abs_path = REPO_ROOT / rel_path
        contenido_original = abs_path.read_text(encoding="utf-8")

        if original not in contenido_original:
            print(f"[{m_id}] ERROR: Patrón original no encontrado en {rel_path}: {repr(original[:40])}...")
            continue

        # Aplicar mutación
        nuevo_contenido = contenido_original.replace(original, mutado, 1)
        abs_path.write_text(nuevo_contenido, encoding="utf-8")

        try:
            # Ejecutar tests sobre el código mutado
            res = subprocess.run(
                ["cargo", "test", "--lib"],
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=60,
            )

            if res.returncode != 0:
                # El test falló -> Mutante ELIMINADO (Éxito)
                eliminados += 1
                print(f"[{m_id}] ELIMINADO (Killed): {desc}")
            else:
                # El test pasó -> Mutante SOBREVIVIÓ (Fallo en la suite)
                supervivientes.append((m_id, desc, rel_path))
                print(f"[{m_id}] SOBREVIVIÓ (Survived): {desc} [!] PUNTO CIEGO")

        except subprocess.TimeoutExpired:
            # Bucle infinito inducido -> Mutante eliminado por timeout
            eliminados += 1
            print(f"[{m_id}] ELIMINADO POR TIMEOUT: {desc}")
        finally:
            # Restaurar fielmente el contenido original desde memoria
            abs_path.write_text(contenido_original, encoding="utf-8")

    print("=" * 70)
    score = (eliminados / total) * 100.0
    print(f"\nResumen de Mutación:")
    print(f"  Total Mutantes: {total}")
    print(f"  Eliminados:     {eliminados}")
    print(f"  Sobrevivientes: {len(supervivientes)}")
    print(f"  Mutation Score: {score:.1f}%")

    if supervivientes:
        print("\nMutantes que sobrevivieron (requieren nuevos tests):")
        for s_id, s_desc, s_path in supervivientes:
            print(f"  - [{s_id}] {s_desc} ({s_path})")
        sys.exit(1)
    else:
        print("\nRESULTADO EXCELENTE: 100% de los mutantes fueron eliminados por la suite de tests.")
        sys.exit(0)


if __name__ == "__main__":
    ejecutar_mutacion()
