use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;
use wgpu::util::DeviceExt;

const ATLAS_CONFIG_PATH: &str = "assets/atlas.json";
const DEFAULT_TEXTURE_PATH: &str = "assets/sprites.png";
const DEFAULT_ATLAS_COLUMNS: u32 = 2;
const DEFAULT_ATLAS_ROWS: u32 = 2;
const DEFAULT_ATLAS_TILE_SIZE: u32 = 32;

const ATLAS_COLORS: [[u8; 4]; 6] = [
    [235, 70, 70, 255],
    [70, 200, 90, 255],
    [70, 120, 235, 255],
    [235, 210, 70, 255],
    [200, 90, 200, 255],
    [60, 180, 200, 255],
];

#[derive(Debug, Deserialize)]
#[serde(default)]
struct AtlasConfig {
    texture: Option<String>,
    columns: u32,
    rows: u32,
    tile_size: u32,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            texture: None,
            columns: DEFAULT_ATLAS_COLUMNS,
            rows: DEFAULT_ATLAS_ROWS,
            tile_size: DEFAULT_ATLAS_TILE_SIZE,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Atlas {
    pub columns: u32,
    pub rows: u32,
    pub tile_size: u32,
}

impl Atlas {
    fn from_config(config: &AtlasConfig) -> Self {
        Self {
            columns: config.columns.max(1),
            rows: config.rows.max(1),
            tile_size: config.tile_size.max(1),
        }
    }

    pub fn tile_count(&self) -> u32 {
        self.columns * self.rows
    }

    pub fn uv_for_index(&self, index: u32) -> (glam::Vec2, glam::Vec2) {
        let count = self.tile_count().max(1);
        let idx = index % count;
        let tile_x = idx % self.columns;
        let tile_y = idx / self.columns;
        let tile_w = 1.0 / self.columns as f32;
        let tile_h = 1.0 / self.rows as f32;
        let min = glam::Vec2::new(tile_x as f32 * tile_w, tile_y as f32 * tile_h);
        let max = glam::Vec2::new((tile_x + 1) as f32 * tile_w, (tile_y + 1) as f32 * tile_h);
        (min, max)
    }
}

pub struct Texture {
    _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        data: &[u8],
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::MipMajor,
            data,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sprite Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    fn from_path(device: &wgpu::Device, queue: &wgpu::Queue, path: &Path) -> Option<Self> {
        let image = image::open(path).ok()?;
        let rgba = image.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();
        let data = rgba.into_raw();
        Some(Self::from_rgba8(
            device,
            queue,
            width,
            height,
            &data,
            "Sprite Texture",
        ))
    }
}

pub struct Assets {
    pub atlas: Atlas,
    pub texture: Texture,
    config_path: PathBuf,
    texture_path: PathBuf,
    config_mtime: Option<SystemTime>,
    texture_mtime: Option<SystemTime>,
}

impl Assets {
    pub fn load(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let config_path = PathBuf::from(ATLAS_CONFIG_PATH);
        let (config, config_mtime) = load_atlas_config(&config_path);
        let atlas = Atlas::from_config(&config);
        let texture_path = texture_path_from_config(&config);
        let texture_mtime = file_mtime(&texture_path);
        let texture = load_texture_or_procedural(device, queue, &atlas, &texture_path);
        Self {
            atlas,
            texture,
            config_path,
            texture_path,
            config_mtime,
            texture_mtime,
        }
    }

    pub fn reload_if_changed(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
        let mut reload_texture = false;
        let current_config_mtime = file_mtime(&self.config_path);
        if current_config_mtime != self.config_mtime {
            let (config, mtime) = load_atlas_config(&self.config_path);
            self.atlas = Atlas::from_config(&config);
            let new_texture_path = texture_path_from_config(&config);
            if new_texture_path != self.texture_path {
                self.texture_path = new_texture_path;
                reload_texture = true;
            }
            self.config_mtime = mtime;
        }

        let current_texture_mtime = file_mtime(&self.texture_path);
        if current_texture_mtime != self.texture_mtime {
            reload_texture = true;
        }

        if reload_texture {
            self.texture = load_texture_or_procedural(device, queue, &self.atlas, &self.texture_path);
            self.texture_mtime = file_mtime(&self.texture_path);
        }

        reload_texture
    }
}

fn texture_path_from_config(config: &AtlasConfig) -> PathBuf {
    config
        .texture
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TEXTURE_PATH))
}

fn load_atlas_config(path: &Path) -> (AtlasConfig, Option<SystemTime>) {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return (AtlasConfig::default(), file_mtime(path)),
    };

    match serde_json::from_str(&contents) {
        Ok(config) => (config, file_mtime(path)),
        Err(err) => {
            log::warn!("Failed to parse {}: {}", path.display(), err);
            (AtlasConfig::default(), file_mtime(path))
        }
    }
}

fn load_texture_or_procedural(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    atlas: &Atlas,
    texture_path: &Path,
) -> Texture {
    if let Some(texture) = Texture::from_path(device, queue, texture_path) {
        log::info!("Loaded texture from {}", texture_path.display());
        texture
    } else {
        log::warn!(
            "Falling back to procedural atlas texture (missing {})",
            texture_path.display()
        );
        create_procedural_atlas_texture(device, queue, atlas)
    }
}

fn create_procedural_atlas_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    atlas: &Atlas,
) -> Texture {
    let width = atlas.columns * atlas.tile_size;
    let height = atlas.rows * atlas.tile_size;
    let mut texels = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let tile_x = x / atlas.tile_size;
            let tile_y = y / atlas.tile_size;
            let tile_index = (tile_y * atlas.columns + tile_x) as usize;
            let color = ATLAS_COLORS[tile_index % ATLAS_COLORS.len()];
            let idx = ((y * width + x) * 4) as usize;
            texels[idx..idx + 4].copy_from_slice(&color);
        }
    }

    Texture::from_rgba8(device, queue, width, height, &texels, "Procedural Atlas")
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}
