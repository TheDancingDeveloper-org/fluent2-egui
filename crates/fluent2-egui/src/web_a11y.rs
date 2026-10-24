//! Browser accessibility support for egui applications.
//!
//! [`WebAccessibilityPlugin`] mirrors egui's AccessKit output into transparent, absolutely
//! positioned DOM elements. Browser find, screen readers, translation tools and extensions can
//! then discover visible text plus the roles, names and focus of common widgets even though egui
//! paints the application into a canvas.
//!
//! This first version mirrors common text, button, link, selection, input, menu, tab, tree and
//! grid roles. It synchronizes focus in both directions. DOM activation, text editing, nested
//! tree structure and virtualized off-screen content are intentionally deferred.
//!
//! ```no_run
//! # #[cfg(target_arch = "wasm32")]
//! # fn install(ctx: &egui::Context) {
//! ctx.add_plugin(fluent2_egui::web_a11y::WebAccessibilityPlugin);
//! # }
//! ```

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use egui::accesskit::{self, Action, ActionRequest, NodeId, Role, Toggled};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::closure::Closure;

const LAYER_ID: &str = "fluent2-accessibility-layer";
const CANVAS_ID: &str = "the_canvas_id";

#[derive(Clone, PartialEq, Eq)]
struct Signature {
    role: &'static str,
    text: String,
    value: Option<String>,
    rect: [i32; 4],
    focusable: bool,
    disabled: bool,
    selected: Option<bool>,
    expanded: Option<bool>,
    checked: Option<&'static str>,
}

struct MirroredNode {
    element: web_sys::HtmlElement,
    signature: Signature,
    _on_focus: Option<Closure<dyn FnMut()>>,
}

#[derive(Default)]
struct Layer {
    root: Option<web_sys::HtmlElement>,
    nodes: HashMap<NodeId, MirroredNode>,
    focused: Option<NodeId>,
}

thread_local! {
    static LAYER: RefCell<Layer> = RefCell::new(Layer::default());
    static PENDING_FOCUS: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
}

/// An egui plugin that exposes the AccessKit tree to browser accessibility APIs.
///
/// Register one instance on wasm before the first frame. The plugin enables AccessKit output,
/// creates its DOM layer lazily, and updates only nodes whose exposed properties changed.
pub struct WebAccessibilityPlugin;

impl egui::Plugin for WebAccessibilityPlugin {
    fn debug_name(&self) -> &'static str {
        "fluent2_web_accessibility"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        ctx.enable_accesskit();
    }

    fn input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        PENDING_FOCUS.with_borrow_mut(|pending| {
            for target_node in pending.drain(..) {
                input
                    .events
                    .push(egui::Event::AccessKitActionRequest(ActionRequest {
                        action: Action::Focus,
                        target_tree: accesskit::TreeId::ROOT,
                        target_node,
                        data: None,
                    }));
            }
        });
    }

    fn output_hook(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput) {
        let Some(update) = output.platform_output.accesskit_update.as_ref() else {
            return;
        };
        let repaint = ctx.clone();
        LAYER.with_borrow_mut(|layer| layer.sync(update, &repaint));
    }
}

impl Layer {
    fn root(&mut self) -> Option<web_sys::HtmlElement> {
        if let Some(root) = &self.root {
            return Some(root.clone());
        }
        let document = web_sys::window()?.document()?;
        let root: web_sys::HtmlElement = document.create_element("div").ok()?.dyn_into().ok()?;
        root.set_id(LAYER_ID);
        let style = root.style();
        let _ = style.set_property("position", "fixed");
        let _ = style.set_property("inset", "0");
        let _ = style.set_property("pointer-events", "none");
        let _ = style.set_property("overflow", "hidden");
        let _ = style.set_property("color", "transparent");
        let _ = style.set_property("z-index", "1");
        document.body()?.append_child(&root).ok()?;
        self.root = Some(root.clone());
        Some(root)
    }

    fn sync(&mut self, update: &accesskit::TreeUpdate, ctx: &egui::Context) {
        let Some(root) = self.root() else {
            return;
        };
        let Some(document) = root.owner_document() else {
            return;
        };
        let offset = canvas_offset(&document);
        let mut seen = HashSet::with_capacity(update.nodes.len());

        for (id, node) in &update.nodes {
            let Some(signature) = signature(node, offset) else {
                continue;
            };
            seen.insert(*id);
            if self
                .nodes
                .get(id)
                .is_some_and(|mirrored| mirrored.signature == signature)
            {
                continue;
            }
            if let Some(mirrored) = self.nodes.get_mut(id) {
                apply(&mirrored.element, &signature);
                mirrored.signature = signature;
                continue;
            }
            let Some(mirrored) = create(&document, *id, signature, ctx) else {
                continue;
            };
            if root.append_child(&mirrored.element).is_ok() {
                self.nodes.insert(*id, mirrored);
            }
        }

        self.nodes.retain(|id, mirrored| {
            if seen.contains(id) {
                true
            } else {
                mirrored.element.remove();
                false
            }
        });

        if self.focused != Some(update.focus) {
            if let Some(previous) = self.focused.and_then(|id| self.nodes.get(&id)) {
                let _ = previous.element.blur();
            }
            self.focused = Some(update.focus);
            if let Some(mirrored) = self.nodes.get(&update.focus) {
                let _ = mirrored.element.focus();
            }
        }
    }
}

fn canvas_offset(document: &web_sys::Document) -> (f64, f64) {
    document
        .get_element_by_id(CANVAS_ID)
        .map(|canvas| canvas.get_bounding_client_rect())
        .map(|rect| (rect.left(), rect.top()))
        .unwrap_or((0.0, 0.0))
}

fn signature(node: &accesskit::Node, offset: (f64, f64)) -> Option<Signature> {
    let role = aria_role(node.role())?;
    if node.is_hidden() {
        return None;
    }
    let text = node
        .label()
        .or_else(|| node.value())
        .unwrap_or_default()
        .to_owned();
    if text.is_empty() && matches!(node.role(), Role::Label | Role::TextRun | Role::Paragraph) {
        return None;
    }
    let bounds = node.bounds()?;
    #[expect(clippy::cast_possible_truncation, reason = "screen coordinates fit in i32")]
    let rect = [
        (bounds.x0 + offset.0).round() as i32,
        (bounds.y0 + offset.1).round() as i32,
        bounds.width().round().max(0.0) as i32,
        bounds.height().round().max(0.0) as i32,
    ];
    Some(Signature {
        role,
        text,
        value: node.value().map(ToOwned::to_owned),
        rect,
        focusable: !node.is_disabled()
            && (node.supports_action(Action::Focus) || is_interactive(node.role())),
        disabled: node.is_disabled(),
        selected: node.is_selected(),
        expanded: node.is_expanded(),
        checked: node.toggled().map(|value| match value {
            Toggled::False => "false",
            Toggled::True => "true",
            Toggled::Mixed => "mixed",
        }),
    })
}

fn apply(element: &web_sys::HtmlElement, signature: &Signature) {
    element.set_text_content(Some(&signature.text));
    set_attribute(
        element,
        "role",
        (!signature.role.is_empty()).then_some(signature.role),
    );
    set_attribute(
        element,
        "aria-label",
        (!signature.text.is_empty()).then_some(&signature.text),
    );
    set_attribute(element, "aria-valuetext", signature.value.as_deref());
    set_attribute(element, "aria-disabled", signature.disabled.then_some("true"));
    set_attribute(element, "aria-selected", signature.selected.map(bool_text));
    set_attribute(element, "aria-expanded", signature.expanded.map(bool_text));
    set_attribute(element, "aria-checked", signature.checked);
    set_attribute(element, "tabindex", signature.focusable.then_some("0"));

    let [left, top, width, height] = signature.rect;
    let style = element.style();
    let _ = style.set_property("left", &format!("{left}px"));
    let _ = style.set_property("top", &format!("{top}px"));
    let _ = style.set_property("width", &format!("{width}px"));
    let _ = style.set_property("height", &format!("{height}px"));
    let _ = style.set_property("font-size", &format!("{}px", (height * 3 / 4).max(1)));
}

fn create(
    document: &web_sys::Document,
    id: NodeId,
    signature: Signature,
    ctx: &egui::Context,
) -> Option<MirroredNode> {
    let tag = if signature.role.is_empty() { "span" } else { "div" };
    let element: web_sys::HtmlElement = document.create_element(tag).ok()?.dyn_into().ok()?;
    let style = element.style();
    let _ = style.set_property("position", "absolute");
    let _ = style.set_property("margin", "0");
    let _ = style.set_property("padding", "0");
    let _ = style.set_property("border", "0");
    let _ = style.set_property("background", "transparent");
    let _ = style.set_property("color", "transparent");
    let _ = style.set_property("white-space", "pre");
    let _ = style.set_property("line-height", "1");
    let _ = style.set_property("overflow", "hidden");

    let on_focus = if signature.focusable {
        let repaint = ctx.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            PENDING_FOCUS.with_borrow_mut(|pending| pending.push(id));
            repaint.request_repaint();
        });
        let _ = element.add_event_listener_with_callback("focus", closure.as_ref().unchecked_ref());
        Some(closure)
    } else {
        None
    };
    apply(&element, &signature);
    Some(MirroredNode {
        element,
        signature,
        _on_focus: on_focus,
    })
}

fn set_attribute(element: &web_sys::HtmlElement, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        let _ = element.set_attribute(name, value);
    } else {
        let _ = element.remove_attribute(name);
    }
}

const fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

const fn is_interactive(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::Link
            | Role::CheckBox
            | Role::RadioButton
            | Role::Switch
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::ComboBox
            | Role::EditableComboBox
            | Role::Slider
            | Role::SpinButton
            | Role::Tab
            | Role::MenuItem
            | Role::MenuItemCheckBox
            | Role::MenuItemRadio
            | Role::ListBoxOption
            | Role::TreeItem
    )
}

const fn aria_role(role: Role) -> Option<&'static str> {
    Some(match role {
        Role::Label | Role::TextRun | Role::Paragraph => "",
        Role::Heading => "heading",
        Role::Button | Role::DefaultButton => "button",
        Role::Link => "link",
        Role::CheckBox => "checkbox",
        Role::RadioButton => "radio",
        Role::Switch => "switch",
        Role::TextInput | Role::MultilineTextInput => "textbox",
        Role::SearchInput => "searchbox",
        Role::ComboBox | Role::EditableComboBox => "combobox",
        Role::Slider => "slider",
        Role::SpinButton => "spinbutton",
        Role::ProgressIndicator => "progressbar",
        Role::Tab => "tab",
        Role::TabList => "tablist",
        Role::TabPanel => "tabpanel",
        Role::Menu => "menu",
        Role::MenuBar => "menubar",
        Role::MenuItem => "menuitem",
        Role::MenuItemCheckBox => "menuitemcheckbox",
        Role::MenuItemRadio => "menuitemradio",
        Role::List => "list",
        Role::ListItem => "listitem",
        Role::ListBox => "listbox",
        Role::ListBoxOption => "option",
        Role::Tree => "tree",
        Role::TreeItem => "treeitem",
        Role::Table | Role::Grid | Role::TreeGrid => "grid",
        Role::Row => "row",
        Role::Cell | Role::GridCell => "gridcell",
        Role::ColumnHeader => "columnheader",
        Role::RowHeader => "rowheader",
        Role::Dialog => "dialog",
        Role::AlertDialog => "alertdialog",
        Role::Alert => "alert",
        Role::Status => "status",
        Role::Toolbar => "toolbar",
        Role::Navigation => "navigation",
        Role::Group => "group",
        Role::RadioGroup => "radiogroup",
        _ => return None,
    })
}
