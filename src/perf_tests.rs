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
            nativos: 1000,
            interprete: 10,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            nativos: 1000,
            interprete: 10,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 10.0,
            espera_ms: 2.0,
            nativos: 1000,
            interprete: 10,
        },
        FrameTiming {
            ejec_ms: 20.0,
            color_ms: 1.0,
            dibujo_ms: 30.0,
            espera_ms: 10.0,
            nativos: 1000,
            interprete: 10,
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
        "t=1s fps=4.0 ejec=20.0/20.0 color=1.0/1.0 dibujo=15.0/30.0 espera=4.0/10.0 nativos=1000 interprete=10 vacias=7 ventana=1200x1920"
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
        "t=1s fps=0.0 ejec=0.0/0.0 color=0.0/0.0 dibujo=0.0/0.0 espera=0.0/0.0 nativos=0 interprete=0 vacias=1000 ventana=1200x1920"
    );
}

#[test]
fn test_medicion_costo_reloj_retorna_valor_positivo() {
    let cost = measure_clock_cost_ns();
    assert!(cost > 0, "el costo de lectura de reloj debe ser >= 1 ns");
}
