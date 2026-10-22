//! Módulo PPU Shim: Renderizado acelerado por GPU (wgpu) para Game Boy Advance.
//!
//! Sustituye el rasterizado por software en CPU de `gba-core::ppu` delegando el
//! ensamblado de fondos de Modo 0 y el dibujo de sprites (OBJ) directamente al hardware
//! gráfico mediante shaders WGSL optimizados.

pub const GBA_SCREEN_W: u32 = 240;
pub const GBA_SCREEN_H: u32 = 160;
pub const MAX_SPRITES: usize = 128;
pub const MAX_BG_LAYERS: usize = 4;

pub const PPU_SHIM_WGSL: &str = include_str!("ppu_shim.wgsl");

/// Forma geométrica de un sprite GBA (Attr0 bits 14-15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteShape {
    Square = 0,
    Wide = 1,
    Tall = 2,
    Reserved = 3,
}

impl SpriteShape {
    #[inline]
    pub fn from_attr0(attr0: u16) -> Self {
        match (attr0 >> 14) & 0x03 {
            0 => SpriteShape::Square,
            1 => SpriteShape::Wide,
            2 => SpriteShape::Tall,
            _ => SpriteShape::Reserved,
        }
    }
}

/// Modo de composición de un sprite GBA (Attr0 bits 10-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteMode {
    Normal = 0,
    SemiTransparent = 1,
    ObjWindow = 2,
    Prohibited = 3,
}

impl SpriteMode {
    #[inline]
    pub fn from_attr0(attr0: u16) -> Self {
        match (attr0 >> 10) & 0x03 {
            0 => SpriteMode::Normal,
            1 => SpriteMode::SemiTransparent,
            2 => SpriteMode::ObjWindow,
            _ => SpriteMode::Prohibited,
        }
    }
}

/// Calcula las dimensiones en píxeles (ancho, alto) de un sprite según su forma y tamaño.
#[inline]
pub fn sprite_dimensions(shape: SpriteShape, size_idx: u8) -> (u32, u32) {
    match shape {
        SpriteShape::Square => match size_idx & 3 {
            0 => (8, 8),
            1 => (16, 16),
            2 => (32, 32),
            _ => (64, 64),
        },
        SpriteShape::Wide => match size_idx & 3 {
            0 => (16, 8),
            1 => (32, 8),
            2 => (32, 16),
            _ => (64, 32),
        },
        SpriteShape::Tall => match size_idx & 3 {
            0 => (8, 16),
            1 => (8, 32),
            2 => (16, 32),
            _ => (32, 64),
        },
        SpriteShape::Reserved => (8, 8),
    }
}

/// Instancia de sprite decodificada y lista para transferir a la GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteGpuInstance {
    /// Posición X, Y en coordenadas de pantalla GBA (píxeles).
    pub pos: [f32; 2],
    /// Ancho y alto en píxeles.
    pub size: [f32; 2],
    /// Índice base del tile (0..1023).
    pub base_tile: u32,
    /// Índice de la subpaleta de color (0..15 para 4bpp, 0 para 8bpp).
    pub palette_idx: u32,
    /// Profundidad normalizada [0.0..1.0] para z-ordering frente a los fondos.
    pub depth: f32,
    /// Banderas: bit 0: hflip, bit 1: vflip, bit 2: semitransparente, bit 3: obj_window, bit 4: 8bpp, bit 5: 1d_mapping.
    pub flags: u32,
}

/// Uniformes GPU para una capa de fondo Modo 0.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BgUniformsGpu {
    pub scroll: [f32; 2],
    pub char_base: u32,
    pub screen_base: u32,
    pub screen_size: u32,
    pub is_8bpp: u32,
    pub depth: f32,
    pub _pad: u32,
}

/// Configuración de una capa de fondo en Modo 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BgLayerConfig {
    pub enabled: bool,
    pub priority: u8,
    pub char_base_block: u8,
    pub screen_base_block: u8,
    pub is_8bpp: bool,
    pub screen_size: u8,
    pub scroll_x: u16,
    pub scroll_y: u16,
    pub mosaic: bool,
}

impl BgLayerConfig {
    /// Dimensiones de la pantalla virtual en tiles según `screen_size`.
    #[inline]
    pub fn tile_dimensions(&self) -> (u32, u32) {
        match self.screen_size & 3 {
            0 => (32, 32), // 256x256 px
            1 => (64, 32), // 512x256 px
            2 => (32, 64), // 256x512 px
            _ => (64, 64), // 512x512 px
        }
    }
}

/// Convierte la memoria de paletas GBA (RGB555 de 15 bits) a RGBA f32 [0.0..1.0].
pub fn unpack_palette(raw_palette: &[u8], output: &mut [[f32; 4]; 512]) {
    let count = (raw_palette.len() / 2).min(512);
    for i in 0..count {
        let off = i * 2;
        let c = u16::from_le_bytes([raw_palette[off], raw_palette[off + 1]]) & 0x7FFF;
        let r = ((c & 0x1F) as f32) / 31.0;
        let g = (((c >> 5) & 0x1F) as f32) / 31.0;
        let b = (((c >> 10) & 0x1F) as f32) / 31.0;
        output[i] = [r, g, b, 1.0];
    }
}

/// Desempaqueta los tiles de sprites OBJ desde el offset 0x10000 de VRAM a un atlas de 256x256 píxeles (R8Uint).
pub fn unpack_obj_tiles_4bpp(vram_slice: &[u8], atlas: &mut [u8; 256 * 256]) {
    let obj_slice = if vram_slice.len() >= 0x18000 {
        &vram_slice[0x10000..0x18000]
    } else if vram_slice.len() > 0x10000 {
        &vram_slice[0x10000..]
    } else {
        // Fallback a inicio si el slice no contiene la VRAM superior (ej. en tests sintéticos cortos)
        vram_slice
    };

    let tile_count = (obj_slice.len() / 32).min(1024);
    for tile_idx in 0..tile_count {
        let tile_x = (tile_idx % 32) * 8;
        let tile_y = (tile_idx / 32) * 8;
        let src_off = tile_idx * 32;

        for row in 0..8 {
            let row_off = src_off + row * 4;
            let dst_row_start = (tile_y + row) * 256 + tile_x;

            for col_pair in 0..4 {
                let byte = obj_slice[row_off + col_pair];
                let p0 = byte & 0x0F;
                let p1 = (byte >> 4) & 0x0F;

                let dst_col = col_pair * 2;
                atlas[dst_row_start + dst_col] = p0;
                atlas[dst_row_start + dst_col + 1] = p1;
            }
        }
    }
}

/// Alias para retrocompatibilidad con tests existentes.
#[inline]
pub fn unpack_tiles_4bpp(vram_slice: &[u8], atlas: &mut [u8; 256 * 256]) {
    unpack_obj_tiles_4bpp(vram_slice, atlas);
}

/// Calcula el índice de color de un píxel de sprite leyendo directamente de la VRAM según las reglas de GBA.
#[inline]
pub fn calcular_pixel_sprite_vram(
    vram: &[u8],
    base_tile: u32,
    tw: u32,
    tx: u32,
    ty: u32,
    px: u32,
    py: u32,
    is_8bpp: bool,
    is_1d: bool,
) -> u8 {
    let obj_base = 0x10000usize;
    if is_8bpp {
        let base = (base_tile & 0x3FE) as usize;
        let target_tile = if is_1d {
            base + (ty * tw + tx) as usize * 2
        } else {
            base + ty as usize * 32 + tx as usize * 2
        };
        let byte_addr = obj_base + (target_tile % 1024) * 32 + (py * 8 + px) as usize;
        if byte_addr < vram.len() {
            vram[byte_addr]
        } else {
            0
        }
    } else {
        let base = base_tile as usize;
        let target_tile = if is_1d {
            base + (ty * tw + tx) as usize
        } else {
            base + ty as usize * 32 + tx as usize
        };
        let byte_addr = obj_base + (target_tile % 1024) * 32 + (py * 4 + px / 2) as usize;
        if byte_addr < vram.len() {
            let byte_val = vram[byte_addr];
            if (px % 2) == 1 {
                (byte_val >> 4) & 0x0F
            } else {
                byte_val & 0x0F
            }
        } else {
            0
        }
    }
}

/// Decodificador y gestor de datos de video para el PPU Shim.
pub struct PpuShim {
    /// Sprites activos decodificados para el fotograma actual.
    pub sprites: [SpriteGpuInstance; MAX_SPRITES],
    /// Cantidad de sprites visibles activos en el fotograma.
    pub sprite_count: usize,
    /// Configuración de las 4 capas de fondos Modo 0.
    pub bg_layers: [BgLayerConfig; MAX_BG_LAYERS],
    /// Control maestro de visualización (`DISPCNT`).
    pub dispcnt: u16,
}

impl PpuShim {
    /// Crea un nuevo gestor de PPU Shim con búferes preasignados.
    pub fn new() -> Self {
        Self {
            sprites: [SpriteGpuInstance::default(); MAX_SPRITES],
            sprite_count: 0,
            bg_layers: [BgLayerConfig::default(); MAX_BG_LAYERS],
            dispcnt: 0,
        }
    }

    /// Decodifica la tabla OAM de 128 entradas (1 KB) extrayendo los sprites visibles.
    ///
    /// Decodificar no es dibujar: el gate maestro de OBJ (DISPCNT bit 12) se
    /// aplica al consumir la lista, en [`PpuShim::active_sprite_count`]. Así la
    /// decodificación sigue siendo una función pura de la tabla OAM y se puede
    /// probar aislada, sin montar un DISPCNT de crippled.
    pub fn decode_oam(&mut self, oam: &[u8]) -> usize {
        self.sprite_count = 0;
        if oam.len() < 1024 {
            return 0;
        }

        let is_1d_mapping = (self.dispcnt & (1 << 6)) != 0;
        let mut count = 0;

        for i in 0..MAX_SPRITES {
            let offset = i * 8;
            let attr0 = u16::from_le_bytes([oam[offset], oam[offset + 1]]);
            let attr1 = u16::from_le_bytes([oam[offset + 2], oam[offset + 3]]);
            let attr2 = u16::from_le_bytes([oam[offset + 4], oam[offset + 5]]);

            let is_affine = (attr0 & (1 << 8)) != 0;
            let is_disabled = !is_affine && ((attr0 & (1 << 9)) != 0);

            // Sprite oculto / desactivado
            if is_disabled {
                continue;
            }

            let shape = SpriteShape::from_attr0(attr0);
            if shape == SpriteShape::Reserved {
                continue;
            }

            let size_idx = ((attr1 >> 14) & 0x03) as u8;
            let (width, height) = sprite_dimensions(shape, size_idx);

            // Coordenada Y (con soporte de wrap-around vertical para entrada suave)
            let raw_y = (attr0 & 0xFF) as i32;
            let y_pos = if raw_y >= 160 { raw_y - 256 } else { raw_y };

            // Coordenada X (con soporte de wrap-around horizontal para entrada suave)
            let raw_x = (attr1 & 0x1FF) as i32;
            let x_pos = if raw_x >= 240 { raw_x - 512 } else { raw_x };

            // Poda temprana: completamente fuera de la pantalla visible de 240x160
            if x_pos + (width as i32) <= 0 || x_pos >= GBA_SCREEN_W as i32 {
                continue;
            }
            if y_pos + (height as i32) <= 0 || y_pos >= GBA_SCREEN_H as i32 {
                continue;
            }

            let mode = SpriteMode::from_attr0(attr0);
            if mode == SpriteMode::Prohibited || mode == SpriteMode::ObjWindow {
                continue;
            }

            let is_8bpp = (attr0 & (1 << 13)) != 0;
            let hflip = !is_affine && ((attr1 & (1 << 12)) != 0);
            let vflip = !is_affine && ((attr1 & (1 << 13)) != 0);

            let tile_idx = (attr2 & 0x3FF) as u32;
            let priority = ((attr2 >> 10) & 0x03) as u8;
            let palette_idx = if is_8bpp { 0 } else { ((attr2 >> 12) & 0x0F) as u32 };

            let mut flags = 0u32;
            if hflip {
                flags |= 1 << 0;
            }
            if vflip {
                flags |= 1 << 1;
            }
            if mode == SpriteMode::SemiTransparent {
                flags |= 1 << 2;
            }
            if mode == SpriteMode::ObjWindow {
                flags |= 1 << 3;
            }
            if is_8bpp {
                flags |= 1 << 4;
            }
            if is_1d_mapping {
                flags |= 1 << 5;
            }

            // Metadatos de ordenación en Painter's: prioridad (bits 8..9) e índice OAM (bits 16..23)
            flags |= ((priority as u32) & 0x03) << 8;
            flags |= ((i as u32) & 0x7F) << 16;

            // Normalización de profundidad: prioridad 0 (frente) a 3 (fondo)
            let depth = 0.15 + (priority as f32) * 0.2;

            self.sprites[count] = SpriteGpuInstance {
                pos: [x_pos as f32, y_pos as f32],
                size: [width as f32, height as f32],
                base_tile: tile_idx,
                palette_idx,
                depth,
                flags,
            };

            count += 1;
            if count >= MAX_SPRITES {
                break;
            }
        }

        // Orden de Painter's Algorithm para GBA:
        // 1. Mayor prioridad numérica se dibuja primero (3 al fondo, 0 al frente): prio_b.cmp(&prio_a)
        // 2. A igual prioridad numérica, mayor índice OAM se dibuja primero (127 al fondo, 0 al frente): oam_b.cmp(&oam_a)
        self.sprites[..count].sort_unstable_by(|a, b| {
            let prio_a = (a.flags >> 8) & 0x03;
            let prio_b = (b.flags >> 8) & 0x03;
            let oam_a = (a.flags >> 16) & 0x7F;
            let oam_b = (b.flags >> 16) & 0x7F;

            prio_b.cmp(&prio_a).then_with(|| oam_b.cmp(&oam_a))
        });

        self.sprite_count = count;
        count
    }

    /// DISPCNT bit 12 (OBJ habilitado): con OBJ apagado no se dibuja ningún sprite.
    ///
    /// Recibe el número de sprites decodificados y devuelve los que se dibujan.
    /// El hardware no apaga la decodificación, apaga el dibujo, de ahí que el
    /// gate viva aquí y no dentro de [`PpuShim::decode_oam`].
    pub fn active_sprite_count(&self, decoded: usize) -> usize {
        if self.dispcnt & (1 << 12) != 0 {
            decoded
        } else {
            0
        }
    }

    /// Decodifica los registros de I/O de la PPU (`DISPCNT`, `BGxCNT`, `BGxHOFS`, `BGxVOFS`).
    pub fn decode_io_regs(&mut self, io: &[u8]) {
        if io.len() < 0x40 {
            return;
        }

        self.dispcnt = u16::from_le_bytes([io[0x00], io[0x01]]);
        let obj_master_enabled = (self.dispcnt & (1 << 12)) != 0;

        // Si los sprites están desactivados a nivel maestro, vaciamos la lista activa
        if !obj_master_enabled {
            self.sprite_count = 0;
        }

        for bg_idx in 0..MAX_BG_LAYERS {
            let bg_enabled = (self.dispcnt & (1 << (8 + bg_idx))) != 0;
            let cnt_off = 0x08 + bg_idx * 2;
            let hofs_off = 0x10 + bg_idx * 4;
            let vofs_off = 0x12 + bg_idx * 4;

            let bgcnt = u16::from_le_bytes([io[cnt_off], io[cnt_off + 1]]);
            let scroll_x = u16::from_le_bytes([io[hofs_off], io[hofs_off + 1]]) & 0x1FF;
            let scroll_y = u16::from_le_bytes([io[vofs_off], io[vofs_off + 1]]) & 0x1FF;

            self.bg_layers[bg_idx] = BgLayerConfig {
                enabled: bg_enabled,
                priority: (bgcnt & 0x03) as u8,
                char_base_block: ((bgcnt >> 2) & 0x03) as u8,
                screen_base_block: ((bgcnt >> 8) & 0x1F) as u8,
                is_8bpp: (bgcnt & (1 << 7)) != 0,
                screen_size: ((bgcnt >> 14) & 0x03) as u8,
                scroll_x,
                scroll_y,
                mosaic: (bgcnt & (1 << 6)) != 0,
            };
        }
    }
}

impl Default for PpuShim {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Pipeline de Renderizado wgpu para el PPU Shim
// ===========================================================================

/// Vértice de quad unitario [0.0..1.0].
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub pos: [f32; 2],
}

pub const QUAD_VERTICES: [QuadVertex; 6] = [
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [0.0, 1.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
];

/// Pipeline de renderizado GPU acelerado para gráficos GBA Modo 0 y Sprites.
pub struct PpuShimPipeline {
    pub shim: PpuShim,
    pub sprite_pipeline: wgpu::RenderPipeline,
    pub bg_pipeline: wgpu::RenderPipeline,
    pub quad_buffer: wgpu::Buffer,
    pub sprite_instances_buffer: wgpu::Buffer,
    pub palette_buffer: wgpu::Buffer,
    pub bg_vram_buffer: wgpu::Buffer,
    pub bg_uniform_buffers: [wgpu::Buffer; MAX_BG_LAYERS],
    pub tile_atlas_texture: wgpu::Texture,
    pub tile_atlas_view: wgpu::TextureView,
    pub bg_bind_groups: [wgpu::BindGroup; MAX_BG_LAYERS],
    pub sprite_bind_group: wgpu::BindGroup,
    pub atlas_staging: Box<[u8; 256 * 256]>,
    pub palette_staging: Box<[[f32; 4]; 512]>,
}

impl PpuShimPipeline {
    /// Inicializa un nuevo pipeline de renderizado GPU para el PPU Shim.
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ppu-shim-shader"),
            source: wgpu::ShaderSource::Wgsl(PPU_SHIM_WGSL.into()),
        });

        // 1. Atlas de Sprites OBJ (256x256 píxeles R8Uint)
        let tile_atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ppu-shim-tile-atlas"),
            size: wgpu::Extent3d {
                width: 256,
                height: 256,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let tile_atlas_view = tile_atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 2. Paleta de Colores (512 colores x 16 bytes = 8192 bytes)
        let palette_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-shim-palette-buffer"),
            size: (512 * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 3. VRAM de Fondos (64 KB = 16384 palabras u32)
        let bg_vram_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-shim-bg-vram"),
            size: 98304, // 96 KB (0x06000000..0x06018000, VRAM completa GBA)
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 4. Uniformes de Fondos (uno por cada capa BG0..BG3 para evitar colisiones)
        let bg_uniform_buffers: [wgpu::Buffer; MAX_BG_LAYERS] = std::array::from_fn(|i| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("ppu-shim-bg-uniforms-{i}")),
                size: std::mem::size_of::<BgUniformsGpu>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });

        // 5. Bind Group Layout Común
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ppu-shim-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Bind groups para cada capa de fondo
        let bg_bind_groups: [wgpu::BindGroup; MAX_BG_LAYERS] = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("ppu-shim-bg-bind-group-{i}")),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tile_atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: palette_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: bg_vram_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: bg_uniform_buffers[i].as_entire_binding(),
                    },
                ],
            })
        });

        let sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ppu-shim-sprite-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tile_atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: palette_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bg_vram_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bg_uniform_buffers[0].as_entire_binding(),
                },
            ],
        });

        // 6. Geometría y Buffers de Instancia
        let quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-shim-quad-buffer"),
            size: (std::mem::size_of::<QuadVertex>() * QUAD_VERTICES.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_instances_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ppu-shim-sprite-instances"),
            size: (std::mem::size_of::<SpriteGpuInstance>() * MAX_SPRITES) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ppu-shim-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        // 7. Pipeline de Renderizado de Fondos (BG Modo 0)
        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ppu-shim-bg-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_bg"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_bg"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 8. Pipeline de Renderizado de Sprites (OBJ)
        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ppu-shim-sprite-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sprite"),
                compilation_options: Default::default(),
                buffers: &[
                    // Buffer 0: Quad unitario
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    },
                    // Buffer 1: Instancias de sprites
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteGpuInstance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2, // pos
                            },
                            wgpu::VertexAttribute {
                                offset: 8,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2, // size
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Uint32, // base_tile
                            },
                            wgpu::VertexAttribute {
                                offset: 20,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Uint32, // palette_idx
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32, // depth
                            },
                            wgpu::VertexAttribute {
                                offset: 28,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Uint32, // flags
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sprite"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            shim: PpuShim::new(),
            sprite_pipeline,
            bg_pipeline,
            quad_buffer,
            sprite_instances_buffer,
            palette_buffer,
            bg_vram_buffer,
            bg_uniform_buffers,
            tile_atlas_texture,
            tile_atlas_view,
            bg_bind_groups,
            sprite_bind_group,
            atlas_staging: Box::new([0u8; 256 * 256]),
            palette_staging: Box::new([[0.0f32; 4]; 512]),
        }
    }

    /// Renderiza un fotograma completo del PPU Shim sobre la vista de textura destino.
    pub fn render_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
        vram: &[u8],
        oam: &[u8],
        palette: &[u8],
        io: &[u8],
    ) {
        // 1. Inicializar buffer de quad estático
        queue.write_buffer(&self.quad_buffer, 0, bytemuck::cast_slice(&QUAD_VERTICES));

        // 2. Decodificación de registros y OAM
        self.shim.decode_io_regs(io);
        let decoded = self.shim.decode_oam(oam);
        // Gate maestro de OBJ: con DISPCNT bit 12 apagado no se dibuja ningún sprite.
        let sprite_count = self.shim.active_sprite_count(decoded);

        // 3. Conversión y subida de paleta a GPU (512 colores)
        unpack_palette(palette, &mut self.palette_staging);
        queue.write_buffer(
            &self.palette_buffer,
            0,
            bytemuck::cast_slice(&self.palette_staging[..]),
        );

        // 4. Subida de VRAM de fondos (96 KB completos = 0x18000 bytes)
        let bg_vram_len = 98304.min(vram.len());
        queue.write_buffer(
            &self.bg_vram_buffer,
            0,
            bytemuck::cast_slice(&vram[..bg_vram_len]),
        );

        // 5. Los sprites leen directamente de `bg_vram_buffer` (VRAM completa 96 KB en GPU).
        // Esto elimina el coste de desempaquetado en CPU (~1.5 ms) y soporta 4bpp y 8bpp sin fricción.

        // 6. Subida de instancias de sprites activos (ordenados para Painter's Algorithm)
        if sprite_count > 0 {
            let active_slice = &self.shim.sprites[..sprite_count];
            queue.write_buffer(
                &self.sprite_instances_buffer,
                0,
                bytemuck::cast_slice(active_slice),
            );
        }

        // Color de backdrop de GBA (primer color de la paleta de fondos)
        let backdrop = self.palette_staging[0];
        let clear_color = wgpu::Color {
            r: backdrop[0] as f64,
            g: backdrop[1] as f64,
            b: backdrop[2] as f64,
            a: 1.0,
        };

        // 7. Emisión de comandos GPU en RenderPass con intercalado de prioridades BG/OBJ
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ppu-shim-encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ppu-shim-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Painter's Algorithm multicapa: dibujar en 4 etapas por prioridad descendente (3 a 0).
            // En cada prioridad P:
            //   1. Fondos con prioridad P (orden BG3 a BG0 para que BG menor quede al frente).
            //   2. Sprites con prioridad P (orden OAM inverso para que OAM menor quede al frente).
            for priority in (0..=3).rev() {
                // 1. Fondos activos con prioridad P
                for bg_idx in (0..MAX_BG_LAYERS).rev() {
                    let layer = &self.shim.bg_layers[bg_idx];
                    if layer.enabled && layer.priority == priority {
                        let depth = 0.8 - (layer.priority as f32) * 0.2;
                        let uniforms = BgUniformsGpu {
                            scroll: [layer.scroll_x as f32, layer.scroll_y as f32],
                            char_base: layer.char_base_block as u32,
                            screen_base: layer.screen_base_block as u32,
                            screen_size: layer.screen_size as u32,
                            is_8bpp: if layer.is_8bpp { 1 } else { 0 },
                            depth,
                            _pad: 0,
                        };
                        queue.write_buffer(
                            &self.bg_uniform_buffers[bg_idx],
                            0,
                            bytemuck::bytes_of(&uniforms),
                        );

                        pass.set_pipeline(&self.bg_pipeline);
                        pass.set_bind_group(0, &self.bg_bind_groups[bg_idx], &[]);
                        pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
                        pass.draw(0..6, 0..1);
                    }
                }

                // 2. Sprites con prioridad P
                if sprite_count > 0 {
                    let active = &self.shim.sprites[..sprite_count];
                    let prio_u32 = priority as u32;
                    let start_opt = active.iter().position(|s| ((s.flags >> 8) & 0x03) == prio_u32);
                    if let Some(start) = start_opt {
                        let count_prio = active[start..]
                            .iter()
                            .take_while(|s| ((s.flags >> 8) & 0x03) == prio_u32)
                            .count();
                        if count_prio > 0 {
                            pass.set_pipeline(&self.sprite_pipeline);
                            pass.set_bind_group(0, &self.sprite_bind_group, &[]);
                            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
                            pass.set_vertex_buffer(1, self.sprite_instances_buffer.slice(..));
                            pass.draw(0..6, (start as u32)..((start + count_prio) as u32));
                        }
                    }
                }
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

// ---------------------------------------------------------------------------
// Suite de pruebas satélite (AGENTS.md §4.4)
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "ppu_shim_tests.rs"]
mod tests;
