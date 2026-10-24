//! Arnés de GPU de los experimentos: dibuja un cuadro con la PPU nativa y
//! lo devuelve como RGB de 8 bits sin relleno.

use blazing_recomp::ppu_nativa::PpuNativaPipeline;

pub const ANCHO: usize = 240;
pub const ALTO: usize = 160;
/// Bytes por fila en el búfer de lectura de la GPU (wgpu exige múltiplos de 256).
pub const FILA_GPU: u32 = (ANCHO as u32 * 4).div_ceil(256) * 256;

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: PpuNativaPipeline,
    objetivo: wgpu::Texture,
    vista: wgpu::TextureView,
    lectura: wgpu::Buffer,
}

impl Gpu {
    /// Arnés con la PPU nativa, que puede fallar al construir su pipeline.
    pub fn nueva_nativa() -> Result<Self, String> {
        let instancia = wgpu::Instance::default();
        let adaptador = pollster::block_on(instancia.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| format!("sin adaptador de GPU: {e:?}"))?;
        let desc = wgpu::DeviceDescriptor {
            required_limits: adaptador.limits(),
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adaptador.request_device(&desc))
            .map_err(|e| format!("sin dispositivo de GPU: {e:?}"))?;

        let formato = wgpu::TextureFormat::Rgba8Unorm;
        let objetivo = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("arnes-objetivo"),
            size: wgpu::Extent3d {
                width: ANCHO as u32,
                height: ALTO as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formato,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let vista = objetivo.create_view(&wgpu::TextureViewDescriptor::default());
        let pipeline = PpuNativaPipeline::new(&device, formato)?;
        let lectura = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("arnes-lectura"),
            size: (FILA_GPU * ALTO as u32) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            objetivo,
            vista,
            lectura,
        })
    }

    /// Dibuja el cuadro con el estado dado y devuelve RGB de 8 bits sin relleno.
    pub fn renderiza(&mut self, vram: &[u8], oam: &[u8], palette: &[u8], io: &[u8]) -> Vec<u8> {
        self.renderiza_con_lineas(vram, oam, palette, io, None, None)
    }

    /// Dibuja el cuadro permitiendo capturas de I/O y paleta por línea para la PPU nativa.
    pub fn renderiza_con_lineas(
        &mut self,
        vram: &[u8],
        oam: &[u8],
        palette: &[u8],
        io: &[u8],
        io_lineas: Option<&[[u8; 0x60]; 160]>,
        paleta_lineas: Option<&[[u8; 1024]; 160]>,
    ) -> Vec<u8> {
        self.pipeline
            .render_frame_lineas(&self.device, &self.queue, &self.vista, vram, oam, palette, io, io_lineas, paleta_lineas);

        let mut codificador = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        codificador.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.objetivo,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.lectura,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(FILA_GPU),
                    rows_per_image: Some(ALTO as u32),
                },
            },
            wgpu::Extent3d {
                width: ANCHO as u32,
                height: ALTO as u32,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(codificador.finish()));

        let slice = self.lectura.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::Wait);
        rx.recv().expect("canal de mapeo").expect("mapeo del búfer");

        let datos = slice.get_mapped_range();
        let mut rgb = Vec::with_capacity(ANCHO * ALTO * 3);
        for y in 0..ALTO {
            let ini = y * FILA_GPU as usize;
            for x in 0..ANCHO {
                let o = ini + x * 4;
                rgb.extend_from_slice(&datos[o..o + 3]);
            }
        }
        drop(datos);
        self.lectura.unmap();
        rgb
    }
}
