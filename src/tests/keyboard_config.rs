use niri_config::{Config, Xkb};

use super::Fixture;

fn config(text: &str) -> Config {
    Config::parse_mem(text).unwrap()
}

fn active_layout_name(state: &mut crate::niri::State) -> String {
    let keyboard = state.niri.seat.get_keyboard().unwrap();
    keyboard.with_xkb_state(state, |context| {
        let xkb = context.xkb().lock().unwrap();
        xkb.layout_name(xkb.active_layout()).to_owned()
    })
}

fn num_lock(state: &mut crate::niri::State) -> bool {
    let keyboard = state.niri.seat.get_keyboard().unwrap();
    keyboard.modifier_state().num_lock
}

const TWO_KEYBOARDS: &str = r#"
    input {
        keyboard {
            xkb {
                layout "gb"
            }
            numlock
        }
        keyboard "Ext" {
            xkb {
                layout "us"
            }
        }
    }
"#;

#[test]
fn switching_keyboard_config_applies_the_named_layout() {
    let config = config(TWO_KEYBOARDS);
    let ext = config.input.keyboard_named("Ext");
    let mut f = Fixture::with_config(config);
    let state = f.niri_state();
    assert_eq!(active_layout_name(state), "English (UK)");

    state.switch_keyboard_config(ext.clone());

    assert_eq!(active_layout_name(state), "English (US)");
    assert_eq!(state.niri.current_keyboard, ext);
}

#[test]
fn switching_keyboard_config_keeps_num_lock() {
    let config = config(TWO_KEYBOARDS);
    let ext = config.input.keyboard_named("Ext");
    let mut f = Fixture::with_config(config);
    let state = f.niri_state();
    assert!(
        num_lock(state),
        "numlock from the config must be on at startup"
    );

    state.switch_keyboard_config(ext);

    assert!(num_lock(state), "a keymap switch must not drop num lock");
}

#[test]
fn switching_back_to_an_unset_xkb_uses_locale1_settings() {
    let config = config(
        r#"
        input {
            keyboard "Ext" {
                xkb {
                    layout "us"
                }
            }
        }
        "#,
    );
    let ext = config.input.keyboard_named("Ext");
    let fallback = config.input.fallback_keyboard();
    let mut f = Fixture::with_config(config);
    let state = f.niri_state();
    state.niri.xkb_from_locale1 = Some(Xkb {
        layout: "de".to_owned(),
        ..Default::default()
    });

    state.switch_keyboard_config(ext);
    assert_eq!(active_layout_name(state), "English (US)");

    // The unnamed keyboard leaves xkb unset, so like a config reload it must
    // pick up the locale1 settings rather than the bare xkb defaults.
    state.switch_keyboard_config(fallback);
    assert_eq!(active_layout_name(state), "German");
}

#[test]
fn reloading_named_keyboard_config_restores_fallback_keymap() {
    let initial = config(TWO_KEYBOARDS);
    let ext = initial.input.keyboard_named("Ext");
    let mut f = Fixture::with_config(initial);
    let state = f.niri_state();
    state.switch_keyboard_config(ext);
    assert_eq!(active_layout_name(state), "English (US)");

    // Changing only the named block must reset the seat and the cached config together.
    state.reload_config(Ok(config(&TWO_KEYBOARDS.replace("us", "de"))));
    assert_eq!(active_layout_name(state), "English (UK)");
    assert!(num_lock(state));

    let ext = state.niri.config.borrow().input.keyboard_named("Ext");
    state.switch_keyboard_config(ext);
    assert_eq!(active_layout_name(state), "German");
}

#[test]
fn loading_configured_keymap_uses_active_physical_keyboard() {
    let config = config(TWO_KEYBOARDS);
    let ext = config.input.keyboard_named("Ext");
    let mut f = Fixture::with_config(config);
    let state = f.niri_state();
    state.switch_keyboard_config(ext);
    state.load_configured_keymap();
    assert_eq!(active_layout_name(state), "English (US)");
    assert!(num_lock(state));
}

#[test]
fn virtual_keyboard_preserves_and_restores_named_physical_keymap() {
    use std::ffi::CString;
    use std::os::fd::AsFd;

    use smithay::utils::SealedFile;

    let config = config(TWO_KEYBOARDS);
    let ext = config.input.keyboard_named("Ext");
    let mut f = Fixture::with_config(config);
    let state = f.niri_state();
    let mut virtual_config = ext.clone();
    virtual_config.xkb.layout = "de".to_owned();
    state.switch_keyboard_config(virtual_config);
    let keyboard = state.niri.seat.get_keyboard().unwrap();
    let keymap = keyboard.with_xkb_state(state, |context| {
        context.xkb().lock().unwrap().keymap_as_string()
    });
    let keymap = CString::new(keymap).unwrap();
    let keymap_file = SealedFile::with_content(c"test-keymap", &keymap).unwrap();
    state.switch_keyboard_config(ext.clone());

    let id = f.add_client();
    let client = f.client(id);
    let seat = client.state.seats.keys().next().unwrap();
    let manager = client.state.virtual_keyboard_manager.as_ref().unwrap();
    let virtual_keyboard = manager.create_virtual_keyboard(seat, &client.qh, ());
    virtual_keyboard.keymap(1, keymap_file.as_fd(), keymap_file.size() as u32);
    // KEY_A, pressed then released. These must retain the virtual keymap rather than
    // selecting the fallback config for a device which isn't backed by libinput.
    virtual_keyboard.key(0, 30, 1);
    virtual_keyboard.key(1, 30, 0);
    f.roundtrip(id);
    assert_eq!(active_layout_name(f.niri_state()), "German");
    assert_eq!(f.niri().current_keyboard, ext);
    assert!(f.niri().virtual_keyboard_keymap.is_some());

    virtual_keyboard.destroy();
    f.roundtrip(id);
    assert_eq!(active_layout_name(f.niri_state()), "English (US)");
    assert!(f.niri().virtual_keyboard_keymap.is_none());
    assert!(num_lock(f.niri_state()));
}
