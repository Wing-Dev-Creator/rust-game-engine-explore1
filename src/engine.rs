use std::sync::Arc;

use glam::Vec2;
use winit::keyboard::KeyCode;
use winit::window::Window;

use crate::assets::Assets;
use crate::input::InputState;
use crate::renderer::{InstanceRaw, Renderer};
use crate::scene::{Animation, Body, Entity, Sprite, Transform, World};
use crate::time::Time;

const SPRITE_SIZE: f32 = 128.0;
const FIXED_DT: f32 = 1.0 / 60.0;
const WORLD_BOUNDS: Vec2 = Vec2::new(520.0, 320.0);

const PALETTE: [[f32; 4]; 6] = [
    [1.0, 1.0, 1.0, 1.0],
    [0.95, 0.75, 0.65, 1.0],
    [0.65, 0.9, 0.7, 1.0],
    [0.6, 0.7, 0.95, 1.0],
    [0.95, 0.85, 0.5, 1.0],
    [0.85, 0.7, 0.95, 1.0],
];

struct Camera {
    position: Vec2,
    zoom: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

pub struct Engine {
    renderer: Renderer,
    assets: Assets,
    world: World,
    input: InputState,
    time: Time,
    camera: Camera,
    player: Entity,
    instance_data: Vec<InstanceRaw>,
    paused: bool,
    player_color_index: usize,
    spawn_counter: u32,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Self {
        let mut renderer = Renderer::new(window).await;
        let assets = Assets::load(renderer.device(), renderer.queue());
        renderer.set_texture(&assets.texture);

        let mut world = World::new();
        let player = world.spawn_sprite(
            Transform::new(Vec2::ZERO),
            Sprite {
                size: Vec2::splat(SPRITE_SIZE),
                tile_index: 0,
                color: PALETTE[0],
                spin: 0.0,
                animation: None,
            },
        );

        let child = world.spawn_sprite(
            Transform::new(Vec2::new(0.0, SPRITE_SIZE * 0.7)),
            Sprite {
                size: Vec2::splat(SPRITE_SIZE * 0.35),
                tile_index: 1,
                color: PALETTE[5],
                spin: 1.2,
                animation: None,
            },
        );
        world.set_parent(child, player);

        world.spawn_sprite_with_body(
            Transform {
                position: Vec2::new(220.0, -80.0),
                rotation: 0.0,
                scale: Vec2::ONE,
            },
            Sprite {
                size: Vec2::splat(SPRITE_SIZE * 0.75),
                tile_index: 1,
                color: PALETTE[2],
                spin: 0.6,
                animation: Some(Animation::new(vec![0, 1, 2, 3], 6.0)),
            },
            Some(Body::new(Vec2::new(80.0, 140.0))),
        );

        world.spawn_sprite_with_body(
            Transform {
                position: Vec2::new(-240.0, 140.0),
                rotation: 0.0,
                scale: Vec2::ONE,
            },
            Sprite {
                size: Vec2::splat(SPRITE_SIZE * 0.9),
                tile_index: 2,
                color: PALETTE[3],
                spin: -0.4,
                animation: None,
            },
            Some(Body::new(Vec2::new(-120.0, 60.0))),
        );

        world.spawn_sprite_with_body(
            Transform {
                position: Vec2::new(-100.0, -200.0),
                rotation: 0.0,
                scale: Vec2::ONE,
            },
            Sprite {
                size: Vec2::splat(SPRITE_SIZE * 0.6),
                tile_index: 3,
                color: PALETTE[4],
                spin: 0.2,
                animation: None,
            },
            Some(Body::new(Vec2::new(140.0, -90.0))),
        );

        let camera = Camera::new();
        renderer.update_camera(camera.position, camera.zoom);

        Self {
            renderer,
            assets,
            world,
            input: InputState::new(),
            time: Time::new(FIXED_DT),
            camera,
            player,
            instance_data: Vec::new(),
            paused: false,
            player_color_index: 0,
            spawn_counter: 4,
        }
    }

    pub fn window(&self) -> &Window {
        self.renderer.window()
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.renderer.resize(new_size);
        self.renderer.update_camera(self.camera.position, self.camera.zoom);
    }

    pub fn handle_key(&mut self, code: KeyCode, pressed: bool) {
        self.input.set_key(code, pressed);
    }

    pub fn redraw(&mut self) -> Result<(), wgpu::SurfaceError> {
        let dt = self.time.advance();
        self.time.update_fps(dt, self.renderer.window(), self.paused);

        let steps = self.time.consume_fixed_steps();
        for _ in 0..steps {
            self.fixed_update(self.time.fixed_dt());
        }

        if self
            .assets
            .reload_if_changed(self.renderer.device(), self.renderer.queue())
        {
            self.renderer.set_texture(&self.assets.texture);
        }

        self.renderer.update_camera(self.camera.position, self.camera.zoom);
        self.instance_data.clear();
        self.world.for_each_sprite_world(|_, transform, sprite| {
            self.instance_data
                .push(InstanceRaw::from_components(transform, sprite, &self.assets.atlas));
        });
        self.renderer.update_instances(&self.instance_data);

        let result = self.renderer.render();
        self.input.finish_frame();
        result
    }

    fn fixed_update(&mut self, dt: f32) {
        if self.input.is_just_pressed(KeyCode::KeyP) {
            self.paused = !self.paused;
        }

        if self.input.is_just_pressed(KeyCode::KeyH) {
            log::info!(
                "Controls: arrows move sprite, WASD pan, Q/E zoom, Z/X rotate, C tint, N spawn, Space reset, P pause"
            );
        }

        if self.paused {
            return;
        }

        let move_speed = 300.0;
        let rotate_speed = 2.4;

        let mut sprite_dir = Vec2::ZERO;
        if self.input.is_pressed(KeyCode::ArrowLeft) {
            sprite_dir.x -= 1.0;
        }
        if self.input.is_pressed(KeyCode::ArrowRight) {
            sprite_dir.x += 1.0;
        }
        if self.input.is_pressed(KeyCode::ArrowUp) {
            sprite_dir.y += 1.0;
        }
        if self.input.is_pressed(KeyCode::ArrowDown) {
            sprite_dir.y -= 1.0;
        }

        if let Some(transform) = self.world.get_transform_mut(self.player) {
            if sprite_dir.length_squared() > 0.0 {
                transform.position += sprite_dir.normalize() * move_speed * dt;
            }
            if self.input.is_pressed(KeyCode::KeyZ) {
                transform.rotation -= rotate_speed * dt;
            }
            if self.input.is_pressed(KeyCode::KeyX) {
                transform.rotation += rotate_speed * dt;
            }
        }

        let mut camera_dir = Vec2::ZERO;
        if self.input.is_pressed(KeyCode::KeyA) {
            camera_dir.x -= 1.0;
        }
        if self.input.is_pressed(KeyCode::KeyD) {
            camera_dir.x += 1.0;
        }
        if self.input.is_pressed(KeyCode::KeyW) {
            camera_dir.y += 1.0;
        }
        if self.input.is_pressed(KeyCode::KeyS) {
            camera_dir.y -= 1.0;
        }
        if camera_dir.length_squared() > 0.0 {
            self.camera.position += camera_dir.normalize() * move_speed * dt;
        }

        let zoom_speed = 1.5;
        if self.input.is_pressed(KeyCode::KeyQ) {
            self.camera.zoom = (self.camera.zoom * (1.0 + zoom_speed * dt)).min(4.0);
        }
        if self.input.is_pressed(KeyCode::KeyE) {
            self.camera.zoom = (self.camera.zoom * (1.0 - zoom_speed * dt)).max(0.25);
        }

        if self.input.is_just_pressed(KeyCode::Space) {
            self.camera.position = Vec2::ZERO;
            self.camera.zoom = 1.0;
            if let Some(transform) = self.world.get_transform_mut(self.player) {
                transform.position = Vec2::ZERO;
                transform.rotation = 0.0;
            }
        }

        if self.input.is_just_pressed(KeyCode::KeyC) {
            self.player_color_index = (self.player_color_index + 1) % PALETTE.len();
            if let Some(sprite) = self.world.get_sprite_mut(self.player) {
                sprite.color = PALETTE[self.player_color_index];
            }
        }

        if self.input.is_just_pressed(KeyCode::KeyN) {
            let grid_x = (self.spawn_counter % 6) as f32;
            let grid_y = (self.spawn_counter / 6) as f32;
            let position = Vec2::new(grid_x * 110.0 - 220.0, grid_y * 110.0 - 160.0);
            let tile_index = self.spawn_counter % self.assets.atlas.tile_count().max(1);
            let color = PALETTE[self.spawn_counter as usize % PALETTE.len()];
            let spin = if self.spawn_counter % 2 == 0 { 0.4 } else { -0.3 };
            let angle = self.spawn_counter as f32 * 0.7;
            let velocity = Vec2::new(angle.cos(), angle.sin()) * 120.0;
            self.world.spawn_sprite_with_body(
                Transform::new(position),
                Sprite {
                    size: Vec2::splat(SPRITE_SIZE * 0.6),
                    tile_index,
                    color,
                    spin,
                    animation: None,
                },
                Some(Body::new(velocity)),
            );
            self.spawn_counter = self.spawn_counter.wrapping_add(1);
        }

        self.world.step_physics(dt, WORLD_BOUNDS);
        self.world.update_animations(dt);
    }
}
