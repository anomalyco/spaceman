//! The update controls: the sidebar's foot under the version, and General's
//! Updates section with the release channel.

use rune::{El, Tx, el, row, text};
use rune_kit::SegmentedControl;
use spaceman::{app::UiUpdateStatus, config::UpdateChannel};

use crate::{
    Ui,
    controls::{self, ButtonKind, Paint},
};

const CHANNELS: [(&str, UpdateChannel); 2] = [
    ("Stable", UpdateChannel::Stable),
    ("Dev", UpdateChannel::Dev),
];

fn status(ui: &Ui) -> UiUpdateStatus {
    ui.agent
        .as_ref()
        .map_or(UiUpdateStatus::Unavailable, |agent| agent.update.clone())
}

/// What the updater is doing, in a sentence.
fn describe(status: &UiUpdateStatus) -> String {
    match status {
        UiUpdateStatus::Idle => "Checks for updates automatically.".into(),
        UiUpdateStatus::Checking => "Checking for updates…".into(),
        UiUpdateStatus::UpToDate => "Spaceman is up to date.".into(),
        UiUpdateStatus::Downloading => "Downloading an update…".into(),
        UiUpdateStatus::Ready { version } => {
            format!("Spaceman {version} is ready. Windows are restored before it restarts.")
        }
        UiUpdateStatus::Installing => "Installing and restarting…".into(),
        UiUpdateStatus::Failed => "Couldn't check for updates. It tries again soon.".into(),
        UiUpdateStatus::SourceBuild => "Built from source; your checkout updates it.".into(),
        UiUpdateStatus::Unavailable => {
            "Updates need Spaceman in a folder you can write to, such as Applications.".into()
        }
    }
}

fn install_button(p: Paint, key: &str, kind: ButtonKind) -> El {
    controls::button(
        p,
        key,
        "Install & Restart",
        kind,
        |ui: &mut Ui, _: &mut Tx| {
            ui.install_update();
        },
    )
}

/// Under the version: the ready update, or a manual check and what the updater is doing.
pub fn sidebar(ui: &Ui, p: Paint) -> El {
    let c = &p.k.c;
    let update = status(ui);
    if update.ready_version().is_some() {
        return row()
            .key("update-ready")
            .mt(6.0)
            .pl(6.0)
            .gap(8.0)
            .center_y()
            .child(controls::status_dot(p.accent()))
            .child(install_button(p, "install-update", ButtonKind::Secondary));
    }
    let label = match update {
        UiUpdateStatus::Checking => "Checking for updates…",
        UiUpdateStatus::UpToDate => "Spaceman is up to date",
        UiUpdateStatus::Downloading => "Downloading update…",
        UiUpdateStatus::Installing => "Restarting…",
        UiUpdateStatus::Failed => "Update check failed",
        UiUpdateStatus::SourceBuild => "Built from source",
        UiUpdateStatus::Unavailable => "Updates unavailable",
        UiUpdateStatus::Idle | UiUpdateStatus::Ready { .. } => "Check for updates",
    };
    let mut foot = row()
        .key("check-for-updates")
        .h(20.0)
        .mt(2.0)
        .px(6.0)
        .center_y()
        .radius(p.k.radius.sm)
        .child(
            text(label)
                .key("label")
                .font(p.k.ty.small)
                .color(c.text_muted)
                .truncate(),
        );
    if update.can_check() {
        foot = foot
            .role("button")
            .label("Check for updates")
            .hover(c.hover)
            .press(c.pressed)
            .focusable(true)
            .focus_ring(2.0, -2.0, c.border_focus)
            .on_press(|ui: &mut Ui| ui.check_for_updates());
    }
    foot
}

/// General's Updates section: the running version and its update, and the channel.
pub fn section(ui: &Ui, p: Paint) -> El {
    let update = status(ui);
    let color = match update {
        UiUpdateStatus::Ready { .. } => p.accent(),
        UiUpdateStatus::Failed => p.k.c.warning.fg,
        _ => p.muted(),
    };
    let control = if update.ready_version().is_some() {
        install_button(p, "install", ButtonKind::Primary)
    } else if update.can_check() {
        controls::button(
            p,
            "check",
            "Check Now",
            ButtonKind::Secondary,
            |ui: &mut Ui, _: &mut Tx| ui.check_for_updates(),
        )
    } else {
        el().key("idle")
    };
    let version = controls::setting_row(
        p,
        "version",
        &format!("Spaceman {}", env!("CARGO_PKG_VERSION")),
        Some((describe(&update), color)),
        control,
    );
    let current = ui.doc.update_channel();
    let channel = controls::setting_row(
        p,
        "channel",
        "Update channel",
        Some((
            "Stable gets releases. Dev gets a build of every change merged to dev.".into(),
            p.muted(),
        )),
        SegmentedControl::new(
            &CHANNELS.map(|(label, _)| label.to_owned()),
            CHANNELS.iter().position(|(_, channel)| *channel == current),
        )
        .fit()
        .on_pick(|ui: &mut Ui, index, _: &mut Tx| {
            if let Some((_, channel)) = CHANNELS.get(index) {
                ui.set_update_channel(*channel);
            }
        })
        .build(p.k)
        .key("channel-picker"),
    );
    controls::section(p, "updates", "Updates", vec![version, channel])
}
