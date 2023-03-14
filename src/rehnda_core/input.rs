use ahash::AHashMap;
use bevy_ecs::prelude::*;
use winit::event::VirtualKeyCode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyState {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyStateChange {
    None,
    JustDown,
    JustUp,
}

#[derive(Resource, Default)]
pub struct InputState {
    key_state: AHashMap<VirtualKeyCode, KeyState>,
    key_state_change: AHashMap<VirtualKeyCode, KeyStateChange>,
}

impl InputState {
    pub fn key_state(&self, key_code: VirtualKeyCode) -> KeyState {
        self.key_state.get(&key_code).map_or(KeyState::Up, |a| *a)
    }

    pub fn is_down(&self, key_code: VirtualKeyCode) -> bool {
        self.key_state.get(&key_code).map_or(false, |a| *a == KeyState::Down)
    }

    pub fn key_state_change(&self, key_code: VirtualKeyCode) -> KeyStateChange {
        self.key_state_change.get(&key_code).map_or(KeyStateChange::None, |a| *a)
    }

    pub fn is_just_down(&self, key_code: VirtualKeyCode) -> bool {
        self.key_state_change.get(&key_code).map_or(false, |a| *a == KeyStateChange::JustDown)
    }

    pub fn is_just_up(&self, key_code: VirtualKeyCode) -> bool {
        self.key_state_change.get(&key_code).map_or(false, |a| *a == KeyStateChange::JustUp)
    }
}

pub mod input_systems {
    use bevy_ecs::prelude::*;
    use log::info;
    use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};
    use crate::rehnda_core::input::{InputState, KeyState, KeyStateChange};

    pub fn input_system(mut input_state: ResMut<InputState>, mut keyboard_events: EventReader<KeyboardInput>) {
        input_state.key_state_change.clear();
        for event in keyboard_events.iter() {
            if let Some(virtual_keycode) = event.virtual_keycode {
                match event.state {
                    ElementState::Pressed => {
                        if input_state.key_state(virtual_keycode) == KeyState::Up {
                            input_state.key_state_change.insert(virtual_keycode, KeyStateChange::JustDown);
                        }
                        input_state.key_state.insert(virtual_keycode, KeyState::Down);
                    }
                    ElementState::Released => {
                        if input_state.key_state(virtual_keycode) == KeyState::Down {
                            input_state.key_state_change.insert(virtual_keycode, KeyStateChange::JustUp);
                        }
                        input_state.key_state.insert(virtual_keycode, KeyState::Up);
                    }
                }
            }
        }
    }
}