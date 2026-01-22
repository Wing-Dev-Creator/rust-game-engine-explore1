use std::time::Instant;
use winit::window::Window;

const MAX_DT: f32 = 0.25;

pub struct Time {
    last_frame: Instant,
    accumulator: f32,
    fixed_dt: f32,
    fps_timer: f32,
    fps_frames: u32,
}

impl Time {
    pub fn new(fixed_dt: f32) -> Self {
        Self {
            last_frame: Instant::now(),
            accumulator: 0.0,
            fixed_dt,
            fps_timer: 0.0,
            fps_frames: 0,
        }
    }

    pub fn advance(&mut self) -> f32 {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(MAX_DT);
        self.last_frame = now;
        self.accumulator += dt;
        dt
    }

    pub fn consume_fixed_steps(&mut self) -> u32 {
        let mut steps = 0;
        while self.accumulator >= self.fixed_dt {
            self.accumulator -= self.fixed_dt;
            steps += 1;
        }
        steps
    }

    pub fn fixed_dt(&self) -> f32 {
        self.fixed_dt
    }

    pub fn update_fps(&mut self, dt: f32, window: &Window, paused: bool) {
        self.fps_timer += dt;
        self.fps_frames += 1;
        if self.fps_timer >= 1.0 {
            let fps = self.fps_frames as f32 / self.fps_timer;
            let paused_marker = if paused { " [paused]" } else { "" };
            window.set_title(&format!("engine2d - {:.0} fps{}", fps, paused_marker));
            self.fps_timer = 0.0;
            self.fps_frames = 0;
        }
    }
}
