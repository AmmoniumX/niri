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
