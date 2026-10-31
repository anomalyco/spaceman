use std::{cell::RefCell, sync::mpsc::Sender};

use block2::RcBlock;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, msg_send, rc::Retained, runtime::Bool, sel,
};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSControlStateValueOff, NSControlStateValueOn, NSImage, NSMenu,
    NSMenuItem, NSStatusBar, NSStatusBarButton, NSStatusItem, NSStringDrawing,
    NSVariableStatusItemLength,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use crate::{app::Action, login_item};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceLayout {
    pub(crate) workspace: String,
    pub(crate) mode: WorkspaceLayoutMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceLayoutMode {
    Split(Vec<crate::Rect>),
    Fullscreen,
}

pub(crate) const WORKSPACE_PREVIEW: crate::Rect = crate::Rect {
    x: 0,
    y: 0,
    width: 1500,
    height: 1100,
};

struct TargetIvars {
    actions: Sender<Action>,
}

objc2::define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TargetIvars]
    struct Target;

    unsafe impl NSObjectProtocol for Target {}

    impl Target {
        #[unsafe(method(toggleAutomatic:))]
        fn toggle_automatic(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::ToggleAutomatic));
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::OpenSettings));
        }

        #[unsafe(method(toggleLaunchAtLogin:))]
        fn toggle_launch_at_login(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::ToggleLaunchAtLogin));
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::Quit));
        }
    }
);

impl Target {
    fn new(actions: Sender<Action>, marker: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(marker).set_ivars(TargetIvars { actions });
        // SAFETY: `NSObject`'s `-init` takes no arguments and returns an initialized `Self`.
        unsafe { msg_send![super(this), init] }
    }
}

struct Controller {
    _item: Retained<NSStatusItem>,
    _menu: Retained<NSMenu>,
    _target: Retained<Target>,
    button: Retained<NSStatusBarButton>,
    status: Retained<NSMenuItem>,
    automatic: Retained<NSMenuItem>,
    launch_at_login: Retained<NSMenuItem>,
    status_title: String,
    workspace_layouts: Vec<WorkspaceLayout>,
}

thread_local! {
    static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) };
}

pub fn install(sender: Sender<Action>) -> Result<(), String> {
    let marker = MainThreadMarker::new().ok_or("status item requires the main thread")?;
    let target = Target::new(sender, marker);
    let menu = NSMenu::initWithTitle(NSMenu::alloc(marker), &NSString::from_str("Spaceman"));
    let status = plain_item("Starting...", marker);
    status.setEnabled(false);
    status.setHidden(true);
    menu.addItem(&status);
    menu.addItem(&NSMenuItem::separatorItem(marker));
    let automatic = action_item("Disable Tiling", &target, sel!(toggleAutomatic:), marker);
    menu.addItem(&automatic);
    menu.addItem(&NSMenuItem::separatorItem(marker));
    let launch_at_login = action_item(
        "Launch at Login",
        &target,
        sel!(toggleLaunchAtLogin:),
        marker,
    );
    menu.addItem(&launch_at_login);
    menu.addItem(&NSMenuItem::separatorItem(marker));
    menu.addItem(&action_item(
        "Settings...",
        &target,
        sel!(openSettings:),
        marker,
    ));
    menu.addItem(&NSMenuItem::separatorItem(marker));
    menu.addItem(&action_item("Quit Spaceman", &target, sel!(quit:), marker));

    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    let button = item
        .button(marker)
        .ok_or("status item button is unavailable")?;
    item.setMenu(Some(&menu));

    CONTROLLER.with(|controller| {
        *controller.borrow_mut() = Some(Controller {
            _item: item,
            _menu: menu,
            _target: target,
            button,
            status,
            automatic,
            launch_at_login,
            status_title: String::new(),
            workspace_layouts: Vec::new(),
        });
    });
    Ok(())
}

pub fn set_status(status: &str) {
    CONTROLLER.with(|controller| {
        let mut controller = controller.borrow_mut();
        if let Some(controller) = controller.as_mut()
            && controller.status_title != status
        {
            controller.status.setHidden(false);
            controller.status.setTitle(&NSString::from_str(status));
            status.clone_into(&mut controller.status_title);
        }
    });
}

pub fn clear_status() {
    CONTROLLER.with(|controller| {
        let mut controller = controller.borrow_mut();
        if let Some(controller) = controller.as_mut() {
            controller.status.setHidden(true);
            controller.status_title.clear();
        }
    });
}

pub fn set_automatic(enabled: bool) {
    CONTROLLER.with(|controller| {
        if let Some(controller) = controller.borrow().as_ref() {
            controller
                .automatic
                .setTitle(&NSString::from_str(if enabled {
                    "Disable Tiling"
                } else {
                    "Enable Tiling"
                }));
        }
    });
}

pub fn set_launch_at_login(status: login_item::Status) {
    CONTROLLER.with(|controller| {
        if let Some(controller) = controller.borrow().as_ref() {
            let (title, state) = match status {
                login_item::Status::Disabled => ("Launch at Login", NSControlStateValueOff),
                login_item::Status::Enabled => ("Launch at Login", NSControlStateValueOn),
                login_item::Status::RequiresApproval => (
                    "Launch at Login (Approval Required...)",
                    NSControlStateValueOff,
                ),
            };
            controller
                .launch_at_login
                .setTitle(&NSString::from_str(title));
            controller.launch_at_login.setState(state);
        }
    });
}

pub fn set_workspace_numbers(workspaces: &[u8]) {
    let title = if workspaces.is_empty() {
        String::new()
    } else {
        format!(
            " {}",
            workspaces
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(" · ")
        )
    };
    CONTROLLER.with(|controller| {
        if let Some(controller) = controller.borrow_mut().as_mut() {
            controller.button.setImage(None);
            controller.button.setTitle(&NSString::from_str(&title));
            controller.workspace_layouts.clear();
        }
    });
}

pub fn set_workspace_layouts(layouts: &[WorkspaceLayout]) {
    CONTROLLER.with(|controller| {
        let mut controller = controller.borrow_mut();
        if let Some(controller) = controller.as_mut()
            && controller.workspace_layouts != layouts
        {
            let image = workspace_layout_image(layouts);
            controller.button.setTitle(&NSString::new());
            controller.button.setImage(Some(&image));
            controller.workspace_layouts = layouts.to_vec();
        }
    });
}

fn workspace_layout_image(layouts: &[WorkspaceLayout]) -> Retained<NSImage> {
    let labels = layouts
        .iter()
        .map(|layout| NSString::from_str(&layout.workspace))
        .collect::<Vec<_>>();
    let label_widths = labels
        .iter()
        .map(|label| {
            // SAFETY: `sizeWithAttributes(None)` measures `label` using AppKit's default font.
            unsafe { label.sizeWithAttributes(None) }.width.ceil()
        })
        .collect::<Vec<_>>();
    let width = label_widths.iter().sum::<f64>() + layouts.len() as f64 * 24.0 - 6.0;
    let layouts = layouts.to_vec();
    let image = NSImage::imageWithSize_flipped_drawingHandler(
        NSSize::new(width, 16.0),
        false,
        &RcBlock::new(move |_: NSRect| {
            draw_workspace_layouts(&layouts, &labels, &label_widths);
            Bool::YES
        }),
    );
    image.setTemplate(true);
    image
}

fn draw_workspace_layouts(
    layouts: &[WorkspaceLayout],
    labels: &[Retained<NSString>],
    label_widths: &[f64],
) {
    NSColor::blackColor().setStroke();
    NSColor::blackColor().setFill();
    let mut x = 0.5;
    for ((layout, label), label_width) in
        layouts.iter().zip(labels).zip(label_widths.iter().copied())
    {
        let frame = NSRect::new(NSPoint::new(x, 2.5), NSSize::new(15.0, 11.0));
        let outline = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, 2.5, 2.5);
        match &layout.mode {
            WorkspaceLayoutMode::Fullscreen => outline.fill(),
            WorkspaceLayoutMode::Split(tiles) => {
                outline.setLineWidth(1.25);
                outline.stroke();
                for tile in tiles {
                    let tile = NSRect::new(
                        NSPoint::new(
                            x + f64::from(tile.x) / f64::from(WORKSPACE_PREVIEW.width) * 15.0,
                            // Preview frames are top-down; the image is unflipped.
                            2.5 + (f64::from(WORKSPACE_PREVIEW.height)
                                - f64::from(tile.y)
                                - f64::from(tile.height))
                                / f64::from(WORKSPACE_PREVIEW.height)
                                * 11.0,
                        ),
                        NSSize::new(
                            f64::from(tile.width) / f64::from(WORKSPACE_PREVIEW.width) * 15.0,
                            f64::from(tile.height) / f64::from(WORKSPACE_PREVIEW.height) * 11.0,
                        ),
                    );
                    let tile = NSBezierPath::bezierPathWithRect(tile);
                    tile.setLineWidth(0.75);
                    tile.stroke();
                }
            }
        }
        // SAFETY: Called inside `NSImage` drawing handler with a active graphics context.
        unsafe { label.drawAtPoint_withAttributes(NSPoint::new(x + 19.0, 0.5), None) };
        x += 24.0 + label_width;
    }
}

fn plain_item(title: &str, marker: MainThreadMarker) -> Retained<NSMenuItem> {
    // SAFETY: `NSMenuItem` is allocated on the main thread with no target/action selector.
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(marker),
            &NSString::from_str(title),
            None,
            &NSString::new(),
        )
    }
}

fn action_item(
    title: &str,
    target: &Target,
    action: objc2::runtime::Sel,
    marker: MainThreadMarker,
) -> Retained<NSMenuItem> {
    // SAFETY: `action` matches a method declared on `Target` taking a single sender object.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(marker),
            &NSString::from_str(title),
            Some(action),
            &NSString::new(),
        )
    };
    // SAFETY: `target` is retained inside `Controller` for the lifetime of the menu.
    unsafe { item.setTarget(Some(target)) };
    item
}
