use super::*;

#[test]
fn test_informe_promedia_y_reporta_peor_caso() {
    let mut perf = FramePerf::new();
    let timings = [
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 7.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            apu_ms: 1.0,
            lecturas: 50000,
            escrituras: 10000,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 7.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            apu_ms: 1.0,
            lecturas: 50000,
            escrituras: 10000,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 7.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            apu_ms: 1.0,
            lecturas: 50000,
            escrituras: 10000,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 30.0,
            espera_ms: 10.0,
            subida_ms: 1.0,
            codificacion_ms: 2.0,
            present_ms: 27.0,
            nativos: 1000,
            interprete: 10,
            ppu_ms: 3.0,
            apu_ms: 1.0,
            lecturas: 50000,
            escrituras: 10000,
        },
    ];

    for t in timings {
        perf.record_frame(t);
    }
    for _ in 0..7 {
        perf.record_empty_turn();
    }

    let report = perf.close_window((1200, 1920));
    assert_eq!(
        report.to_line(),
        "t=1s fps=4.0 ejec=20.0/20.0 ppu=3.0 apu=1.0 lecturas=50000 escrituras=10000 color=1.0/1.0 dibujo=15.0/30.0 espera=4.0/10.0 subida=1.0 codificacion=2.0 present=12.0 nativos=1000 interprete=10 vacias=7 ventana=1200x1920"
    );
}

#[test]
fn test_report_breaks_down_draw_time() {
    let mut perf = FramePerf::new();
    perf.record_frame(FrameTiming {
        ejec_ms: 12.0,
        color_ms: 1.0,
        dibujo_ms: 55.0,
        espera_ms: 0.6,
        subida_ms: 3.0,
        codificacion_ms: 4.0,
        present_ms: 48.0,
        nativos: 10,
        interprete: 20,
        ppu_ms: 2.0,
        apu_ms: 0.5,
        lecturas: 1000,
        escrituras: 500,
    });

    let report = perf.close_window((1200, 2000));
    let line = report.to_line();

    assert!(line.contains("subida=3.0"), "la línea debe reportar la subida: {line}");
    assert!(line.contains("codificacion=4.0"), "la línea debe reportar la codificación: {line}");
    assert!(line.contains("present=48.0"), "la línea debe reportar la presentación: {line}");
    assert!(line.contains("ppu=2.0"), "la línea debe reportar la PPU: {line}");
    assert!(line.contains("apu=0.5"), "la línea debe reportar el APU: {line}");
    assert!(line.contains("lecturas=1000"), "la línea debe reportar las lecturas: {line}");
    assert!(line.contains("escrituras=500"), "la línea debe reportar las escrituras: {line}");
    assert!(
        report.dibujo_avg_ms >= report.present_avg_ms,
        "el total de dibujo debe cubrir la fase de presentación"
    );
    assert_eq!(
        report.subida_avg_ms + report.codificacion_avg_ms + report.present_avg_ms,
        report.dibujo_avg_ms,
        "las tres fases deben sumar el total de dibujo"
    );
}

#[test]
fn test_ventana_sin_fotogramas_no_divide_por_cero() {
    let mut perf = FramePerf::new();
    for _ in 0..1000 {
        perf.record_empty_turn();
    }

    let report = perf.close_window((1200, 1920));
    assert_eq!(
        report.to_line(),
        "t=1s fps=0.0 ejec=0.0/0.0 ppu=0.0 apu=0.0 lecturas=0 escrituras=0 color=0.0/0.0 dibujo=0.0/0.0 espera=0.0/0.0 subida=0.0 codificacion=0.0 present=0.0 nativos=0 interprete=0 vacias=1000 ventana=1200x1920"
    );
}

#[test]
fn test_medicion_costo_reloj_retorna_valor_positivo() {
    let cost = measure_clock_cost_ns();
    assert!(cost > 0, "el costo de lectura de reloj debe ser >= 1 ns");
}

#[test]
fn test_resolve_perf_candidates_prioritizes_downloads() {
    let extra = vec![
        std::path::PathBuf::from("/storage/emulated/0/Android/data/com.blazingrecomp.game/files"),
        std::path::PathBuf::from("/sdcard/Download"),
    ];
    let candidates = resolve_perf_candidates(&extra);
    assert_eq!(candidates[0], std::path::PathBuf::from("/sdcard/Download"));
    assert_eq!(candidates[1], std::path::PathBuf::from("/storage/emulated/0/Download"));
    assert_eq!(
        candidates[2],
        std::path::PathBuf::from("/storage/emulated/0/Android/data/com.blazingrecomp.game/files")
    );
    // Verificar que no hay duplicados de /sdcard/Download
    assert_eq!(candidates.iter().filter(|p| **p == std::path::PathBuf::from("/sdcard/Download")).count(), 1);
}

