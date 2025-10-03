use glam::FloatExt;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use crate::collections::hashmap::HashMap;
use crate::collections::hashset::HashSet;
use crate::collections::SmallVec;

/// Represents the state of a key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// KeyCode was just pressed this frame
    JustPressed,
    /// KeyCode is being held down
    Held,
    /// KeyCode was just released this frame
    JustReleased,
    /// KeyCode is not pressed
    Released,
}

pub struct InputManager {
    keys_pressed: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    keys_just_released: HashSet<KeyCode>,
    keys_with_repeat: HashSet<KeyCode>,
    prev_keys_pressed: HashSet<KeyCode>,

    mouse_pressed: HashSet<MouseButton>,
    mouse_just_pressed: HashSet<MouseButton>,
    mouse_just_released: HashSet<MouseButton>,
    prev_mouse_pressed: HashSet<MouseButton>,

    modifiers: ModifiersState,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ModifiersState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            keys_with_repeat: HashSet::new(),
            prev_keys_pressed: HashSet::new(),

            mouse_pressed: HashSet::new(),
            mouse_just_pressed: HashSet::new(),
            mouse_just_released: HashSet::new(),
            prev_mouse_pressed: HashSet::new(),

            modifiers: ModifiersState::default(),
        }
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !event.repeat {
                                // only register as pressed if it's not a repeat event
                                self.keys_pressed.insert(keycode);
                                self.keys_with_repeat.remove(&keycode);
                            } else {
                                // mark this key as having repeat events
                                self.keys_with_repeat.insert(keycode);
                            }
                        }
                        ElementState::Released => {
                            self.keys_pressed.remove(&keycode);
                            self.keys_with_repeat.remove(&keycode);
                        }
                    }
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.mouse_pressed.insert(*button);
                    }
                    ElementState::Released => {
                        self.mouse_pressed.remove(button);
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = ModifiersState {
                    shift: modifiers.state().shift_key(),
                    ctrl: modifiers.state().control_key(),
                    alt: modifiers.state().alt_key(),
                    super_key: modifiers.state().super_key(),
                };
            }
            WindowEvent::Focused(false) => {
                // clear all input when window loses focus
                self.clear_all_input();
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();

        for key in &self.keys_pressed {
            if !self.prev_keys_pressed.contains(key) {
                self.keys_just_pressed.insert(*key);
            }
        }

        for key in &self.prev_keys_pressed {
            if !self.keys_pressed.contains(key) {
                self.keys_just_released.insert(*key);
            }
        }

        for button in &self.mouse_pressed {
            if !self.prev_mouse_pressed.contains(button) {
                self.mouse_just_pressed.insert(*button);
            }
        }

        for button in &self.prev_mouse_pressed {
            if !self.mouse_pressed.contains(button) {
                self.mouse_just_released.insert(*button);
            }
        }

        self.prev_keys_pressed = self.keys_pressed.clone();
        self.prev_mouse_pressed = self.mouse_pressed.clone();
    }
    
    pub fn key_state(&self, key: KeyCode) -> KeyState {
        if self.keys_just_pressed.contains(&key) {
            KeyState::JustPressed
        } else if self.keys_pressed.contains(&key) {
            KeyState::Held
        } else if self.keys_just_released.contains(&key) {
            KeyState::JustReleased
        } else {
            KeyState::Released
        }
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }
    
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }
    
    pub fn is_key_held(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key) && !self.keys_just_pressed.contains(&key)
    }
    
    pub fn pressed_keys(&self) -> &HashSet<KeyCode> {
        &self.keys_pressed
    }
    
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_just_pressed.contains(&button)
    }

    pub fn is_mouse_just_released(&self, button: MouseButton) -> bool {
        self.mouse_just_released.contains(&button)
    }

    pub fn modifiers(&self) -> &ModifiersState {
        &self.modifiers
    }

    pub fn clear_all_input(&mut self) {
        self.keys_pressed.clear();
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.keys_with_repeat.clear();
        self.mouse_pressed.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
    }
}

pub struct InputActionMapper {
    input: InputManager,
    action_mappings: HashMap<String, SmallVec<[KeyCode; 1]>>,
    axis_mappings: HashMap<String, AxisMapping>,
}

#[derive(Debug, Clone)]
pub struct AxisMapping {
    positive: SmallVec<[KeyCode; 1]>,
    negative: SmallVec<[KeyCode; 1]>,
    axis: f32,
    // Higher the value, server the lagging. Zero means no smoothing
    smoothing_factor: f32,
}

impl InputActionMapper {
    pub fn new() -> Self {
        Self {
            input: InputManager::new(),
            action_mappings: HashMap::new(),
            axis_mappings: HashMap::new(),
        }
    }

    pub fn register_action(&mut self, action: &str, keys: impl IntoIterator<Item = KeyCode>) {
        self.action_mappings.insert(action.to_string(), keys.into_iter().collect::<SmallVec<_>>());
    }

    pub fn register_axis(&mut self, axis: &str, positive: impl IntoIterator<Item = KeyCode>, negative: impl IntoIterator<Item = KeyCode>, smoothing_factor: f32) {
        self.axis_mappings.insert(
            axis.to_string(),
            AxisMapping {
                positive: positive.into_iter().collect::<SmallVec<_>>(),
                negative: negative.into_iter().collect::<SmallVec<_>>(),
                axis: 0.0,
                smoothing_factor,
            }
        );
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        self.input.on_window_event(event);
    }

    pub fn tick(&mut self, delta_time: f32) {
        self.input.tick();

        for mapping in self.axis_mappings.values_mut() {
            let blend_factor = 1.0 - mapping.smoothing_factor.powf(20. * delta_time);
            let axis_acceleration = 0.0.lerp(1.0, blend_factor);

            let mut any_input = false;
            for key in &mapping.positive {
                if self.input.is_key_pressed(*key) {
                    mapping.axis += axis_acceleration;
                    any_input = true;
                }
            }

            for key in &mapping.negative {
                if self.input.is_key_pressed(*key) {
                    mapping.axis -= axis_acceleration;
                    any_input = true;
                }
            }
            mapping.axis = mapping.axis.clamp(-1.0, 1.0);

            if !any_input {
                mapping.axis = mapping.axis.lerp(0.0, blend_factor);
            }
        }
    }

    pub fn is_action_pressed(&self, action: &str) -> bool {
        if let Some(keys) = self.action_mappings.get(action) {
            keys.iter().any(|key| self.input.is_key_pressed(*key))
        } else {
            false
        }
    }

    pub fn is_action_just_pressed(&self, action: &str) -> bool {
        if let Some(keys) = self.action_mappings.get(action) {
            keys.iter().any(|key| self.input.is_key_just_pressed(*key))
        } else {
            false
        }
    }

    pub fn get_axis(&self, axis: &str) -> f32 {
        if let Some(mapping) = self.axis_mappings.get(axis) {
            mapping.axis
        } else {
            0.0
        }
    }

    pub fn raw_input(&self) -> &InputManager {
        &self.input
    }
}