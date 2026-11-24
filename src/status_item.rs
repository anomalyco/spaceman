use std::{cell::RefCell, sync::mpsc::Sender};

use block2::RcBlock;
use objc2::runtime::AnyObject;
use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, msg_send, rc::Retained, runtime::Bool, sel,
};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSCompositingOperation, NSControlStateValueOff, NSControlStateValueOn,
    NSFont, NSFontAttributeName, NSGraphicsContext, NSImage, NSMenu, NSMenuItem, NSStatusBar,
    NSStatusBarButton, NSStatusItem, NSStringDrawing, NSVariableStatusItemLength,
};
use objc2_foundation::{
    NSDictionary, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use crate::{
    WorkspaceId,
    app::{Action, UiHudLayout, WorkerAction},
    login_item,
};

mod workspaces;

pub(crate) use workspaces::{DisplayWorkspaces, MenuWorkspace};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceLayout {
    pub(crate) workspace: String,
    /// The display's visible workspace: drawn filled, the others outlined.
    pub(crate) current: bool,
    pub(crate) mode: UiHudLayout,
}

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
        #[unsafe(method(openSetup:))]
        fn open_setup(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::OpenSetup));
        }

        #[unsafe(method(toggleAutomatic:))]
        fn toggle_automatic(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::ToggleAutomatic));
        }

        #[unsafe(method(toggleHud:))]
        fn toggle_hud(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::ToggleHud));
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::OpenSettings));
        }

        /// Checks and opens Settings, whose sidebar shows the check's progress.
        #[unsafe(method(checkForUpdates:))]
        fn check_for_updates(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::CheckForUpdates));
            drop(self.ivars().actions.send(Action::OpenSettings));
        }

        #[unsafe(method(installUpdate:))]
        fn install_update(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::InstallUpdate));
        }

        #[unsafe(method(toggleLaunchAtLogin:))]
        fn toggle_launch_at_login(&self, _sender: &NSObject) {
            drop(self.ivars().actions.send(Action::ToggleLaunchAtLogin));
        }

        #[unsafe(method(switchWorkspace:))]
        fn switch_workspace(&self, sender: &NSMenuItem) {
            if let Ok(index) = usize::try_from(sender.tag()) {
                drop(self.ivars().actions.send(Action::Worker(
                    WorkerAction::SelectWorkspace(WorkspaceId::from_index(index)),
                )));
            }
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
    menu: Retained<NSMenu>,
    target: Retained<Target>,
    button: Retained<NSStatusBarButton>,
    status: Retained<NSMenuItem>,
    setup: Retained<NSMenuItem>,
    setup_separator: Retained<NSMenuItem>,
    automatic: Retained<NSMenuItem>,
    hud: Retained<NSMenuItem>,
    launch_at_login: Retained<NSMenuItem>,
    update: Retained<NSMenuItem>,
    status_title: String,
    workspace_layouts: Vec<WorkspaceLayout>,
    workspace_menu: Vec<DisplayWorkspaces>,
    workspace_items: Vec<Retained<NSMenuItem>>,
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
    let setup = action_item("Setup Tour…", &target, sel!(openSetup:), "", marker);
    setup.setHidden(true);
    menu.addItem(&setup);
    let setup_separator = NSMenuItem::separatorItem(marker);
    setup_separator.setHidden(true);
    menu.addItem(&setup_separator);
    let automatic = action_item("Pause Tiling", &target, sel!(toggleAutomatic:), "", marker);
    menu.addItem(&automatic);
    menu.addItem(&NSMenuItem::separatorItem(marker));
    let hud = action_item("Workspace HUD", &target, sel!(toggleHud:), "", marker);
    hud.setState(NSControlStateValueOn);
    menu.addItem(&hud);
    let launch_at_login = action_item(
        "Launch at Login",
        &target,
        sel!(toggleLaunchAtLogin:),
        "",
        marker,
    );
    menu.addItem(&launch_at_login);
    menu.addItem(&NSMenuItem::separatorItem(marker));
    menu.addItem(&action_item(
        "Settings...",
        &target,
        sel!(openSettings:),
        ",",
        marker,
    ));
    let update = action_item(
        CHECK_FOR_UPDATES,
        &target,
        sel!(checkForUpdates:),
        "",
        marker,
    );
    menu.addItem(&update);
    menu.addItem(&NSMenuItem::separatorItem(marker));
    menu.addItem(&action_item(
        "Quit Spaceman",
        &target,
        sel!(quit:),
        "q",
        marker,
    ));

    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    let button = item
        .button(marker)
        .ok_or("status item button is unavailable")?;
    item.setMenu(Some(&menu));

    CONTROLLER.with(|controller| {
        *controller.borrow_mut() = Some(Controller {
            _item: item,
            menu,
            target,
            button,
            status,
            setup,
            setup_separator,
            automatic,
            hud,
            launch_at_login,
            update,
            status_title: String::new(),
            workspace_layouts: Vec::new(),
            workspace_menu: Vec::new(),
            workspace_items: Vec::new(),
        });
    });
    Ok(())
}

const CHECK_FOR_UPDATES: &str = "Check for Updates...";

/// The update item checks, or installs a ready update.
pub fn set_update(ready: Option<&str>) {
    CONTROLLER.with(|controller| {
        if let Some(controller) = controller.borrow().as_ref() {
            let (title, action) = match ready {
                Some(version) => (
                    format!("Install Spaceman {version} and Restart"),
                    sel!(installUpdate:),
                ),
                None => (CHECK_FOR_UPDATES.to_owned(), sel!(checkForUpdates:)),
            };
            controller.update.setTitle(&NSString::from_str(&title));
            // SAFETY: Both selectors are methods on `Target`, the item's retained target.
            unsafe { controller.update.setAction(Some(action)) };
        }
    });
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

pub fn set_accessibility(trusted: bool) {
    CONTROLLER.with(|controller| {
        if let Some(controller) = controller.borrow().as_ref() {
            controller.setup.setHidden(trusted);
            controller.setup_separator.setHidden(trusted);
        }
    });
}

pub fn set_hud(enabled: bool) {
    CONTROLLER.with(|controller| {
        if let Some(controller) = controller.borrow().as_ref() {
            controller.hud.setState(if enabled {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    });
}

pub fn set_automatic(enabled: bool) {
    if !enabled {
        // Switching is ignored while paused, and the list goes stale.
        set_workspace_menu(&[]);
    }
    CONTROLLER.with(|controller| {
        if let Some(controller) = controller.borrow().as_ref() {
            controller
                .automatic
                .setTitle(&NSString::from_str(if enabled {
                    "Pause Tiling"
                } else {
                    "Resume Tiling"
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

/// Lists `displays`' workspaces above the tiling toggle.
pub fn set_workspace_menu(displays: &[DisplayWorkspaces]) {
    let Some(marker) = MainThreadMarker::new() else {
        return;
    };
    CONTROLLER.with(|controller| {
        let mut controller = controller.borrow_mut();
        let Some(controller) = controller.as_mut() else {
            return;
        };
        if controller.workspace_menu == displays {
            return;
        }
        for item in controller.workspace_items.drain(..) {
            controller.menu.removeItem(&item);
        }
        let items = workspaces::items(displays, &controller.target, marker);
        let start = controller.menu.indexOfItem(&controller.automatic);
        for (offset, item) in (start..).zip(&items) {
            controller.menu.insertItem_atIndex(item, offset);
        }
        controller.workspace_items = items;
        controller.workspace_menu = displays.to_vec();
    });
}

const BOX_HEIGHT: f64 = 18.0;
const BOX_MIN_WIDTH: f64 = 20.0;
const BOX_PADDING: f64 = 10.0;
const BOX_GAP: f64 = 5.0;
const IMAGE_HEIGHT: f64 = 20.0;
const BOX_Y: f64 = (IMAGE_HEIGHT - BOX_HEIGHT) / 2.0;

/// AeroSpace-style strip: every listed workspace as a numbered box, the visible one filled
/// with its number knocked out, the others outlined.
fn workspace_layout_image(layouts: &[WorkspaceLayout]) -> Retained<NSImage> {
    let labels = layouts
        .iter()
        .map(|layout| NSString::from_str(&layout.workspace))
        .collect::<Vec<_>>();
    let attributes = label_attributes();
    let label_sizes = labels
        .iter()
        .map(|label| {
            // SAFETY: `attributes` maps `NSFontAttributeName` to an `NSFont`.
            unsafe { label.sizeWithAttributes(Some(&attributes)) }
        })
        .collect::<Vec<_>>();
    let box_widths = label_sizes
        .iter()
        .map(|size| (size.width.ceil() + BOX_PADDING).max(BOX_MIN_WIDTH))
        .collect::<Vec<_>>();
    let width =
        (box_widths.iter().sum::<f64>() + layouts.len().saturating_sub(1) as f64 * BOX_GAP + 1.0)
            .max(1.0);
    let layouts = layouts.to_vec();
    let image = NSImage::imageWithSize_flipped_drawingHandler(
        NSSize::new(width, IMAGE_HEIGHT),
        false,
        &RcBlock::new(move |_: NSRect| {
            draw_workspace_boxes(&layouts, &labels, &label_sizes, &box_widths, &attributes);
            Bool::YES
        }),
    );
    image.setTemplate(true);
    image
}

fn label_attributes() -> Retained<NSDictionary<NSString, AnyObject>> {
    let font = NSFont::boldSystemFontOfSize(13.5);
    // SAFETY: `NSFontAttributeName` is an immutable AppKit constant.
    let key = unsafe { NSFontAttributeName };
    NSDictionary::<NSString, AnyObject>::from_slices(&[key], &[font.as_ref()])
}

fn draw_workspace_boxes(
    layouts: &[WorkspaceLayout],
    labels: &[Retained<NSString>],
    label_sizes: &[NSSize],
    box_widths: &[f64],
    attributes: &NSDictionary<NSString, AnyObject>,
) {
    NSColor::blackColor().setStroke();
    NSColor::blackColor().setFill();
    let context = NSGraphicsContext::currentContext();
    let mut x = 0.75;
    for (((layout, label), size), box_width) in layouts
        .iter()
        .zip(labels)
        .zip(label_sizes)
        .zip(box_widths.iter().copied())
    {
        let frame = NSRect::new(NSPoint::new(x, BOX_Y), NSSize::new(box_width, BOX_HEIGHT));
        let outline = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(frame, 4.5, 4.5);
        let text_at = NSPoint::new(
            x + (box_width - size.width) / 2.0,
            BOX_Y + (BOX_HEIGHT - size.height) / 2.0,
        );
        if layout.current {
            outline.fill();
            if let Some(context) = &context {
                context.setCompositingOperation(NSCompositingOperation::Clear);
            }
        } else {
            outline.setLineWidth(1.5);
            outline.stroke();
        }
        // SAFETY: Called inside `NSImage` drawing handler with an active graphics context, and
        // `attributes` maps `NSFontAttributeName` to an `NSFont`.
        unsafe { label.drawAtPoint_withAttributes(text_at, Some(attributes)) };
        if let Some(context) = &context {
            context.setCompositingOperation(NSCompositingOperation::SourceOver);
        }
        x += box_width + BOX_GAP;
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
    key: &str,
    marker: MainThreadMarker,
) -> Retained<NSMenuItem> {
    // SAFETY: `action` matches a method declared on `Target` taking a single sender object.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(marker),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(key),
        )
    };
    // SAFETY: `target` is retained inside `Controller` for the lifetime of the menu.
    unsafe { item.setTarget(Some(target)) };
    item
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    fn layout(workspace: &str, current: bool) -> WorkspaceLayout {
        WorkspaceLayout {
            workspace: workspace.to_owned(),
            current,
            mode: UiHudLayout::Fullscreen,
        }
    }

    /// Renders sample strips to `target/menubar-preview-*.tiff` for eyeballing.
    #[test]
    #[ignore = "writes preview images"]
    fn render_menu_bar_preview() {
        let samples: [(&str, Vec<WorkspaceLayout>); 3] = [
            ("one-three", vec![layout("1", false), layout("3", true)]),
            ("single", vec![layout("3", true)]),
            (
                "busy",
                vec![
                    layout("1", false),
                    layout("2", true),
                    layout("4", false),
                    layout("10", false),
                    layout("B", false),
                ],
            ),
        ];
        for (name, layouts) in samples {
            let image = workspace_layout_image(&layouts);
            let data = image.TIFFRepresentation().expect("TIFF data");
            std::fs::write(format!("target/menubar-preview-{name}.tiff"), data.to_vec())
                .expect("write preview");
        }
    }
}
