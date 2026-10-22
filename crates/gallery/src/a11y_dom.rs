//! Prototype: a hidden, accessible DOM layer that mirrors egui's AccessKit tree on wasm
//! (GitHub issue #108, design in `docs/specs/web-accessibility-layer.md`).
//!
//! eframe 0.36's web runner drops `PlatformOutput::accesskit_update`, so this plugin reads
//! it in [`egui::Plugin::output_hook`] first. Each frame it diffs the tree against the
//! elements it already made and only touches the DOM for nodes that changed. Labels become
//! text spans and buttons become `<button>`s, absolutely positioned over the canvas with
//! transparent text, so the browser's find (Ctrl+F) and screen readers see them. A click on
//! a mirrored button is queued and fed back to egui as an AccessKit `Click` action.
//!
//! Gated by the gallery's `web-a11y-dom` feature; wasm only.

use std::cell::RefCell;
use std::collections::HashMap;

use eframe::wasm_bindgen::JsCast as _;
use eframe::wasm_bindgen::closure::Closure;
use egui::accesskit::{self, Action, ActionRequest, NodeId, Role};

/// The id of the container element the layer lives in.
const LAYER_ID: &str = "fluent2-a11y-layer";

/// What the layer mirrors a node as.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Button,
}

/// The parts of a node the DOM shows; an element is rewritten only when this changes.
#[derive(Clone, PartialEq)]
struct Signature {
    kind: Kind,
    text: String,
    rect: [i32; 4],
}

struct Mirrored {
    element: web_sys::HtmlElement,
    signature: Signature,
    // Kept alive for as long as the element is.
    _on_click: Option<Closure<dyn FnMut()>>,
}

#[derive(Default)]
struct Layer {
    root: Option<web_sys::HtmlElement>,
    nodes: HashMap<NodeId, Mirrored>,
}

thread_local! {
    // DOM handles are `!Send`, and `egui::Plugin` must be `Send + Sync`, so the state lives
    // here. wasm runs the app on one thread.
    static LAYER: RefCell<Layer> = RefCell::new(Layer::default());
    static PENDING: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
}

/// The egui plugin. Register it with `ctx.add_plugin(A11yDomPlugin)`.
pub struct A11yDomPlugin;

impl egui::Plugin for A11yDomPlugin {
    fn debug_name(&self) -> &'static str {
        "a11y_dom"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        // eframe's web runner never turns AccessKit on.
        ctx.enable_accesskit();
    }

    fn input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        PENDING.with_borrow_mut(|pending| {
            for target_node in pending.drain(..) {
                input
                    .events
                    .push(egui::Event::AccessKitActionRequest(ActionRequest {
                        action: Action::Click,
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
    /// Creates the container next to the canvas on first use.
    fn root(&mut self) -> Option<web_sys::HtmlElement> {
        if let Some(root) = &self.root {
            return Some(root.clone());
        }
        let document = web_sys::window()?.document()?;
        let root: web_sys::HtmlElement = document.create_element("div").ok()?.dyn_into().ok()?;
        root.set_id(LAYER_ID);
        // Over the canvas, with transparent text and no pointer events, so egui still draws
        // and handles the mouse.
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
        let mut seen = std::collections::HashSet::with_capacity(update.nodes.len());

        for (id, node) in &update.nodes {
            let Some(signature) = signature(node, offset) else {
                continue;
            };
            seen.insert(*id);
            if self.nodes.get(id).is_some_and(|m| m.signature == signature) {
                continue;
            }
            if self
                .nodes
                .get(id)
                .is_some_and(|m| m.signature.kind != signature.kind)
                && let Some(old) = self.nodes.remove(id)
            {
                old.element.remove();
            }
            if let Some(mirrored) = self.nodes.get_mut(id) {
                apply(&mirrored.element, &signature);
                mirrored.signature = signature;
            } else if let Some(mirrored) = create(&document, *id, signature, ctx) {
                let _ = root.append_child(&mirrored.element);
                self.nodes.insert(*id, mirrored);
            }
        }

        // egui sends the whole tree each frame, so anything not in it has gone.
        self.nodes.retain(|id, mirrored| {
            let keep = seen.contains(id);
            if !keep {
                mirrored.element.remove();
            }
            keep
        });
    }
}

/// The canvas's top-left corner in CSS pixels; AccessKit bounds are in egui points, which
/// eframe maps 1:1 to CSS pixels.
fn canvas_offset(document: &web_sys::Document) -> (f64, f64) {
    document
        .get_element_by_id("the_canvas_id")
        .map(|canvas| {
            let rect = canvas.get_bounding_client_rect();
            (rect.left(), rect.top())
        })
        .unwrap_or((0.0, 0.0))
}

fn signature(node: &accesskit::Node, offset: (f64, f64)) -> Option<Signature> {
    let kind = match node.role() {
        Role::Label => Kind::Text,
        Role::Button => Kind::Button,
        _ => return None,
    };
    if node.is_hidden() {
        return None;
    }
    let text = node.label().or_else(|| node.value())?.to_owned();
    if text.is_empty() {
        return None;
    }
    let bounds = node.bounds()?;
    #[expect(clippy::cast_possible_truncation, reason = "screen coordinates fit in i32")]
    let rect = [
        (bounds.x0 + offset.0).round() as i32,
        (bounds.y0 + offset.1).round() as i32,
        bounds.width().round() as i32,
        bounds.height().round() as i32,
    ];
    Some(Signature { kind, text, rect })
}

fn apply(element: &web_sys::HtmlElement, signature: &Signature) {
    element.set_text_content(Some(&signature.text));
    let [x, y, w, h] = signature.rect;
    let style = element.style();
    let _ = style.set_property("left", &format!("{x}px"));
    let _ = style.set_property("top", &format!("{y}px"));
    let _ = style.set_property("width", &format!("{w}px"));
    let _ = style.set_property("height", &format!("{h}px"));
    // Size the text to the box so the browser's find highlight lands on the drawn text.
    let _ = style.set_property("font-size", &format!("{}px", (h * 3 / 4).max(1)));
}

fn create(
    document: &web_sys::Document,
    id: NodeId,
    signature: Signature,
    ctx: &egui::Context,
) -> Option<Mirrored> {
    let tag = match signature.kind {
        Kind::Text => "span",
        Kind::Button => "button",
    };
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
    let on_click = if signature.kind == Kind::Button {
        // Clicks from a screen reader or keyboard (the element is focusable) are routed back
        // as AccessKit actions. Mouse clicks stay on the canvas: the layer has no pointer
        // events, so egui keeps its own hover and press handling.
        let ctx = ctx.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            PENDING.with_borrow_mut(|pending| pending.push(id));
            ctx.request_repaint();
        });
        let _ = element.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        Some(closure)
    } else {
        None
    };
    apply(&element, &signature);
    Some(Mirrored {
        element,
        signature,
        _on_click: on_click,
    })
}
