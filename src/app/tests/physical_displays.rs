use super::*;

/// A landscape main display with a portrait one of the same model to its left, which macOS
/// lists second.
fn portrait_left() -> macos::DisplayOutputs {
    macos::DisplayOutputs::new(vec![
        macos::DisplayOutput {
            id: "main-id".into(),
            name: "DELL U2720Q (1)".into(),
            frame: crate::Rect {
                width: 1000,
                height: 800,
                ..crate::Rect::default()
            },
            is_main: true,
        },
        macos::DisplayOutput {
            id: "left-id".into(),
            name: "DELL U2720Q (2)".into(),
            frame: crate::Rect {
                x: -800,
                width: 800,
                height: 1000,
                ..crate::Rect::default()
            },
            is_main: false,
        },
    ])
    .unwrap()
}

fn shown(status: &CycleStatus) -> Vec<(String, bool, Option<String>)> {
    status
        .layouts
        .iter()
        .map(|layout| {
            (
                layout.workspace.clone(),
                layout.portrait,
                layout.display.clone(),
            )
        })
        .collect()
}

#[test]
fn menu_bar_keeps_the_order_macos_reports_while_the_lab_is_off() {
    let runtime = runtime(FakeBackend::default());
    let status = runtime.status(0, &portrait_left());
    assert_eq!(
        shown(&status),
        [("1".into(), false, None), ("6".into(), false, None)]
    );
    let names: Vec<_> = status.menu.iter().map(|d| d.display.as_str()).collect();
    assert_eq!(names, ["DELL U2720Q (1)", "DELL U2720Q (2)"]);
}

#[test]
fn menu_bar_follows_the_physical_order_and_shape_with_the_lab_on() {
    let config = Config::parse("version = 1\n[labs]\nphysical_displays = true\n").unwrap();
    let runtime = runtime_with_config(FakeBackend::default(), config);
    let status = runtime.status(0, &portrait_left());
    assert_eq!(
        shown(&status),
        [
            ("6".into(), true, Some("DELL U2720Q (left)".into())),
            ("1".into(), false, Some("DELL U2720Q (right)".into())),
        ]
    );
    let names: Vec<_> = status.menu.iter().map(|d| d.display.as_str()).collect();
    assert_eq!(names, ["DELL U2720Q (left)", "DELL U2720Q (right)"]);
}
