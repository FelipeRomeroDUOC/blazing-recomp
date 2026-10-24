//! Prototipo de docs/propuestas/2026-10-16_arnes-comparacion-shim-vs-software.md.
//! No es parte del producto. Ejecutar con:
//!   cargo run --release --features proto --example ppu_proto -- <ruta.gba> [cuadros]
//! Comprueba: (1) wgpu sin ventana, (2) determinismo de gba-core, (3) tiempo por cuadro.

use blazing_recomp::ppu_shim::PpuShimPipeline;
use std::hash::{Hash, Hasher};
use std::time::Instant;

#[path = "comun/mod.rs"]
mod comun;

const W: u32 = 240;
const H: u32 = 160;

fn hash_fb(fb: &[u16]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    fb.hash(&mut h);
    h.finish()
}

fn corre(rom: &[u8], cuadros: u32) -> (Vec<u64>, f64) {
    let mut m = gba_core::Machine::new(rom.to_vec());
    let mut hashes = Vec::with_capacity(cuadros as usize);
    let t0 = Instant::now();
    for _ in 0..cuadros {
        m.run_frame(5_000_000);
        hashes.push(hash_fb(&m.bus.framebuffer));
    }
    let ms_por_cuadro = t0.elapsed().as_secs_f64() * 1000.0 / cuadros as f64;
    (hashes, ms_por_cuadro)
}

fn prueba_wgpu(rom: &[u8], cuadros: u32) {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(a) => a,
        Err(e) => {
            println!("wgpu: FALLA, sin adaptador: {e:?}");
            return;
        }
    };
    let info = adapter.get_info();
    println!("wgpu: adaptador = {:?} ({:?})", info.name, info.backend);

    let desc = wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        ..Default::default()
    };
    let (device, queue) = match pollster::block_on(adapter.request_device(&desc)) {
        Ok(dq) => dq,
        Err(e) => {
            println!("wgpu: FALLA, sin dispositivo: {e:?}");
            return;
        }
    };

    let formato = wgpu::TextureFormat::Rgba8Unorm;
    let objetivo = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("proto-objetivo"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: formato,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let vista = objetivo.create_view(&wgpu::TextureViewDescriptor::default());
    let mut shim = PpuShimPipeline::new(&device, formato);

    let mut m = gba_core::Machine::new(rom.to_vec());
    for _ in 0..cuadros {
        m.run_frame(5_000_000);
    }
    shim.render_frame(&device, &queue, &vista, &m.bus.vram, &m.bus.oam, &m.bus.palette, &m.bus.io);

    let fila = ((W * 4 + 255) / 256) * 256;
    let lectura = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("proto-lectura"),
        size: (fila * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut codificador = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    codificador.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &objetivo,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &lectura,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(fila),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    queue.submit(Some(codificador.finish()));

    let slice = lectura.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = device.poll(wgpu::PollType::Wait);
    rx.recv().expect("canal de mapeo").expect("mapeo del búfer");

    let datos = slice.get_mapped_range();
    let mut con_color: u32 = 0;
    let mut suma: u64 = 0;
    for y in 0..H {
        let ini = (y * fila) as usize;
        for x in 0..W {
            let o = ini + (x * 4) as usize;
            let (r, g, b) = (datos[o], datos[o + 1], datos[o + 2]);
            if r | g | b != 0 {
                con_color += 1;
            }
            suma += r as u64 + g as u64 + b as u64;
        }
    }
    drop(datos);
    lectura.unmap();

    let veredicto = if con_color > 0 { "OK: el shim dibujó" } else { "FALLA: todo negro" };
    println!("wgpu: {veredicto} ({con_color} de {} píxeles con color, suma={suma})", W * H);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(ruta) = args.get(1) else {
        eprintln!("uso: ppu_proto <ruta.gba> [cuadros]");
        std::process::exit(2);
    };
    let cuadros: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(600);
    let rom = match std::fs::read(ruta) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ROM no encontrado: {ruta} ({e})");
            std::process::exit(1);
        }
    };

    let (h1, ms1) = corre(&rom, cuadros);
    let (h2, ms2) = corre(&rom, cuadros);
    let iguales = h1.iter().zip(&h2).filter(|(a, b)| a == b).count();
    let veredicto = if h1 == h2 { "OK" } else { "FALLA" };
    println!("determinismo: {veredicto} ({iguales} de {cuadros} cuadros con hash igual)");
    println!("gba-core: {ms1:.3} ms/cuadro (corrida 1), {ms2:.3} ms/cuadro (corrida 2)");
    println!("estimado para 10.800 cuadros de espera: {:.1} s", ms1 * 10_800.0 / 1000.0);

    prueba_wgpu(&rom, cuadros);
}
