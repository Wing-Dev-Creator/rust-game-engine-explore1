use glam::Vec2;

pub type Entity = u32;

#[derive(Clone, Copy)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Transform {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Body {
    pub velocity: Vec2,
    pub damping: f32,
    pub bounce: f32,
}

impl Body {
    pub fn new(velocity: Vec2) -> Self {
        Self {
            velocity,
            damping: 0.4,
            bounce: 0.75,
        }
    }
}

pub struct Animation {
    frames: Vec<u32>,
    fps: f32,
    timer: f32,
    current: usize,
}

impl Animation {
    pub fn new(frames: Vec<u32>, fps: f32) -> Self {
        Self {
            frames,
            fps,
            timer: 0.0,
            current: 0,
        }
    }

    pub fn update(&mut self, dt: f32) -> Option<u32> {
        if self.frames.is_empty() || self.fps <= 0.0 {
            return None;
        }

        let frame_time = 1.0 / self.fps;
        self.timer += dt;
        while self.timer >= frame_time {
            self.timer -= frame_time;
            self.current = (self.current + 1) % self.frames.len();
        }
        Some(self.frames[self.current])
    }
}

pub struct Sprite {
    pub size: Vec2,
    pub tile_index: u32,
    pub color: [f32; 4],
    pub spin: f32,
    pub animation: Option<Animation>,
}

pub struct World {
    transforms: Vec<Option<Transform>>,
    sprites: Vec<Option<Sprite>>,
    bodies: Vec<Option<Body>>,
    parents: Vec<Option<Entity>>,
    world_cache: Vec<Option<Transform>>,
    free: Vec<Entity>,
}

impl World {
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
            sprites: Vec::new(),
            bodies: Vec::new(),
            parents: Vec::new(),
            world_cache: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn spawn_sprite(&mut self, transform: Transform, sprite: Sprite) -> Entity {
        self.spawn_sprite_with_body(transform, sprite, None)
    }

    pub fn spawn_sprite_with_body(
        &mut self,
        transform: Transform,
        sprite: Sprite,
        body: Option<Body>,
    ) -> Entity {
        if let Some(entity) = self.free.pop() {
            let index = entity as usize;
            if index >= self.transforms.len() {
                return self.push_new(transform, sprite, body);
            }
            self.transforms[index] = Some(transform);
            self.sprites[index] = Some(sprite);
            self.bodies[index] = body;
            self.parents[index] = None;
            self.world_cache[index] = None;
            entity
        } else {
            self.push_new(transform, sprite, body)
        }
    }

    pub fn set_parent(&mut self, child: Entity, parent: Entity) {
        let index = child as usize;
        if index >= self.parents.len() {
            return;
        }
        self.parents[index] = Some(parent);
        self.world_cache[index] = None;
    }

    pub fn get_transform_mut(&mut self, entity: Entity) -> Option<&mut Transform> {
        self.transforms.get_mut(entity as usize)?.as_mut()
    }

    pub fn get_sprite_mut(&mut self, entity: Entity) -> Option<&mut Sprite> {
        self.sprites.get_mut(entity as usize)?.as_mut()
    }

    pub fn step_physics(&mut self, dt: f32, bounds: Vec2) {
        for index in 0..self.transforms.len() {
            let (Some(transform), Some(body)) = (
                self.transforms[index].as_mut(),
                self.bodies[index].as_mut(),
            ) else {
                continue;
            };

            let damping = (1.0 - body.damping * dt).clamp(0.0, 1.0);
            body.velocity *= damping;
            transform.position += body.velocity * dt;

            if transform.position.x < -bounds.x {
                transform.position.x = -bounds.x;
                body.velocity.x = body.velocity.x.abs() * body.bounce;
            } else if transform.position.x > bounds.x {
                transform.position.x = bounds.x;
                body.velocity.x = -body.velocity.x.abs() * body.bounce;
            }

            if transform.position.y < -bounds.y {
                transform.position.y = -bounds.y;
                body.velocity.y = body.velocity.y.abs() * body.bounce;
            } else if transform.position.y > bounds.y {
                transform.position.y = bounds.y;
                body.velocity.y = -body.velocity.y.abs() * body.bounce;
            }
        }
    }

    pub fn update_animations(&mut self, dt: f32) {
        for index in 0..self.transforms.len() {
            if let (Some(transform), Some(sprite)) = (
                self.transforms[index].as_mut(),
                self.sprites[index].as_mut(),
            ) {
                if let Some(animation) = sprite.animation.as_mut() {
                    if let Some(frame) = animation.update(dt) {
                        sprite.tile_index = frame;
                    }
                }
                transform.rotation += sprite.spin * dt;
            }
        }
    }

    pub fn for_each_sprite_world<F: FnMut(Entity, &Transform, &Sprite)>(&mut self, mut f: F) {
        self.build_world_transforms();
        for index in 0..self.transforms.len() {
            if let (Some(world), Some(sprite)) = (
                self.world_cache[index].as_ref(),
                self.sprites[index].as_ref(),
            ) {
                f(index as Entity, world, sprite);
            }
        }
    }

    fn build_world_transforms(&mut self) {
        for entry in &mut self.world_cache {
            *entry = None;
        }
        let len = self.transforms.len();
        for index in 0..len {
            let _ = self.compute_world(index);
        }
    }

    fn compute_world(&mut self, index: usize) -> Option<Transform> {
        if index >= self.transforms.len() {
            return None;
        }
        if let Some(cached) = self.world_cache[index] {
            return Some(cached);
        }

        let local = self.transforms[index]?;
        let world = match self.parents.get(index).and_then(|parent| *parent) {
            Some(parent) => {
                let parent_index = parent as usize;
                if parent_index == index {
                    local
                } else if let Some(parent_world) = self.compute_world(parent_index) {
                    combine_transforms(parent_world, local)
                } else {
                    local
                }
            }
            None => local,
        };

        self.world_cache[index] = Some(world);
        Some(world)
    }

    fn push_new(&mut self, transform: Transform, sprite: Sprite, body: Option<Body>) -> Entity {
        let entity = self.transforms.len() as Entity;
        self.transforms.push(Some(transform));
        self.sprites.push(Some(sprite));
        self.bodies.push(body);
        self.parents.push(None);
        self.world_cache.push(None);
        entity
    }
}

fn combine_transforms(parent: Transform, local: Transform) -> Transform {
    let scaled = local.position * parent.scale;
    let rotated = rotate_vec2(scaled, parent.rotation);
    Transform {
        position: parent.position + rotated,
        rotation: parent.rotation + local.rotation,
        scale: parent.scale * local.scale,
    }
}

fn rotate_vec2(value: Vec2, angle: f32) -> Vec2 {
    let c = angle.cos();
    let s = angle.sin();
    Vec2::new(value.x * c - value.y * s, value.x * s + value.y * c)
}
