//! Translate egui input into RFB key and pointer events, with a Mac-shaped
//! keyboard layout: the Windows key becomes Command (Super), Alt becomes
//! Option, Ctrl stays Ctrl.

use egui::{Key, Modifiers, PointerButton};
use macpane_rfb::keysym;
use macpane_rfb::messages::buttons;

/// Map an egui key to an X11 keysym. Returns `None` for keys that should be
/// delivered through text input instead (letters, digits, punctuation).
pub fn keysym_for(key: Key) -> Option<u32> {
    Some(match key {
        Key::Backspace => keysym::BACKSPACE,
        Key::Tab => keysym::TAB,
        Key::Enter => keysym::RETURN,
        Key::Escape => keysym::ESCAPE,
        Key::Insert => keysym::INSERT,
        Key::Delete => keysym::DELETE,
        Key::Home => keysym::HOME,
        Key::End => keysym::END,
        Key::PageUp => keysym::PAGE_UP,
        Key::PageDown => keysym::PAGE_DOWN,
        Key::ArrowLeft => keysym::LEFT,
        Key::ArrowUp => keysym::UP,
        Key::ArrowRight => keysym::RIGHT,
        Key::ArrowDown => keysym::DOWN,
        Key::Space => keysym::SPACE,
        Key::F1 => keysym::function_key(1),
        Key::F2 => keysym::function_key(2),
        Key::F3 => keysym::function_key(3),
        Key::F4 => keysym::function_key(4),
        Key::F5 => keysym::function_key(5),
        Key::F6 => keysym::function_key(6),
        Key::F7 => keysym::function_key(7),
        Key::F8 => keysym::function_key(8),
        Key::F9 => keysym::function_key(9),
        Key::F10 => keysym::function_key(10),
        Key::F11 => keysym::function_key(11),
        Key::F12 => keysym::function_key(12),
        _ => return None,
    })
}

/// Modifier keysyms to press before a key when the given modifiers are
/// active, in press order. Command (Super) is emitted for the Windows key.
pub fn modifier_keysyms(m: Modifiers) -> Vec<u32> {
    let mut v = Vec::with_capacity(4);
    if m.shift {
        v.push(keysym::SHIFT_L);
    }
    if m.ctrl {
        v.push(keysym::CONTROL_L);
    }
    if m.alt {
        v.push(keysym::ALT_L);
    }
    if m.mac_cmd || m.command && !m.ctrl {
        v.push(keysym::SUPER_L);
    }
    v
}

pub fn button_bit(b: PointerButton) -> u8 {
    match b {
        PointerButton::Primary => buttons::LEFT,
        PointerButton::Middle => buttons::MIDDLE,
        PointerButton::Secondary => buttons::RIGHT,
        _ => 0,
    }
}

/// Convert a scroll delta into the RFB "button 4/5 click" convention.
/// Returns button bits to pulse (press + release) once per notch.
pub fn scroll_pulses(delta_y: f32, delta_x: f32) -> Vec<u8> {
    let mut out = Vec::new();
    let notches_y = (delta_y / 40.0).round() as i32;
    let notches_x = (delta_x / 40.0).round() as i32;
    for _ in 0..notches_y.abs() {
        out.push(if notches_y > 0 {
            buttons::SCROLL_UP
        } else {
            buttons::SCROLL_DOWN
        });
    }
    for _ in 0..notches_x.abs() {
        out.push(if notches_x > 0 {
            buttons::SCROLL_LEFT
        } else {
            buttons::SCROLL_RIGHT
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_go_through_text_input() {
        assert_eq!(keysym_for(Key::A), None);
        assert_eq!(keysym_for(Key::Enter), Some(keysym::RETURN));
    }

    #[test]
    fn windows_key_becomes_command() {
        let m = Modifiers {
            command: true,
            ..Default::default()
        };
        assert_eq!(modifier_keysyms(m), vec![keysym::SUPER_L]);
        let m = Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        assert_eq!(modifier_keysyms(m), vec![keysym::CONTROL_L]);
    }

    #[test]
    fn scroll_maps_to_button_pulses() {
        assert_eq!(
            scroll_pulses(80.0, 0.0),
            vec![buttons::SCROLL_UP, buttons::SCROLL_UP]
        );
        assert_eq!(scroll_pulses(-40.0, 0.0), vec![buttons::SCROLL_DOWN]);
        assert_eq!(scroll_pulses(0.0, -40.0), vec![buttons::SCROLL_RIGHT]);
        assert!(scroll_pulses(5.0, 0.0).is_empty());
    }
}
