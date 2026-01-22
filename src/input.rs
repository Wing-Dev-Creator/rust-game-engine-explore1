use std::collections::HashSet;
use winit::keyboard::KeyCode;

pub struct InputState {
    pressed: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        }
    }

    pub fn set_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            if self.pressed.insert(key) {
                self.just_pressed.insert(key);
            }
        } else if self.pressed.remove(&key) {
            self.just_released.insert(key);
        }
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn is_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }

    pub fn finish_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}
