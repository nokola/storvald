//! Emits a compiled page as Rust source for `build.rs`: the pools as
//! `static` slices (`Debug` output of the `Copy` page types is valid Rust
//! with the variant globs imported), a `Handler` enum from the file's
//! `on:` names and auto-wired Buttons, `enum Arrangement` and `enum Anim`
//! (typed handles for `Runtime::go` / `play`), and a `Names` struct of
//! node ids — a widget instance is a nested `<Widget>Names` entry
//! (`NAMES.slider_size.track`). Build-wide: `emit_keys` (the translation
//! keys as `KEYS` + `enum Key`) and `emit_widget_names` (one
//! `<Widget>Names` struct per widget the build's pages instance, and one
//! `enum <Widget>Arrangement` per widget that declares arrangements).

use crate::locale::Keys;
use crate::page::*;
use crate::surface::SurfaceDraw;

/// Rust's keywords: a node named `box` or `type` becomes the field
/// `r#box` / `r#type` — the name the file wrote, kept legal.
const KEYWORDS: &[&str] = &["as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen"];

/// `snake_case`, escaped when the result is a Rust keyword (`r#box`).
/// `crate`, `self`, `Self` and `super` cannot be escaped and stay as they
/// are — the emitted struct then fails to compile with rustc's own message,
/// naming the field.
pub fn field_name(name: &str) -> String {
    let s = snake_case(name);
    if KEYWORDS.contains(&s.as_str()) { format!("r#{s}") } else { s }
}

/// `save_button` from `saveButton` / `SaveButton` / `save_button`.
pub fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            out.push(c);
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        }
    }
    out
}

/// `SaveButton` from `saveButton` / `save_button`.
pub fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in name.chars() {
        if c == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Handler names: every `Action::Named`, plus every named Button/Check/
/// Toggle/Radio without an `on:activate` and every named Slider/Input
/// without an `on:change` (the auto-wiring convention: the node's own
/// name is its handler, for `activate` and for `change`/`edit` alike),
/// plus the name of every widget instance with such a node inside it, or
/// with a `behavior` line and no `on:change` (`instance_that_hears`), plus
/// every signal a control on the page declares.
pub fn handler_names(page: &Page) -> Vec<String> {
    handler_names_with(page, &WidgetCode::default())
}

/// `handler_names` for a build where some widgets have code (`code`): a
/// name raised inside an instance of one is that instance's code's, not
/// the page's, and is left out. The instance itself has a variant only
/// when its widget reports (`report_variants`).
pub fn handler_names_with(page: &Page, code: &WidgetCode<'_>) -> Vec<String> {
    let inside: Vec<bool> = inside_code(page, code);
    // Which handlers sit on a node inside an instance with code.
    let mut is_code_handler: Vec<bool> = vec![false; page.handlers.len()];
    for (index, node) in page.nodes.iter().enumerate() {
        if inside[index] {
            for h in node.handlers.0..node.handlers.0 + node.handlers.1 {
                if let Some(flag) = is_code_handler.get_mut(h as usize) {
                    *flag = true;
                }
            }
        }
    }
    let mut names: Vec<String> = Vec::new();
    for (index, h) in page.handlers.iter().enumerate() {
        if is_code_handler[index] {
            continue;
        }
        if let Action::Named(id) | Action::Emit(id) = h.action {
            let n = page.str(id).to_string();
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    // A named `choice` reports its pick to a handler of its own name.
    for c in &page.choices {
        if let Some(Action::Named(id) | Action::Emit(id)) = c.action {
            let n = page.str(id).to_string();
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    let activate = page.intern_lookup("activate");
    let change = page.intern_lookup("change");
    for (index, node) in page.nodes.iter().enumerate() {
        if inside[index] {
            continue;
        }
        let value_control = matches!(node.kind, Kind::Slider | Kind::Input);
        if (node.kind.is_press_surface() || value_control) && node.name != NO_STR && !page.str(node.name).contains('/') {
            let wired = if value_control { change } else { activate };
            let has_activate = page.handlers[node.handlers.0 as usize..(node.handlers.0 + node.handlers.1) as usize].iter().any(|h| h.event == wired);
            if !has_activate {
                let n = page.str(node.name).to_string();
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
    }
    // Instance names come last, so a page's other variants keep their
    // order when a widget is placed on it.
    for (index, node) in page.nodes.iter().enumerate() {
        if inside[index] {
            continue;
        }
        // A control with a `behavior` line reports its value as a Slider does.
        let value_control: bool = matches!(node.kind, Kind::Slider | Kind::Input) || page.behaviors.iter().any(|behavior| behavior.root.index() == index);
        if !(node.kind.is_press_surface() || value_control) {
            continue;
        }
        let wired: StrId = if value_control { change } else { activate };
        let is_heard_by_own_line: bool = page.handlers[node.handlers.0 as usize..(node.handlers.0 + node.handlers.1) as usize].iter().any(|h| h.event == wired);
        if is_heard_by_own_line {
            continue;
        }
        if let Some(instance) = instance_that_hears(page, NodeId::from_index(index)) {
            let n: String = page.str(instance).to_string();
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    // A key a control claims (`keys` line) is reported like its value.
    let key: StrId = page.intern_lookup("key");
    for (root, _) in &page.key_claims {
        if inside[root.index()] {
            continue;
        }
        let node: &Node = &page.nodes[root.index()];
        let is_heard_by_own_line: bool = page.handlers[node.handlers.0 as usize..(node.handlers.0 + node.handlers.1) as usize].iter().any(|h| h.event == key);
        if is_heard_by_own_line {
            continue;
        }
        if let Some(instance) = instance_that_hears(page, *root) {
            let n: String = page.str(instance).to_string();
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    // A control's declared signal is reported under the signal's own
    // name, as an `emit()` is.
    for behavior in &page.behaviors {
        if inside[behavior.root.index()] {
            continue;
        }
        for signal in [behavior.drag_started, behavior.drag_ended] {
            if signal != NO_STR {
                let n: String = page.str(signal).to_string();
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
    }
    names
}

/// Name of the widget instance that answers for `node`, a node that
/// reports with no `on:` line.
///
/// Needed so a page hears a button inside a widget it placed: the event
/// walks up to the nearest name the page's file wrote, and that name
/// needs a `Handler` variant. Without one, a tap on `ctlSave/btnSave`
/// reaches no code of a page that matches on `Handler`.
///
/// Same walk as a host's `handler_name`: a name with `/` belongs to a
/// widget's body and is passed.
///
/// Returns `None` when the nearest such name is a plain node's: only an
/// instance's name is given a variant here.
fn instance_that_hears(page: &Page, node: NodeId) -> Option<StrId> {
    let mut at: NodeId = node;
    loop {
        let here: &Node = &page.nodes[at.index()];
        if here.name != NO_STR && !page.str(here.name).contains('/') {
            return if here.widget != NO_STR { Some(here.name) } else { None };
        }
        if here.parent == NO_NODE {
            return None;
        }
        at = here.parent;
    }
}

fn slice<T: std::fmt::Debug>(items: &[T]) -> String {
    let mut s = String::from("&[");
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("{it:?}"));
    }
    s.push(']');
    s
}

/// The page's theme values as code: `enum Theme` (one variant per `$name`
/// the page sees — its own and its `uses` files'; the discriminant is the
/// name's string id) — `Runtime::theme_color(page, Theme::Accent)` cannot
/// name a value the page lacks.
fn emit_theme(page: &Page, out: &mut String) {
    let mut names: Vec<StrId> = Vec::new();
    for t in &page.theme {
        if !names.contains(&t.name) {
            names.push(t.name);
        }
    }
    if names.is_empty() {
        return;
    }
    out.push_str("    /// The page's theme values (`$name = value`), for `Runtime::theme_color`.\n");
    out.push_str("    #[allow(clippy::enum_variant_names, non_camel_case_types, dead_code)]\n    #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n    #[repr(u32)]\n    pub enum Theme {\n");
    for n in &names {
        out.push_str(&format!("        {} = {},\n", key_variant(page.str(*n)), n.index()));
    }
    out.push_str("    }\n\n");
    out.push_str("    impl gogo_ui::ThemeName for Theme {\n        fn id(self) -> gogo_ui::StrId {\n            gogo_ui::StrId::from_index(self as usize)\n        }\n    }\n\n");
}

/// The page's arrangements as code: `enum Arrangement` (one variant per
/// declared name, `#[repr(u32)]` = the index `when <name>` poses match),
/// plus `Base` (the rest pose) — `Runtime::go(Arrangement::Wide)` cannot
/// name a pose the page lacks (GPAR27).
fn emit_arrangements(page: &Page, out: &mut String) {
    if page.arrangements.is_empty() {
        return;
    }
    out.push_str("    /// The page's declared arrangements; `Base` is the rest pose.\n");
    out.push_str("    #[allow(clippy::enum_variant_names, non_camel_case_types, dead_code)]\n    #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n    #[repr(u32)]\n    pub enum Arrangement {\n");
    for (i, a) in page.arrangements.iter().enumerate() {
        out.push_str(&format!("        {} = {i},\n", key_variant(page.str(*a))));
    }
    out.push_str(&format!("        Base = {},\n", u32::MAX));
    out.push_str("    }\n\n");
    out.push_str("    impl gogo_ui::ArrangementName for Arrangement {\n        fn id(self) -> gogo_ui::ArrangementId {\n            gogo_ui::ArrangementId::new(self as u32)\n        }\n    }\n\n");
}

/// The page's own anims as code: `enum Anim` (one variant per `anim` the
/// file declares at page level, `#[repr(u32)]` = the name's string id)
/// plus the `AnimName` impl — `Runtime::play(page, Anim::Appear, …)`
/// cannot misspell, and nothing interns the name per frame. A widget's
/// anims (`inst/anim`) have their own enum, `<Widget>Anim`
/// (`widget_anims`), and are not listed. `Err` when two names spell one variant (`fadeIn` / `fade_in`).
fn emit_anims(page: &Page, out: &mut String) -> Result<(), String> {
    let mut names: Vec<(String, StrId)> = Vec::new();
    for h in &page.anim_heads {
        let name = page.str(h.anim);
        if name.contains('/') || names.iter().any(|(n, _)| n == name) {
            continue;
        }
        names.push((name.to_string(), h.anim));
    }
    if names.is_empty() {
        return Ok(());
    }
    let mut variants: Vec<String> = Vec::with_capacity(names.len());
    for (name, _) in &names {
        let v = key_variant(name);
        if let Some(j) = variants.iter().position(|x| *x == v) {
            return Err(format!("anims `{}` and `{name}` both become `Anim::{v}` — rename one\n", names[j].0));
        }
        variants.push(v);
    }
    out.push_str("    /// The page's declared anims — `Runtime::play` / `stop` / `is_playing` take one.\n");
    out.push_str("    #[allow(clippy::enum_variant_names, non_camel_case_types, dead_code)]\n    #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n    #[repr(u32)]\n    pub enum Anim {\n");
    for ((_, id), v) in names.iter().zip(&variants) {
        out.push_str(&format!("        {v} = {},\n", id.index()));
    }
    out.push_str("    }\n\n");
    out.push_str("    impl gogo_ui::AnimName for Anim {\n        fn id(self) -> gogo_ui::StrId {\n            gogo_ui::StrId::new(self as u32)\n        }\n    }\n\n");
    Ok(())
}

/// One entry of a `Names` struct: a plain named node, a `Surface` (typed
/// by its `draw=`), or a widget instance (its own struct,
/// `<Widget>Names`).
#[derive(Clone, Debug, PartialEq)]
pub enum NameEntry {
    Node(NodeId),
    /// Needed so code that fills a Surface another way than its file
    /// says does not compile: `Runtime::set_picture` takes a
    /// `PictureSurface`, and the name of a `draw=commands` one is none.
    Surface(NodeId, SurfaceDraw),
    Instance { root: NodeId, widget: String, body: Vec<(String, NameEntry)> },
}

/// The entry of a plain node: a `Surface` by its `draw=`, any other
/// node by its id.
fn node_entry(page: &Page, id: NodeId) -> NameEntry {
    match SurfaceDraw::of(page, id) {
        Some(draw) => NameEntry::Surface(id, draw),
        None => NameEntry::Node(id),
    }
}

/// The Rust type of a Surface's name, as written in a build's code.
fn surface_type(draw: SurfaceDraw) -> &'static str {
    match draw {
        SurfaceDraw::Picture => "gogo_ui::PictureSurface",
        SurfaceDraw::Commands => "gogo_ui::CommandSurface",
        SurfaceDraw::Gpu => "gogo_ui::GpuSurface",
    }
}

/// The Rust struct name for widget `SliderPhone`: `SliderPhoneNames`.
pub fn widget_struct(widget: &str) -> String {
    format!("{}Names", pascal_case(widget))
}

/// The Rust enum name for the arrangements of widget `CompactSwitch`:
/// `CompactSwitchArrangement`.
pub fn widget_arrangement_enum(widget: &str) -> String {
    format!("{}Arrangement", pascal_case(widget))
}

/// A widget's arrangements as code: `enum <Widget>Arrangement` (one
/// variant per `arrangement` line of the widget's body, `#[repr(u32)]` =
/// its place among them, plus `Base`), its `ControlArrangementName` impl,
/// and the `HasArrangements` impl that ties the enum to
/// `<Widget>Names`.
///
/// Needed so `Runtime::go_in(&NAMES.restore_switch,
/// CompactSwitchArrangement::Option1)` cannot name an arrangement the
/// widget lacks, nor hand one widget's arrangement to another widget's
/// instance: both are rustc errors.
///
/// Returns `Err` when two names spell one variant (`fadeIn` /
/// `fade_in`), or a name spells `Base`.
fn widget_arrangements(widget: &str, names: &[String]) -> Result<String, String> {
    let enum_name: String = widget_arrangement_enum(widget);
    let mut variants: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        let variant: String = key_variant(name);
        if variant == "Base" {
            return Err(format!("widget `{widget}`: arrangement `{name}` becomes `{enum_name}::Base`, the name of the widget with no arrangement on — rename it\n"));
        }
        if let Some(at) = variants.iter().position(|v| *v == variant) {
            return Err(format!("widget `{widget}`: arrangements `{}` and `{name}` both become `{enum_name}::{variant}` — rename one\n", names[at]));
        }
        variants.push(variant);
    }
    let mut out = String::new();
    out.push_str(&format!("/// The arrangements `{widget}` declares; `Base` is an instance with none on. Each instance is in one of them on its own (`Runtime::go_in`).\n"));
    out.push_str(&format!("#[allow(clippy::enum_variant_names, non_camel_case_types, dead_code)]\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n#[repr(u32)]\npub enum {enum_name} {{\n"));
    for (at, variant) in variants.iter().enumerate() {
        out.push_str(&format!("    {variant} = {at},\n"));
    }
    out.push_str(&format!("    Base = {},\n}}\n\n", u32::MAX));
    out.push_str(&format!("impl gogo_ui::ControlArrangementName for {enum_name} {{\n    fn id(self) -> gogo_ui::ArrangementId {{\n        gogo_ui::ArrangementId::new(self as u32)\n    }}\n\n    fn from_id(id: gogo_ui::ArrangementId) -> {enum_name} {{\n        match id.index() {{\n"));
    for (at, variant) in variants.iter().enumerate() {
        out.push_str(&format!("            {at} => {enum_name}::{variant},\n"));
    }
    out.push_str(&format!("            _ => {enum_name}::Base,\n        }}\n    }}\n}}\n\n"));
    out.push_str(&format!("impl gogo_ui::HasArrangements for {} {{\n    type Arrangement = {enum_name};\n\n    fn root(&self) -> NodeId {{\n        self.root\n    }}\n}}\n\n", widget_struct(widget)));
    Ok(out)
}

/// The Rust enum name for the anims of widget `CompactSwitch`:
/// `CompactSwitchAnim`.
pub fn widget_anim_enum(widget: &str) -> String {
    format!("{}Anim", pascal_case(widget))
}

/// A widget's anims as code: `enum <Widget>Anim` (one variant per `anim`
/// beside the widget's body, `#[repr(u32)]` = its place among them), its
/// `ControlAnimName` impl, and the `HasAnims` impl that ties the enum to
/// `<Widget>Names`.
///
/// Needed so `Runtime::play_in(page, &NAMES.restore_switch,
/// CompactSwitchAnim::ToastFade, …)` cannot name an anim the widget
/// lacks, nor hand one widget's anim to another widget's instance: both
/// are rustc errors.
///
/// Returns `Err` when two names spell one variant (`fadeIn` /
/// `fade_in`).
fn widget_anims(widget: &str, names: &[String]) -> Result<String, String> {
    let enum_name: String = widget_anim_enum(widget);
    let mut variants: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        let variant: String = key_variant(name);
        if let Some(at) = variants.iter().position(|v| *v == variant) {
            return Err(format!("widget `{widget}`: anims `{}` and `{name}` both become `{enum_name}::{variant}` — rename one\n", names[at]));
        }
        variants.push(variant);
    }
    let mut out = String::new();
    out.push_str(&format!("/// The anims `{widget}` declares. Each instance plays its own copy (`Runtime::play_in`).\n"));
    out.push_str(&format!("#[allow(clippy::enum_variant_names, non_camel_case_types, dead_code)]\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n#[repr(u32)]\npub enum {enum_name} {{\n"));
    for (at, variant) in variants.iter().enumerate() {
        out.push_str(&format!("    {variant} = {at},\n"));
    }
    out.push_str("}\n\n");
    out.push_str(&format!("impl gogo_ui::ControlAnimName for {enum_name} {{\n    fn place(self) -> u32 {{\n        self as u32\n    }}\n}}\n\n"));
    out.push_str(&format!("impl gogo_ui::HasAnims for {} {{\n    type Anim = {enum_name};\n\n    fn root(&self) -> NodeId {{\n        self.root\n    }}\n}}\n\n", widget_struct(widget)));
    Ok(out)
}

/// The `Names` entries of the nodes directly under `prefix` (`""` = the
/// page level; `"inst"` = the body of widget instance `inst`): every named
/// node whose name is `prefix/x` with no further `/`, snake_cased; a
/// widget instance root becomes an `Instance` entry with its body
/// resolved the same way. Unnamed instances (`#1/…`) and repeat rows
/// (`list/item/…`, no widget root) have no entry — nothing in code can
/// address them stably.
fn name_entries(page: &Page, prefix: &str) -> Result<Vec<(String, NameEntry)>, String> {
    let mut out: Vec<(String, NameEntry)> = Vec::new();
    for (i, n) in page.nodes.iter().enumerate() {
        if n.name == NO_STR {
            continue;
        }
        let full = page.str(n.name);
        let local = if prefix.is_empty() {
            full
        } else {
            match full.strip_prefix(prefix).and_then(|r| r.strip_prefix('/')) {
                Some(r) => r,
                None => continue,
            }
        };
        if local.contains('/') || local.starts_with('#') || local.is_empty() {
            continue;
        }
        let field = field_name(local);
        let entry = if n.widget != NO_STR {
            let body = name_entries(page, full)?;
            if body.iter().any(|(f, _)| f == "root") {
                return Err(format!("widget `{}`: a body node named `root` clashes with the instance's `root` field — rename it\n", page.str(n.widget)));
            }
            NameEntry::Instance { root: NodeId::from_index(i), widget: page.str(n.widget).to_string(), body }
        } else {
            node_entry(page, NodeId::from_index(i))
        };
        if let Some((f, other)) = out.iter().find(|(f, _)| *f == field) {
            if *other != entry {
                return Err(format!("nodes `{full}` and another under `{prefix}` both become `NAMES.{f}` — rename one\n"));
            }
            continue;
        }
        out.push((field, entry));
    }
    Ok(out)
}

/// The type a `Names` field has.
fn entry_type(e: &NameEntry) -> String {
    match e {
        NameEntry::Node(_) => "NodeId".to_string(),
        NameEntry::Surface(_, draw) => surface_type(*draw).to_string(),
        NameEntry::Instance { widget, .. } => format!("super::{}", widget_struct(widget)),
    }
}

/// The value a `Names` field has (`indent` = the literal's nesting).
fn entry_value(e: &NameEntry, indent: usize) -> String {
    match e {
        NameEntry::Node(id) => format!("{id:?}"),
        NameEntry::Surface(id, draw) => format!("{}::from_build(gogo_ui::{id:?})", surface_type(*draw)),
        NameEntry::Instance { root, widget, body } => {
            let pad = " ".repeat(indent);
            let mut s = format!("super::{} {{\n{pad}    root: {root:?},\n", widget_struct(widget));
            for (f, e) in body {
                s.push_str(&format!("{pad}    {f}: {},\n", entry_value(e, indent + 4)));
            }
            s.push_str(&format!("{pad}}}"));
            s
        }
    }
}

/// One field of a `<Widget>Names` struct.
#[derive(Clone, Debug, PartialEq)]
struct WidgetRow {
    /// The Rust field (`root_check`).
    field: String,
    /// `NodeId`, a Surface's type (`gogo_ui::PictureSurface`), or
    /// `super::<Widget>Names` for a nested instance.
    ty: String,
    /// The body node's own name in the page (`rootCheck`), the last
    /// segment of its full name — what `bind` looks the node up by.
    local: String,
}

/// The last segment of a node's full name (`control/rootCheck` → `rootCheck`).
fn local_name(page: &Page, id: NodeId) -> String {
    let full = page.str(page.nodes[id.index()].name);
    full.rsplit('/').next().unwrap_or(full).to_string()
}

/// The widget structs a page's instances need: widget → field rows, every
/// instance of one widget agreeing on the rows (they stamp one body).
fn collect_widget_shapes(page: &Page, entries: &[(String, NameEntry)], shapes: &mut Vec<(String, Vec<WidgetRow>)>, arranged: &mut Vec<(String, Vec<String>)>, animated: &mut Vec<(String, Vec<String>)>, where_: &str) -> Result<(), String> {
    for (_, e) in entries {
        if let NameEntry::Instance { root, widget, body } = e {
            // The arrangements the widget declares, read from this
            // instance: every instance of one widget names the same ones.
            if let Some(entry) = page.control_arrangements_of(*root) {
                let names: Vec<String> = page.control_arrangement_names[entry.first as usize..(entry.first + entry.count) as usize].iter().map(|n| page.str(*n).to_string()).collect();
                match arranged.iter().find(|(w, _)| w == widget) {
                    Some((_, have)) if *have != names => {
                        return Err(format!("widget `{widget}`: its instances do not declare the same arrangements (see {where_}) — `{}` cannot be one enum\n", widget_arrangement_enum(widget)));
                    }
                    Some(_) => {}
                    None => arranged.push((widget.clone(), names)),
                }
            }
            // The anims the widget declares, read from this instance's
            // own copies (`<instance>/<anim>`): every instance of one
            // widget lists the same ones.
            if let Some(entry) = page.control_anims_of(*root) {
                let names: Vec<String> = page.control_anim_names[entry.first as usize..(entry.first + entry.count) as usize]
                    .iter()
                    .map(|copy| {
                        let full: &str = page.str(*copy);
                        full.rsplit('/').next().unwrap_or(full).to_string()
                    })
                    .collect();
                match animated.iter().find(|(w, _)| w == widget) {
                    Some((_, have)) if *have != names => {
                        return Err(format!("widget `{widget}`: its instances do not declare the same anims (see {where_}) — `{}` cannot be one enum\n", widget_anim_enum(widget)));
                    }
                    Some(_) => {}
                    None => animated.push((widget.clone(), names)),
                }
            }
            let mut rows: Vec<WidgetRow> = vec![WidgetRow { field: "root".to_string(), ty: "NodeId".to_string(), local: String::new() }];
            for (f, be) in body {
                let id = match be {
                    NameEntry::Node(id) | NameEntry::Surface(id, _) => *id,
                    NameEntry::Instance { root, .. } => *root,
                };
                rows.push(WidgetRow { field: f.clone(), ty: entry_type(be), local: local_name(page, id) });
            }
            match shapes.iter().find(|(w, _)| w == widget) {
                Some((_, have)) if *have != rows => {
                    return Err(format!("widget `{widget}`: its instances do not stamp one body (see {where_}) — `{}Names` cannot be one struct\n", pascal_case(widget)));
                }
                Some(_) => {}
                None => shapes.push((widget.clone(), rows)),
            }
            collect_widget_shapes(page, body, shapes, arranged, animated, where_)?;
        }
    }
    Ok(())
}

/// The build-wide `<Widget>Names` structs: one per widget any page in
/// `pages` instances, at the generated module's top level (`(module, page)`
/// pairs). One struct per widget means a widget's code-behind takes
/// `&SliderPhoneNames` whichever page the instance sits on. Each struct
/// also gets `bind` (the same names over a page compiled at run time —
/// a test's host page) and `rebase` (a repeat slot's copy).
pub fn emit_widget_names(pages: &[(&str, &Page)]) -> Result<String, String> {
    let mut shapes: Vec<(String, Vec<WidgetRow>)> = Vec::new();
    let mut arranged: Vec<(String, Vec<String>)> = Vec::new();
    let mut animated: Vec<(String, Vec<String>)> = Vec::new();
    for (module, page) in pages {
        let entries = name_entries(page, "").map_err(|e| format!("{module}: {e}"))?;
        collect_widget_shapes(page, &entries, &mut shapes, &mut arranged, &mut animated, module)?;
        // A widget placed only inside a repeat's template still needs its
        // struct: the slots are typed through it (`<Repeat>Slot::at`).
        for (_, template) in repeat_templates(page) {
            let entries = name_entries(page, &template).map_err(|e| format!("{module}: {e}"))?;
            collect_widget_shapes(page, &entries, &mut shapes, &mut arranged, &mut animated, module)?;
        }
    }
    let mut out = String::new();
    for (widget, rows) in &shapes {
        out.push_str(&format!("/// The body of one `{widget}` instance, by node id (`root` = the instance itself).\n"));
        out.push_str(&format!("#[allow(dead_code)]\n#[derive(Clone, Copy, Debug)]\npub struct {} {{\n", widget_struct(widget)));
        for r in rows {
            out.push_str(&format!("    pub {}: {},\n", r.field, plain_type(&r.ty)));
        }
        out.push_str("}\n\n");
        out.push_str(&names_impl(&widget_struct(widget), rows, ""));
        if let Some((_, names)) = arranged.iter().find(|(w, _)| w == widget) {
            out.push_str(&widget_arrangements(widget, names)?);
        }
        if let Some((_, names)) = animated.iter().find(|(w, _)| w == widget) {
            out.push_str(&widget_anims(widget, names)?);
        }
    }
    if !shapes.is_empty() {
        out = format!("use gogo_ui::NodeId;\n\n{out}");
    }
    Ok(out)
}

/// A row's type as written beside the struct it names (same module).
fn plain_type(ty: &str) -> &str {
    ty.strip_prefix("super::").unwrap_or(ty)
}

/// `impl <Struct>` with `bind` and `rebase`. `bind(page, instance)` builds
/// the struct over a page compiled at run time (a test's host page: `uses`
/// the widget, places one instance, no build step) — the instance by its
/// name, every body node by its name under it, a nested instance by its
/// own `bind`; `None` when a node is missing (no such instance, or a body
/// that differs from this build's). `rebase(from, to)` moves every field
/// to a cloned subtree (a repeat slot) — a node field by `NodeId::rebase`,
/// an instance field by its own `rebase`. `indent` is the module nesting.
fn names_impl(struct_name: &str, rows: &[WidgetRow], indent: &str) -> String {
    let mut s = format!("{indent}impl {struct_name} {{\n{indent}    /// The same names over the instance `instance` of a page compiled at run time (a test's host page); `None` when a body node is missing.\n{indent}    pub fn bind(page: &gogo_ui::Page, instance: &str) -> Option<{struct_name}> {{\n{indent}        Some({struct_name} {{\n");
    for r in rows {
        if r.field == "root" {
            s.push_str(&format!("{indent}            root: page.find(instance)?,\n"));
        } else if r.ty == "NodeId" {
            s.push_str(&format!("{indent}            {}: page.find(&format!(\"{{instance}}/{}\"))?,\n", r.field, r.local));
        } else if r.ty.starts_with("gogo_ui::") {
            // A Surface: found by name AND by its `draw=`, so a body
            // that fills it another way binds nothing.
            s.push_str(&format!("{indent}            {}: {}::find(page, &format!(\"{{instance}}/{}\"))?,\n", r.field, r.ty, r.local));
        } else {
            s.push_str(&format!("{indent}            {}: {}::bind(page, &format!(\"{{instance}}/{}\"))?,\n", r.field, plain_type(&r.ty), r.local));
        }
    }
    s.push_str(&format!("{indent}        }})\n{indent}    }}\n{indent}}}\n\n"));
    let fields: Vec<(String, String)> = rows.iter().map(|r| (r.field.clone(), r.ty.clone())).collect();
    s.push_str(&rebase_impl(struct_name, &fields, indent));
    s
}

/// `impl <Struct> { pub fn rebase(self, from, to) -> Self }` moving every
/// field to a cloned subtree (a repeat slot) — a node field by
/// `NodeId::rebase`, an instance field by its own `rebase`. `indent` is
/// the module nesting.
fn rebase_impl(struct_name: &str, rows: &[(String, String)], indent: &str) -> String {
    let mut s = format!("{indent}impl {struct_name} {{\n{indent}    /// The same names under a subtree cloned from `from` to `to` (a repeat slot).\n{indent}    pub const fn rebase(self, from: NodeId, to: NodeId) -> {struct_name} {{\n{indent}        {struct_name} {{\n");
    for (f, t) in rows {
        s.push_str(&format!("{indent}            {f}: self.{f}.rebase(from, to),\n"));
        let _ = t;
    }
    s.push_str(&format!("{indent}        }}\n{indent}    }}\n{indent}}}\n\n"));
    s
}

/// Every repeat on the page with its template's name prefix
/// (`<repeat>/item`): (repeat node, prefix).
fn repeat_templates(page: &Page) -> Vec<(NodeId, String)> {
    page.repeats.iter().map(|r| (r.node, format!("{}/item", page.str(page.nodes[r.node].name)))).collect()
}

/// The Rust source of `pub mod <module>`. `Err` for a name clash the
/// generated code could not carry (two anims spelling one variant, two
/// nodes spelling one field, a body node named `root`).
pub fn emit_rust(page: &Page, module: &str) -> Result<String, String> {
    emit_rust_with(page, module, &WidgetCode::default())
}

/// `emit_rust` for a build where some widgets have code (`code`): the
/// page's `Handler` gets a variant per instance with code that reports,
/// and the page gets its `Widgets` struct with the routing to them.
pub fn emit_rust_with(page: &Page, module: &str, code: &WidgetCode<'_>) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(&format!("#[allow(unused_imports, clippy::all)]\npub mod {module} {{\n"));
    out.push_str("    use super::gogo_ui;\n    use gogo_ui::page::*;\n    use gogo_ui::page::{Kind::*, Len::*, Align::*, PropKey::*, Value::*, State::*, Action::*, Auto::*, Payload::*};\n\n");
    emit_arrangements(page, &mut out);
    emit_theme(page, &mut out);
    emit_anims(page, &mut out).map_err(|e| format!("{module}: {e}"))?;
    out.push_str("    pub static PAGE: StaticPage = StaticPage {\n");
    out.push_str(&format!("        nodes: {},\n", slice(&page.nodes)));
    out.push_str(&format!("        strings: {},\n", slice(&page.strings.iter().map(|s| &**s).collect::<Vec<_>>())));
    out.push_str(&format!("        props: {},\n", slice(&page.props)));
    out.push_str(&format!("        values: {},\n", slice(&page.values)));
    out.push_str(&format!("        whens: {},\n", slice(&page.whens)));
    out.push_str(&format!("        handlers: {},\n", slice(&page.handlers)));
    out.push_str(&format!("        theme: {},\n", slice(&page.theme)));
    out.push_str(&format!("        kind_defaults: {},\n", slice(&page.kind_defaults)));
    out.push_str(&format!("        arrangements: {},\n", slice(&page.arrangements)));
    out.push_str(&format!("        control_arrangement_names: {},\n", slice(&page.control_arrangement_names)));
    out.push_str(&format!("        control_arrangements: {},\n", slice(&page.control_arrangements)));
    out.push_str(&format!("        control_anim_names: {},\n", slice(&page.control_anim_names)));
    out.push_str(&format!("        control_anims: {},\n", slice(&page.control_anims)));
    out.push_str(&format!("        custom_states: {},\n", slice(&page.custom_states)));
    out.push_str(&format!("        key_claims: {},\n", slice(&page.key_claims)));
    out.push_str(&format!("        popups: {},\n", slice(&page.popups)));
    out.push_str(&format!("        behaviors: {},\n", slice(&page.behaviors)));
    out.push_str(&format!("        transitions: {},\n", slice(&page.transitions)));
    out.push_str(&format!("        prop_transitions: {},\n", slice(&page.prop_transitions)));
    out.push_str(&format!("        anims: {},\n", slice(&page.anims)));
    out.push_str(&format!("        anim_heads: {},\n", slice(&page.anim_heads)));
    out.push_str(&format!("        choices: {},\n", slice(&page.choices)));
    out.push_str(&format!("        repeats: {},\n", slice(&page.repeats)));
    out.push_str(&format!("        bind_sites: {},\n", slice(&page.bind_sites)));
    out.push_str(&format!("        drivers: {},\n", slice(&page.drivers)));
    out.push_str(&format!("        value_follows: {},\n", slice(&page.value_follows)));
    // The build's one slice table (`build_pages` emits it beside
    // `IMAGES`): every page points at it, so a slice id code sets draws
    // on any page and no page copies it.
    out.push_str("        slices: &super::SLICES,\n");
    out.push_str("    };\n\n");

    // Names struct entries, read first: the instances with code among
    // them give the Handler enum its report variants.
    let entries = name_entries(page, "").map_err(|e| format!("{module}: {e}"))?;
    let mut slots: Vec<CodeSlot> = Vec::new();
    code_slots(&entries, "", "", code, &mut slots);
    let rows: Vec<CodeRows> = code_rows(page, code).map_err(|e| format!("{module}: {e}"))?;
    check_code_reached(page, &entries, code).map_err(|e| format!("{module}: {e}"))?;

    // Handler enum.
    let names = handler_names_with(page, code);
    let reporting: Vec<&CodeSlot> = slots.iter().filter(|slot| slot.has_reports).collect();
    for slot in &reporting {
        if names.iter().any(|n| pascal_case(n) == slot.variant) {
            return Err(format!("{module}: the instance `{}` of widget `{}` reports as `Handler::{}`, and a handler of the file has that name too — rename one\n", slot.names, slot.widget, slot.variant));
        }
    }
    let reporting_rows: Vec<&CodeRows> = rows.iter().filter(|rows| rows.slots.iter().any(|slot| slot.has_reports)).collect();
    for rows in &reporting_rows {
        if names.iter().any(|n| pascal_case(n) == rows.variant) || reporting.iter().any(|slot| slot.variant == rows.variant) {
            return Err(format!("{module}: the rows of `repeat {}` report as `Handler::{}`, and a handler of the file has that name too — rename one\n", rows.repeat_name, rows.variant));
        }
    }
    out.push_str("    /// The file's handler names; the app matches on these. An instance of a widget with code that reports has a variant carrying its report.\n");
    if reporting.is_empty() && reporting_rows.is_empty() {
        out.push_str("    #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n    pub enum Handler {\n");
    } else {
        out.push_str("    #[derive(Clone, Debug, PartialEq)]\n    pub enum Handler {\n");
    }
    for n in &names {
        out.push_str(&format!("        {},\n", pascal_case(n)));
    }
    for slot in &reporting {
        out.push_str(&format!("        {}(<{} as gogo_ui::Widget>::Report),\n", slot.variant, slot.path));
    }
    for rows in &reporting_rows {
        out.push_str(&format!("        /// A report of a widget in row `.0` of `repeat {}`, by the row's place among the rows it made.\n        {}(usize, {}),\n", rows.repeat_name, rows.variant, rows.inside_enum));
    }
    out.push_str("    }\n\n    impl Handler {\n        pub fn from_name(name: &str) -> Option<Handler> {\n            match name {\n");
    for n in &names {
        out.push_str(&format!("                {n:?} => Some(Handler::{}),\n", pascal_case(n)));
    }
    out.push_str("                _ => None,\n            }\n        }\n    }\n\n");
    out.push_str("    impl gogo_ui::HandlerName for Handler {\n");
    if !slots.is_empty() || !rows.is_empty() {
        out.push_str("        const HAS_WIDGET_CODE: bool = true;\n\n");
    }
    out.push_str("        fn from_name(name: &str) -> Option<Handler> {\n            Handler::from_name(name)\n        }\n    }\n\n");

    // Names struct: plain nodes by id; a widget instance as its
    // `<Widget>Names` (root + body), nested for a widget inside a widget.
    out.push_str("    /// Every named node, by id; a widget instance by its `<Widget>Names`.\n    #[allow(dead_code)]\n    #[derive(Clone, Copy, Debug)]\n    pub struct Names {\n");
    for (f, e) in &entries {
        out.push_str(&format!("        pub {f}: {},\n", entry_type(e)));
    }
    out.push_str("    }\n\n    pub const NAMES: Names = Names {\n");
    for (f, e) in &entries {
        out.push_str(&format!("        {f}: {},\n", entry_value(e, 8)));
    }
    out.push_str("    };\n");
    if !slots.is_empty() || !rows.is_empty() {
        out.push_str(&emit_page_widgets(&slots, &rows));
    }
    emit_slots(page, &mut out).map_err(|e| format!("{module}: {e}"))?;
    out.push_str("}\n");
    Ok(out)
}

/// One `<Repeat>Slot` struct per repeat: the template's named parts
/// (`root` = the slot itself) as typed fields, `TEMPLATE` for the hidden
/// template, and `at(slot_root)` for a live row (`Runtime::items`). Needed
/// so a code-behind over a pooled row binds its widgets by field
/// (`BigRowsSlot::at(root).slider`) instead of formatting name paths.
fn emit_slots(page: &Page, out: &mut String) -> Result<(), String> {
    for (repeat, prefix) in repeat_templates(page) {
        let name = page.str(page.nodes[repeat].name);
        let template: NodeId = page.repeats.iter().find(|r| r.node == repeat).map(|r| r.template).expect("a repeat has a template");
        // The template root carries the prefix too (`rows/item/row`): it IS
        // the slot, so it is the `root` field, not a part.
        let is_root = |e: &NameEntry| matches!(e, NameEntry::Node(n) | NameEntry::Surface(n, _) if *n == template) || matches!(e, NameEntry::Instance { root, .. } if *root == template);
        // The row itself, typed like any other name: a row that is a
        // Surface is filled through `root`.
        let root_entry: NameEntry = node_entry(page, template);
        let root_node: &str = if matches!(root_entry, NameEntry::Surface(..)) { ".node()" } else { "" };
        let entries: Vec<(String, NameEntry)> = name_entries(page, &prefix)?.into_iter().filter(|(_, e)| !is_root(e)).collect();
        if entries.iter().any(|(f, _)| f == "root") {
            return Err(format!("repeat `{name}`: a template node named `root` clashes with the slot's `root` field — rename it\n"));
        }
        // A repeat inside a widget instance is `inst/marks`: the path
        // joins into one name (`InstMarksSlot`).
        let struct_name = format!("{}Slot", pascal_case(&name.replace('/', "_")));
        out.push_str(&format!("\n    /// One row of `repeat {name}`: the template's named parts by id (`root` = the row itself).\n    #[allow(dead_code)]\n    #[derive(Clone, Copy, Debug)]\n    pub struct {struct_name} {{\n        pub root: {},\n", entry_type(&root_entry)));
        for (f, e) in &entries {
            out.push_str(&format!("        pub {f}: {},\n", entry_type(e)));
        }
        out.push_str("    }\n\n");
        out.push_str(&format!("    impl {struct_name} {{\n        /// The hidden template the rows are cloned from.\n        pub const TEMPLATE: {struct_name} = {struct_name} {{\n            root: {},\n", entry_value(&root_entry, 12)));
        for (f, e) in &entries {
            out.push_str(&format!("            {f}: {},\n", entry_value(e, 12)));
        }
        out.push_str(&format!("        }};\n\n        /// The row whose root is `slot_root` (one of `Runtime::items`).\n        pub const fn at(slot_root: NodeId) -> {struct_name} {{\n            {struct_name}::TEMPLATE.rebase({struct_name}::TEMPLATE.root{root_node}, slot_root)\n        }}\n    }}\n\n"));
        let mut rows: Vec<(String, String)> = vec![("root".to_string(), entry_type(&root_entry))];
        rows.extend(entries.iter().map(|(f, e)| (f.clone(), entry_type(e))));
        out.push_str(&rebase_impl(&struct_name, &rows, "    "));
    }
    Ok(())
}

/// The Rust variant for translation key `name`: `pickColorHint` →
/// `PickColorHint`; a key that starts with a digit gets `N` in front
/// (`16by9` → `N16by9`), since a variant cannot.
pub fn key_variant(name: &str) -> String {
    // A runtime key may carry a dash or a dot (`fxTile3-half`): every
    // character that cannot sit in an identifier becomes `_`.
    let clean: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    let p = pascal_case(&clean);
    if p.chars().next().is_some_and(|c| c.is_ascii_digit()) { format!("N{p}") } else { p }
}

/// The build's translation keys as code, at the `ui` module's top level:
/// `KEYS: &[(name, English)]` by `KeyId`, `enum Key` (one variant per key,
/// `key_variant` spelling, `#[repr(u32)]` = the id), `Key::from_name` for
/// loading a translation file, and the `gogo_ui::KeyName` impl that lets
/// `Runtime::set_key` take a `Key` and nothing else. `Err` when two names
/// spell the same variant (`fxColor` and `fx_color`) — a build error, not a
/// silent merge.
pub fn emit_keys(keys: &Keys) -> Result<String, String> {
    let mut out = String::new();
    let mut variants: Vec<String> = Vec::with_capacity(keys.names.len());
    for (i, name) in keys.names.iter().enumerate() {
        let mut v = key_variant(name);
        if let Some(j) = variants.iter().position(|x| *x == v) {
            // A runtime key (`BuildOptions::runtime_keys`) is looked up by
            // its spelling, never by variant, so a case-only twin of a
            // page key (`save`/`Save`) takes a numbered variant instead of
            // stopping the build; two PAGE keys colliding is an authoring
            // mistake and stays one.
            if keys.origins[i] == "runtime" {
                let mut n = 2;
                while variants.iter().any(|x| *x == format!("{v}_{n}")) {
                    n += 1;
                }
                v = format!("{v}_{n}");
            } else {
                return Err(format!("translation keys `{}` and `{name}` both become `Key::{v}` — rename one\n", keys.names[j]));
            }
        }
        variants.push(v);
    }
    out.push_str("/// Every translation key a page names, by `KeyId`: (key as written, its English).\n");
    out.push_str("pub static KEYS: &[(&str, &str)] = &[\n");
    for (name, english) in keys.names.iter().zip(&keys.english) {
        out.push_str(&format!("    ({name:?}, {english:?}),\n"));
    }
    out.push_str("];\n\n");
    out.push_str("/// The translation keys as code: `Key::Ok.id()` indexes `KEYS`; a key no page declares does not compile.\n");
    out.push_str("#[allow(clippy::enum_variant_names, non_camel_case_types)]\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n#[repr(u32)]\npub enum Key {\n");
    for (i, v) in variants.iter().enumerate() {
        out.push_str(&format!("    {v} = {i},\n"));
    }
    out.push_str("}\n\n");
    out.push_str("impl Key {\n    /// The key spelled as the `.gogo` files (and translation files) write it.\n    pub fn from_name(name: &str) -> Option<Key> {\n        match name {\n");
    for (name, v) in keys.names.iter().zip(&variants) {
        out.push_str(&format!("            {name:?} => Some(Key::{v}),\n"));
    }
    out.push_str("            _ => None,\n        }\n    }\n\n    pub const fn id(self) -> gogo_ui::KeyId {\n        gogo_ui::KeyId::new(self as u32)\n    }\n}\n\n");
    out.push_str("impl gogo_ui::KeyName for Key {\n    fn id(self) -> gogo_ui::KeyId {\n        gogo_ui::KeyId::new(self as u32)\n    }\n}\n\n");
    Ok(out)
}

/// `pub static SLICES`: every atlas slice of the build, by
/// `ImageKey::slice_index`. Every page's `slices` points here, so the
/// table exists once in the program and no page copies it.
pub fn emit_slices(slices: &[ImageSlice]) -> String {
    let mut out = format!("/// Every atlas slice (`atlas` lines), by `ImageKey::slice_index`: the picture it is cut from and where. Every page points here.\npub static SLICES: [gogo_ui::page::ImageSlice; {}] = [\n", slices.len());
    for s in slices {
        out.push_str(&format!("    gogo_ui::page::ImageSlice {{ picture: gogo_ui::ImageKey::new({}), rect: {:?}, edge: {:?}, untrimmed: {:?} }},\n", s.picture.index(), s.rect, s.edge, s.untrimmed));
    }
    out.push_str("];\n\n");
    out
}

/// One page of a build, as `build_pages` met it: its module, its file
/// (empty for a page with no file), the files it read through `uses` and
/// the `reload::TextStamp` of all of them.
pub struct BuiltPageRow {
    pub module: String,
    pub file: String,
    pub uses: Vec<String>,
    pub text: u64,
}

/// `pub static BUILD`: everything a hot reload needs to know about the
/// build (`reload::BuildTables`), pointing at the tables emitted above it
/// (`KEYS`, `IMAGES`, `SLICES`, every page's `PAGE`).
///
/// Needed so a host hands a reload ONE thing: a table added to a build
/// later reaches every host through this constant, with no host code
/// changed. A program that never reloads does not reference it, so the
/// linker leaves it out.
pub fn emit_build(pages: &[BuiltPageRow], atlases: &[AtlasDecl], behaviors: &[BehaviorSpec], require_keys: bool) -> String {
    let key = |k: &ImageKey| format!("gogo_ui::ImageKey::new({})", k.index());
    let mut out = String::from("/// Everything a hot reload needs to know about this build: hand it whole to `gogo_ui::reload::Reloader::new`.\npub static BUILD: gogo_ui::reload::BuildTables<'static> = gogo_ui::reload::BuildTables {\n    keys: KEYS,\n    images: IMAGES,\n    slices: &SLICES,\n    atlases: &[\n");
    for a in atlases {
        let shape: String = match &a.shape {
            AtlasShape::Grid(keys) => format!("gogo_ui::reload::BuiltShape::Grid(&[{}])", keys.iter().map(key).collect::<Vec<_>>().join(", ")),
            AtlasShape::Named(list) => format!("gogo_ui::reload::BuiltShape::Named(&[{}])", list.iter().map(|(n, k)| format!("({n:?}, {})", key(k))).collect::<Vec<_>>().join(", ")),
            AtlasShape::Whole(k) => format!("gogo_ui::reload::BuiltShape::Whole({})", key(k)),
        };
        out.push_str(&format!("        gogo_ui::reload::BuiltAtlas {{ name: {:?}, said: &[{}], shape: {shape} }},\n", a.name, a.said.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")));
    }
    out.push_str("    ],\n");
    out.push_str("    behaviors: &[\n");
    for b in behaviors {
        out.push_str(&format!("        gogo_ui::page::BehaviorSpec {{ name: {:?}, slots: &{:?}, keys: {}, payload: gogo_ui::Payload::{:?} }},\n", b.name, b.slots, b.keys, b.payload));
    }
    out.push_str(&format!("    ],\n    require_keys: {require_keys},\n    pages: &[\n"));
    for p in pages {
        out.push_str(&format!("        gogo_ui::reload::BuiltPage {{ file: {:?}, page: &{}::PAGE, uses: &{:?}, text: {} }},\n", p.file, p.module, p.uses, p.text));
    }
    out.push_str("    ],\n};\n\n");
    out
}

/// `pub mod atlas`: the build's atlases as code, spelled the way the
/// `.gogo` files spell them — a grid is an array (`ANIMALS[12]` is
/// `animals[12]`), named slices a module (`toolbar::PLAY` is
/// `toolbar.play`), a whole picture one id (`BUBBLE` is `bubble`). `Err`
/// when two atlas names become the same Rust name.
///
/// Needed because code picks slices too (a sticker page fills its tiles
/// from a table), and a typed constant is found by grep and checked by
/// the compiler, where a string lookup would fail at run time.
pub fn emit_atlases(atlases: &[AtlasDecl]) -> Result<String, String> {
    let mut out = String::from("/// The build's atlases (`atlas` lines), named as the `.gogo` files name them.\n#[allow(dead_code, unused_imports)]\npub mod atlas {\n    use super::gogo_ui;\n    use gogo_ui::ImageKey;\n\n");
    let mut taken: Vec<(String, &str)> = Vec::new();
    let key = |k: &ImageKey| format!("ImageKey::new({})", k.index());
    for a in atlases {
        let from = if a.origin.is_empty() { String::new() } else { format!(" ({})", a.origin) };
        let rust_name: String = match &a.shape {
            AtlasShape::Named(_) => field_name(&a.name),
            _ => snake_case(&a.name).to_ascii_uppercase(),
        };
        if let Some((_, other)) = taken.iter().find(|(n, _)| n.trim_start_matches("r#").eq_ignore_ascii_case(rust_name.trim_start_matches("r#"))) {
            return Err(format!("atlases `{other}` and `{}` both become `atlas::{rust_name}` in code — rename one\n", a.name));
        }
        taken.push((rust_name.clone(), &a.name));
        match &a.shape {
            AtlasShape::Grid(keys) => {
                out.push_str(&format!("    /// `atlas {}`{from}: {} cells, `{}[i]` in a `.gogo` file.\n", a.name, keys.len(), a.name));
                out.push_str(&format!("    pub const {rust_name}: [ImageKey; {}] = [{}];\n\n", keys.len(), keys.iter().map(key).collect::<Vec<_>>().join(", ")));
            }
            AtlasShape::Whole(k) => {
                out.push_str(&format!("    /// `atlas {}`{from}: the whole picture, `source={}` in a `.gogo` file.\n", a.name, a.name));
                out.push_str(&format!("    pub const {rust_name}: ImageKey = {};\n\n", key(k)));
            }
            AtlasShape::Named(list) => {
                out.push_str(&format!("    /// `atlas {}`{from}: `{}.<slice>` in a `.gogo` file.\n    pub mod {rust_name} {{\n        use super::gogo_ui;\n        use gogo_ui::ImageKey;\n", a.name, a.name));
                let mut consts: Vec<String> = Vec::new();
                for (sname, k) in list {
                    let c = snake_case(sname).to_ascii_uppercase();
                    if consts.contains(&c) {
                        return Err(format!("atlas `{}`: two slices become `atlas::{rust_name}::{c}` in code — rename one\n", a.name));
                    }
                    out.push_str(&format!("        pub const {c}: ImageKey = {};\n", key(k)));
                    consts.push(c);
                }
                out.push_str("    }\n\n");
            }
        }
    }
    out.push_str("}\n\n");
    Ok(out)
}

/// What a build knows of the widgets that have code: each one's struct,
/// as the app's build script names it (`BuildOptions::widget_code`), and
/// the names of its `report` lines (`Compiled::widget_reports`).
#[derive(Clone, Copy, Default)]
pub struct WidgetCode<'a> {
    /// (widget, path of its struct: `crate::ui::code::Lamp`).
    pub structs: &'a [(&'a str, &'a str)],
    /// (widget, its `report` names), for every widget the build declared.
    pub reports: &'a [(String, Vec<String>)],
}

impl WidgetCode<'_> {
    /// The path of `widget`'s struct, when the widget has code.
    pub fn path(&self, widget: &str) -> Option<&str> {
        self.structs.iter().find(|(w, _)| *w == widget).map(|(_, path)| *path)
    }

    /// The names of `widget`'s `report` lines, empty when it has none.
    pub fn reports_of(&self, widget: &str) -> &[String] {
        self.reports.iter().find(|(w, _)| w == widget).map(|(_, reports)| reports.as_slice()).unwrap_or(&[])
    }
}

/// One instance with code that a level (a page, or a widget's body)
/// routes to.
#[derive(Clone, Debug)]
struct CodeSlot {
    /// The field of the level's `Widgets` (`brush_options`, or
    /// `panel_slider` for one inside an instance without code).
    field: String,
    /// Its variant in the level's `Handler` or `<Widget>Inside`.
    variant: String,
    widget: String,
    /// The path of the widget's struct.
    path: String,
    /// The instance's names below the level's names (`brush_options`,
    /// `panel.slider`).
    names: String,
    has_reports: bool,
}

/// The instances with code among `entries`, a level's names: each one is
/// a slot; the body of an instance without code is searched, and a slot
/// found there is named by its path (`panel_slider`).
fn code_slots(entries: &[(String, NameEntry)], field_prefix: &str, names_prefix: &str, code: &WidgetCode<'_>, out: &mut Vec<CodeSlot>) {
    for (field, entry) in entries {
        let NameEntry::Instance { widget, body, .. } = entry else { continue };
        let field_here: String = if field_prefix.is_empty() { field.trim_start_matches("r#").to_string() } else { format!("{field_prefix}_{}", field.trim_start_matches("r#")) };
        let names_here: String = if names_prefix.is_empty() { field.clone() } else { format!("{names_prefix}.{field}") };
        match code.path(widget) {
            Some(path) => out.push(CodeSlot { variant: pascal_case(&field_here), field: field_here, widget: widget.clone(), path: path.to_string(), names: names_here, has_reports: !code.reports_of(widget).is_empty() }),
            None => code_slots(body, &field_here, &names_here, code, out),
        }
    }
}

/// For each node of `page`: whether it is an instance of a widget with
/// code, or inside one.
fn inside_code(page: &Page, code: &WidgetCode<'_>) -> Vec<bool> {
    let mut inside: Vec<bool> = vec![false; page.nodes.len()];
    if code.structs.is_empty() {
        return inside;
    }
    for index in 0..page.nodes.len() {
        let mut at: NodeId = NodeId::from_index(index);
        loop {
            let here: &Node = &page.nodes[at.index()];
            if here.widget != NO_STR && code.path(page.str(here.widget)).is_some() {
                inside[index] = true;
                break;
            }
            if here.parent == NO_NODE {
                break;
            }
            at = here.parent;
        }
    }
    inside
}

/// `Err` naming an instance of a widget with code that no routing
/// reaches: an instance with no name, or one in a repeat's row. Its code
/// would never hear a tap.
fn check_code_reached(page: &Page, entries: &[(String, NameEntry)], code: &WidgetCode<'_>) -> Result<(), String> {
    if code.structs.is_empty() {
        return Ok(());
    }
    let mut reached: Vec<NodeId> = Vec::new();
    fn walk(entries: &[(String, NameEntry)], code: &WidgetCode<'_>, reached: &mut Vec<NodeId>) {
        for (_, entry) in entries {
            if let NameEntry::Instance { root, widget, body } = entry {
                if code.path(widget).is_some() {
                    reached.push(*root);
                }
                walk(body, code, reached);
            }
        }
    }
    walk(entries, code, &mut reached);
    let inside: Vec<bool> = inside_code(page, code);
    for (repeat, prefix) in repeat_templates(page) {
        if !inside[repeat.index()] {
            if let Ok(row) = row_entries(page, repeat, &prefix) {
                walk(&row, code, &mut reached);
            }
        }
    }
    for (index, node) in page.nodes.iter().enumerate() {
        if node.widget == NO_STR || code.path(page.str(node.widget)).is_none() || reached.contains(&NodeId::from_index(index)) {
            continue;
        }
        let name: &str = if node.name == NO_STR { "(no name)" } else { page.str(node.name) };
        return Err(format!("the instance `{name}` of widget `{}` has code, and no routing reaches it: give it a name; in a repeat's row, put it inside a node of the row (a row that is itself the widget is not routed), and keep the repeat out of a widget with code\n", page.str(node.widget)));
    }
    Ok(())
}

/// The routing loop of a level: walks up from `event.node`, and keeps the
/// outermost slot whose root it passes (a slot inside a slot is the outer
/// slot's to route). `names` is how the level reaches its names (`NAMES`,
/// `self.names`); `stop` ends the walk at the level's own root, or `None`
/// for a page.
fn route_body(slots: &[CodeSlot], names: &str, stop: Option<&str>, heard: &dyn Fn(&CodeSlot) -> String, indent: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("{indent}let mut at: gogo_ui::NodeId = event.node;\n"));
    s.push_str(&format!("{indent}let mut slot: usize = usize::MAX;\n"));
    s.push_str(&format!("{indent}while at != gogo_ui::NO_NODE && at.index() < page.nodes.len() {{\n"));
    if let Some(stop) = stop {
        s.push_str(&format!("{indent}    if at == {stop} {{\n{indent}        break;\n{indent}    }}\n"));
    }
    for (index, slot) in slots.iter().enumerate() {
        s.push_str(&format!("{indent}    if at == {names}.{}.root {{\n{indent}        slot = {index};\n{indent}    }}\n", slot.names));
    }
    s.push_str(&format!("{indent}    at = page.nodes[at.index()].parent;\n{indent}}}\n"));
    s.push_str(&format!("{indent}match slot {{\n"));
    for (index, slot) in slots.iter().enumerate() {
        s.push_str(&format!("{indent}    {index} => {{\n{indent}        gogo_ui::Widget::route(&mut self.{}, page, rt, event, &mut |report| {});\n{indent}        true\n{indent}    }}\n", slot.field, heard(slot)));
    }
    s.push_str(&format!("{indent}    _ => false,\n{indent}}}\n"));
    s
}

/// The rest of a `WidgetSet` impl: `load`, `frame` and `after_layout` of
/// every slot, in the file's order.
fn set_forwards(slots: &[CodeSlot], indent: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("{indent}fn load(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime) {{\n"));
    for slot in slots {
        s.push_str(&format!("{indent}    gogo_ui::Widget::load_all(&mut self.{}, page, rt);\n", slot.field));
    }
    s.push_str(&format!("{indent}}}\n\n"));
    s.push_str(&format!("{indent}fn frame(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, dt_sec: f32) -> bool {{\n{indent}    let mut is_moving: bool = false;\n"));
    for slot in slots {
        s.push_str(&format!("{indent}    is_moving |= gogo_ui::Widget::frame_all(&mut self.{}, page, rt, dt_sec);\n", slot.field));
    }
    s.push_str(&format!("{indent}    is_moving\n{indent}}}\n\n"));
    s.push_str(&format!("{indent}fn after_layout(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, rects: &[gogo_ui::Rect]) {{\n"));
    for slot in slots {
        s.push_str(&format!("{indent}    gogo_ui::Widget::after_layout_all(&mut self.{}, page, rt, rects);\n", slot.field));
    }
    s.push_str(&format!("{indent}}}\n"));
    s
}

/// A page's `Widgets`: one field per instance with code, made from the
/// page's `NAMES`, and its `WidgetSet<Handler>` (the routing).
fn emit_page_widgets(slots: &[CodeSlot], rows: &[CodeRows]) -> String {
    let mut s = String::new();
    for row in rows {
        s.push_str(&emit_row_widgets(row));
    }
    s.push_str("\n    /// The page's instances of widgets with code, one field each, and one struct per row of each repeat with such widgets in its rows. The page's code owns it, calls an instance directly, and hands it the page's events (`gogo_ui::WidgetSet::route`).\n    pub struct Widgets {\n");
    for slot in slots {
        s.push_str(&format!("        pub {}: {},\n", slot.field, slot.path));
    }
    for row in rows {
        s.push_str(&format!("        /// One per row `repeat {}` made, in the order they were made.\n        pub {}: gogo_ui::Rows<{}>,\n", row.repeat_name, row.field, row.row_struct));
    }
    s.push_str("    }\n\n    impl Widgets {\n        /// Every instance over its names; nothing is written into the page until `load`.\n        pub fn new() -> Widgets {\n            Widgets {\n");
    for slot in slots {
        s.push_str(&format!("                {}: <{} as gogo_ui::Widget>::new(&NAMES.{}),\n", slot.field, slot.path, slot.names));
    }
    for row in rows {
        s.push_str(&format!("                {}: gogo_ui::Rows::new(),\n", row.field));
    }
    s.push_str("            }\n        }\n\n");
    s.push_str("        /// Makes and loads the struct of every row made since the last call. The routing calls it at load, before each event and after each layout; a page that made rows and writes into their widgets at once calls it first.\n        pub fn sync_rows(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime) {\n");
    for row in rows {
        s.push_str(&format!("            self.{}.sync(page, rt, NAMES.{}, |root| {}::new(&{}::at(root)), |row, page, rt| gogo_ui::WidgetSet::load(row, page, rt));\n", row.field, row.repeat_names, row.row_struct, row.slot_struct));
    }
    let _ = rows;
    s.push_str("        }\n    }\n\n    impl Default for Widgets {\n        fn default() -> Widgets {\n            Widgets::new()\n        }\n    }\n\n");
    s.push_str("    impl gogo_ui::WidgetSet<Handler> for Widgets {\n        fn route(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, event: &gogo_ui::Event, heard: &mut dyn FnMut(Handler)) -> bool {\n");
    s.push_str("            self.sync_rows(page, rt);\n");
    let heard = |slot: &CodeSlot| if slot.has_reports { format!("heard(Handler::{}(report))", slot.variant) } else { "match report {}".to_string() };
    s.push_str(&route_body_with_rows(slots, rows, &heard, "            "));
    s.push_str("        }\n\n");
    s.push_str(&set_forwards_with_rows(slots, rows, "        "));
    s.push_str("    }\n");
    s
}

/// A repeat at page level whose rows hold widgets with code.
#[derive(Clone, Debug)]
struct CodeRows {
    /// The repeat's name in the file (`bigRows`, `zoom/list`).
    repeat_name: String,
    /// Its field of the page's `Widgets` (`big_rows`).
    field: String,
    /// Its variant of the page's `Handler` (`BigRows`).
    variant: String,
    /// How the page's names reach the repeat node (`big_rows`, `zoom.list`).
    repeat_names: String,
    /// The build's struct of one row's names (`BigRowsSlot`).
    slot_struct: String,
    /// One row's widgets with code (`BigRowsWidgets`).
    row_struct: String,
    /// The reports of one row's widgets, by instance (`BigRowsInside`).
    inside_enum: String,
    /// `.node()` when the row's root is a Surface, else nothing.
    root_suffix: String,
    /// The instances with code of one row, below the row's names.
    slots: Vec<CodeSlot>,
}

/// The names of one row of `repeat` below the row's root (`prefix` is
/// `<repeat>/item`): the root is the row itself, the `root` field of its
/// `<Repeat>Slot`, never one of these.
fn row_entries(page: &Page, repeat: NodeId, prefix: &str) -> Result<Vec<(String, NameEntry)>, String> {
    let template: NodeId = page.repeats.iter().find(|r| r.node == repeat).map(|r| r.template).expect("a repeat has a template");
    Ok(name_entries(page, prefix)?.into_iter().filter(|(_, e)| !matches!(e, NameEntry::Node(n) | NameEntry::Surface(n, _) if *n == template) && !matches!(e, NameEntry::Instance { root, .. } if *root == template)).collect())
}

/// The repeats of `page`, outside every instance with code, whose rows
/// hold instances with code.
fn code_rows(page: &Page, code: &WidgetCode<'_>) -> Result<Vec<CodeRows>, String> {
    let mut out: Vec<CodeRows> = Vec::new();
    if code.structs.is_empty() {
        return Ok(out);
    }
    let inside: Vec<bool> = inside_code(page, code);
    for (repeat, prefix) in repeat_templates(page) {
        if inside[repeat.index()] {
            continue;
        }
        let template: NodeId = page.repeats.iter().find(|r| r.node == repeat).map(|r| r.template).expect("a repeat has a template");
        let entries: Vec<(String, NameEntry)> = row_entries(page, repeat, &prefix)?;
        let mut slots: Vec<CodeSlot> = Vec::new();
        code_slots(&entries, "", "", code, &mut slots);
        if slots.is_empty() {
            continue;
        }
        let name: String = page.str(page.nodes[repeat].name).to_string();
        let joined: String = name.replace('/', "_");
        let pascal: String = pascal_case(&joined);
        let root_suffix: String = if matches!(node_entry(page, template), NameEntry::Surface(..)) { ".node()".to_string() } else { String::new() };
        out.push(CodeRows { field: snake_case(&joined), variant: pascal.clone(), repeat_names: name.split('/').map(field_name).collect::<Vec<String>>().join("."), slot_struct: format!("{pascal}Slot"), row_struct: format!("{pascal}Widgets"), inside_enum: format!("{pascal}Inside"), root_suffix, repeat_name: name, slots });
    }
    Ok(out)
}

/// One row's struct of a repeat with widgets with code: `<Repeat>Inside`,
/// `<Repeat>Widgets` over the row's names, and its routing.
fn emit_row_widgets(row: &CodeRows) -> String {
    let mut s = String::new();
    s.push_str(&format!("\n    /// The reports of the widgets with code in one row of `repeat {}`, by instance.\n    #[allow(dead_code)]\n    #[derive(Clone, Debug, PartialEq)]\n    pub enum {} {{\n", row.repeat_name, row.inside_enum));
    for slot in row.slots.iter().filter(|slot| slot.has_reports) {
        s.push_str(&format!("        {}(<{} as gogo_ui::Widget>::Report),\n", slot.variant, slot.path));
    }
    s.push_str("    }\n\n");
    s.push_str(&format!("    /// The widgets with code in one row of `repeat {}`, and the row's names.\n    pub struct {} {{\n        pub names: {},\n", row.repeat_name, row.row_struct, row.slot_struct));
    for slot in &row.slots {
        s.push_str(&format!("        pub {}: {},\n", slot.field, slot.path));
    }
    s.push_str(&format!("    }}\n\n    impl {} {{\n        /// Every instance of the row `names` names.\n        pub fn new(names: &{}) -> {} {{\n            {} {{\n                names: *names,\n", row.row_struct, row.slot_struct, row.row_struct, row.row_struct));
    for slot in &row.slots {
        s.push_str(&format!("                {}: <{} as gogo_ui::Widget>::new(&names.{}),\n", slot.field, slot.path, slot.names));
    }
    s.push_str("            }\n        }\n    }\n\n");
    s.push_str(&format!("    impl gogo_ui::WidgetSet<{}> for {} {{\n        fn route(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, event: &gogo_ui::Event, heard: &mut dyn FnMut({})) -> bool {{\n", row.inside_enum, row.row_struct, row.inside_enum));
    let inside_enum: String = row.inside_enum.clone();
    let heard = move |slot: &CodeSlot| if slot.has_reports { format!("heard({inside_enum}::{}(report))", slot.variant) } else { "match report {}".to_string() };
    let stop: String = format!("self.names.root{}", row.root_suffix);
    s.push_str(&route_body(&row.slots, "self.names", Some(&stop), &heard, "            "));
    s.push_str("        }\n\n");
    s.push_str(&set_forwards(&row.slots, "        "));
    s.push_str("    }\n");
    s
}

/// The page's routing loop with rows: as `route_body`, and a row root it
/// passes picks that row's struct.
fn route_body_with_rows(slots: &[CodeSlot], rows: &[CodeRows], heard: &dyn Fn(&CodeSlot) -> String, indent: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("{indent}let mut at: gogo_ui::NodeId = event.node;\n"));
    s.push_str(&format!("{indent}let mut slot: usize = usize::MAX;\n"));
    s.push_str(&format!("{indent}let mut row: usize = 0;\n"));
    s.push_str(&format!("{indent}while at != gogo_ui::NO_NODE && at.index() < page.nodes.len() {{\n"));
    for (index, slot) in slots.iter().enumerate() {
        s.push_str(&format!("{indent}    if at == NAMES.{}.root {{\n{indent}        slot = {index};\n{indent}    }}\n", slot.names));
    }
    for (place, rows_here) in rows.iter().enumerate() {
        let index: usize = slots.len() + place;
        s.push_str(&format!("{indent}    if let Some(found) = self.{}.iter().position(|one| one.names.root{} == at) {{\n{indent}        slot = {index};\n{indent}        row = found;\n{indent}    }}\n", rows_here.field, rows_here.root_suffix));
    }
    s.push_str(&format!("{indent}    at = page.nodes[at.index()].parent;\n{indent}}}\n"));
    s.push_str(&format!("{indent}let _ = row;\n"));
    s.push_str(&format!("{indent}match slot {{\n"));
    for (index, slot) in slots.iter().enumerate() {
        s.push_str(&format!("{indent}    {index} => {{\n{indent}        gogo_ui::Widget::route(&mut self.{}, page, rt, event, &mut |report| {});\n{indent}        true\n{indent}    }}\n", slot.field, heard(slot)));
    }
    for (place, rows_here) in rows.iter().enumerate() {
        let index: usize = slots.len() + place;
        let reports: bool = rows_here.slots.iter().any(|slot| slot.has_reports);
        let to_page: String = if reports { format!("heard(Handler::{}(row, inside))", rows_here.variant) } else { "match inside {}".to_string() };
        s.push_str(&format!("{indent}    {index} => gogo_ui::WidgetSet::route(&mut self.{}[row], page, rt, event, &mut |inside| {to_page}),\n", rows_here.field));
    }
    s.push_str(&format!("{indent}    _ => false,\n{indent}}}\n"));
    s
}

/// The rest of the page's `WidgetSet` impl with rows: the slots and every
/// row. `load` and `after_layout` first make the rows made since the last
/// call; `frame` does not: a row made since the last layout is made after
/// the next one, and its first `frame` comes the frame after.
fn set_forwards_with_rows(slots: &[CodeSlot], rows: &[CodeRows], indent: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("{indent}fn load(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime) {{\n"));
    for slot in slots {
        s.push_str(&format!("{indent}    gogo_ui::Widget::load_all(&mut self.{}, page, rt);\n", slot.field));
    }
    s.push_str(&format!("{indent}    self.sync_rows(page, rt);\n{indent}}}\n\n"));
    s.push_str(&format!("{indent}fn frame(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, dt_sec: f32) -> bool {{\n{indent}    let mut is_moving: bool = false;\n"));
    for slot in slots {
        s.push_str(&format!("{indent}    is_moving |= gogo_ui::Widget::frame_all(&mut self.{}, page, rt, dt_sec);\n", slot.field));
    }
    for row in rows {
        s.push_str(&format!("{indent}    for one in self.{}.iter_mut() {{\n{indent}        is_moving |= gogo_ui::WidgetSet::frame(one, page, rt, dt_sec);\n{indent}    }}\n", row.field));
    }
    s.push_str(&format!("{indent}    is_moving\n{indent}}}\n\n"));
    s.push_str(&format!("{indent}fn after_layout(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, rects: &[gogo_ui::Rect]) {{\n{indent}    self.sync_rows(page, rt);\n"));
    for slot in slots {
        s.push_str(&format!("{indent}    gogo_ui::Widget::after_layout_all(&mut self.{}, page, rt, rects);\n", slot.field));
    }
    for row in rows {
        s.push_str(&format!("{indent}    for one in self.{}.iter_mut() {{\n{indent}        gogo_ui::WidgetSet::after_layout(one, page, rt, rects);\n{indent}    }}\n", row.field));
    }
    s.push_str(&format!("{indent}}}\n"));
    s
}

/// Per widget with code that the build's pages place: its `WidgetShape`
/// (its names, the instances with code inside it and their routing, as
/// `<Widget>Widgets` and `<Widget>Inside`), and a check that its report
/// enum has exactly the widget's `report` names. `Err` for a widget the
/// build script names that no compile declared.
pub fn emit_widget_code(pages: &[(&str, &Page)], code: &WidgetCode<'_>) -> Result<String, String> {
    let mut out = String::new();
    for (widget, path) in code.structs {
        if !code.reports.iter().any(|(w, _)| w == widget) {
            return Err(format!("`widget_code` names widget `{widget}` ({path}), and no page or `uses` file declares it\n"));
        }
        // The widget's body names, read from its first instance.
        let mut body: Option<Vec<(String, NameEntry)>> = None;
        for (module, page) in pages {
            let entries = name_entries(page, "").map_err(|e| format!("{module}: {e}"))?;
            if let Some(found) = find_body(&entries, widget) {
                body = Some(found);
                break;
            }
            for (_, template) in repeat_templates(page) {
                let entries = name_entries(page, &template).map_err(|e| format!("{module}: {e}"))?;
                if let Some(found) = find_body(&entries, widget) {
                    body = Some(found);
                    break;
                }
            }
            if body.is_some() {
                break;
            }
        }
        // A widget no page places has no `<Widget>Names`: nothing to write.
        let Some(body) = body else { continue };
        let mut slots: Vec<CodeSlot> = Vec::new();
        code_slots(&body, "", "", code, &mut slots);
        let names_struct: String = widget_struct(widget);
        let inside_enum: String = format!("{}Inside", pascal_case(widget));
        let set_struct: String = format!("{}Widgets", pascal_case(widget));
        let pascal: String = pascal_case(widget);
        if slots.is_empty() {
            out.push_str(&format!("impl gogo_ui::WidgetShape for {path} {{\n    type Names = {names_struct};\n    type InsideReport = gogo_ui::NothingInside;\n\n    fn route_inside(&mut self, _page: &mut gogo_ui::Page, _rt: &mut gogo_ui::Runtime, _event: &gogo_ui::Event, _heard: &mut dyn FnMut(gogo_ui::NothingInside)) -> bool {{\n        false\n    }}\n\n    fn load_inside(&mut self, _page: &mut gogo_ui::Page, _rt: &mut gogo_ui::Runtime) {{}}\n\n    fn frame_inside(&mut self, _page: &mut gogo_ui::Page, _rt: &mut gogo_ui::Runtime, _dt_sec: f32) -> bool {{\n        false\n    }}\n\n    fn after_layout_inside(&mut self, _page: &mut gogo_ui::Page, _rt: &mut gogo_ui::Runtime, _rects: &[gogo_ui::Rect]) {{}}\n}}\n\n"));
        } else {
            let reporting: Vec<&CodeSlot> = slots.iter().filter(|slot| slot.has_reports).collect();
            out.push_str(&format!("/// The reports of the instances with code inside `{widget}`, by instance.\n#[allow(dead_code)]\n#[derive(Clone, Debug, PartialEq)]\npub enum {inside_enum} {{\n"));
            for slot in &reporting {
                out.push_str(&format!("    {}(<{} as gogo_ui::Widget>::Report),\n", slot.variant, slot.path));
            }
            out.push_str("}\n\n");
            out.push_str(&format!("/// The instances with code inside one `{widget}`: the widget's struct keeps it in a field named `widgets`.\npub struct {set_struct} {{\n    names: {names_struct},\n"));
            for slot in &slots {
                out.push_str(&format!("    pub {}: {},\n", slot.field, slot.path));
            }
            out.push_str(&format!("}}\n\nimpl {set_struct} {{\n    /// Every instance inside the one `names` names.\n    pub fn new(names: &{names_struct}) -> {set_struct} {{\n        {set_struct} {{\n            names: *names,\n"));
            for slot in &slots {
                out.push_str(&format!("            {}: <{} as gogo_ui::Widget>::new(&names.{}),\n", slot.field, slot.path, slot.names));
            }
            out.push_str("        }\n    }\n}\n\n");
            out.push_str(&format!("impl gogo_ui::WidgetSet<{inside_enum}> for {set_struct} {{\n    fn route(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, event: &gogo_ui::Event, heard: &mut dyn FnMut({inside_enum})) -> bool {{\n"));
            let heard = |slot: &CodeSlot| if slot.has_reports { format!("heard({inside_enum}::{}(report))", slot.variant) } else { "match report {}".to_string() };
            out.push_str(&route_body(&slots, "self.names", Some("self.names.root"), &heard, "        "));
            out.push_str("    }\n\n");
            out.push_str(&set_forwards(&slots, "    "));
            out.push_str("}\n\n");
            out.push_str(&format!("impl gogo_ui::WidgetShape for {path} {{\n    type Names = {names_struct};\n    type InsideReport = {inside_enum};\n\n    fn route_inside(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, event: &gogo_ui::Event, heard: &mut dyn FnMut({inside_enum})) -> bool {{\n        gogo_ui::WidgetSet::route(&mut self.widgets, page, rt, event, heard)\n    }}\n\n    fn load_inside(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime) {{\n        gogo_ui::WidgetSet::load(&mut self.widgets, page, rt)\n    }}\n\n    fn frame_inside(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, dt_sec: f32) -> bool {{\n        gogo_ui::WidgetSet::frame(&mut self.widgets, page, rt, dt_sec)\n    }}\n\n    fn after_layout_inside(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime, rects: &[gogo_ui::Rect]) {{\n        gogo_ui::WidgetSet::after_layout(&mut self.widgets, page, rt, rects)\n    }}\n}}\n\n"));
        }
        // The report enum has exactly the file's `report` names: a variant
        // missing here, or one the file lacks, fails this match.
        let alias: String = format!("{pascal}ReportOfFile");
        out.push_str(&format!("/// `{widget}`'s report enum, for the check below.\n#[allow(dead_code)]\ntype {alias} = <{path} as gogo_ui::Widget>::Report;\n\n"));
        out.push_str(&format!("/// `{widget}`'s report enum has exactly the `report` names of its file.\n#[allow(dead_code)]\nconst _: fn(&{alias}) = |report| match report {{\n"));
        for report in code.reports_of(widget) {
            out.push_str(&format!("    {alias}::{} {{ .. }} => {{}}\n", key_variant(report)));
        }
        out.push_str("};\n\n");
    }
    Ok(out)
}

/// The body names of the first instance of `widget` among `entries`,
/// searched in the bodies of other instances too.
fn find_body(entries: &[(String, NameEntry)], widget: &str) -> Option<Vec<(String, NameEntry)>> {
    for (_, entry) in entries {
        if let NameEntry::Instance { widget: w, body, .. } = entry {
            if w == widget {
                return Some(body.clone());
            }
            if let Some(found) = find_body(body, widget) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile;

    #[test]
    fn keys_are_code() {
        let mut k = Keys::default();
        k.intern("ok", "OK", "").unwrap();
        k.intern("16by9", "16:9", "").unwrap();
        k.intern("pickColorHint", "Pick a color", "").unwrap();
        let src = emit_keys(&k).unwrap();
        assert!(src.contains("pub static KEYS: &[(&str, &str)] = &[\n    (\"ok\", \"OK\"),\n    (\"16by9\", \"16:9\"),\n    (\"pickColorHint\", \"Pick a color\"),\n];"), "{src}");
        assert!(src.contains("pub enum Key {\n    Ok = 0,\n    N16by9 = 1,\n    PickColorHint = 2,\n}"), "{src}");
        assert!(src.contains("\"16by9\" => Some(Key::N16by9),"), "{src}");
        assert!(src.contains("impl gogo_ui::KeyName for Key"), "{src}");
    }

    /// GPAR27: a page with arrangements emits their typed handle —
    /// `enum Arrangement { Wide = 0, Base }` + the `ArrangementName`
    /// impl — so `Runtime::go(Arrangement::Wide)` cannot misspell.
    #[test]
    fn arrangements_emit_a_typed_enum() {
        let c = compile("arrangement wide\nView v { Panel p width=10 }\nwhen wide { p.width = 20 }\n", &|_| None);
        let src = emit_rust(&c.page, "t").unwrap();
        assert!(src.contains("pub enum Arrangement"), "{src}");
        assert!(src.contains("Wide = 0"), "{src}");
        assert!(src.contains("impl gogo_ui::ArrangementName for Arrangement"), "{src}");
        let c = compile("View v { Panel p width=10 }", &|_| None);
        assert!(!emit_rust(&c.page, "t").unwrap().contains("enum Arrangement"), "no arrangements, no enum");
        let mut clash = Keys::default();
        clash.intern("fxColor", "a", "").unwrap();
        clash.intern("fx_color", "b", "").unwrap();
        let err = emit_keys(&clash).unwrap_err();
        assert!(err.contains("`fxColor` and `fx_color` both become `Key::FxColor`"), "{err}");
    }

    #[test]
    fn names_and_handlers_are_code() {
        let c = compile("View v { Button saveButton text=\"Save\"; Button b text=\"x\", on:activate=clear_all }", &|_| None);
        let src = emit_rust(&c.page, "main").unwrap();
        assert!(src.contains("pub enum Handler {\n        ClearAll,\n        SaveButton,\n    }"), "{src}");
        assert!(src.contains("pub save_button: NodeId"));
        assert!(src.contains("kind: Button"));
        assert!(src.contains("strings: &[\"\", \"dark\", \"View\", \"v\""), "the well-known variant name is interned on every page (GPAR111)");
    }

    /// Handler names of `page_lines` after `widgets`, sorted.
    fn sorted_handler_names(widgets: &str, page_lines: &str) -> Vec<String> {
        let c = compile(&format!("{widgets}View v {{\n    Column c {{\n{page_lines}    }}\n}}\n"), &|_| None);
        assert!(!crate::has_errors(&c.diags), "{:#?}", c.diags);
        let mut names: Vec<String> = handler_names(&c.page);
        names.sort();
        names
    }

    /// A button inside a widget is heard under the name of the instance:
    /// `Prompt ask` gives `Handler::Ask`, and the event's node says which
    /// button. A widget whose buttons each name a handler gives none, nor
    /// does one with nothing to press. A widget inside a widget reports
    /// under the outer instance, the name the page's file wrote.
    #[test]
    fn an_instance_with_a_button_inside_gets_a_handler_name() {
        const WIDGETS: &str = "widget Prompt {\n    Column {\n        Label title text=\"Keep?\"\n        Button keep text=\"Keep\"\n        Button drop text=\"Drop\"\n    }\n}\nwidget Wired {\n    Column {\n        Button go text=\"Go\", on:activate=emit(WiredGo)\n    }\n}\nwidget Plate {\n    Column {\n        Label words text=\"x\"\n    }\n}\nwidget Outer {\n    Column {\n        Prompt inner\n    }\n}\n";
        assert_eq!(sorted_handler_names(WIDGETS, "        Prompt ask\n"), ["ask"]);
        assert_eq!(sorted_handler_names(WIDGETS, "        Wired wired\n"), ["WiredGo"]);
        assert_eq!(sorted_handler_names(WIDGETS, "        Plate plate\n"), [""; 0]);
        assert_eq!(sorted_handler_names(WIDGETS, "        Outer outer\n"), ["outer"]);
        assert_eq!(sorted_handler_names(WIDGETS, "        Prompt ask\n        Prompt again\n        Button own text=\"Own\"\n"), ["again", "ask", "own"]);
        assert_eq!(sorted_handler_names(WIDGETS, "        Button text=\"No name\"\n"), [""; 0], "a button with no name under a plain node names no handler: `c` is no instance");
        let c = compile(&format!("{WIDGETS}View v {{\n    Column c {{\n        Prompt ask\n    }}\n}}\n"), &|_| None);
        let src = emit_rust(&c.page, "t").unwrap();
        assert!(src.contains("pub enum Handler {\n        Ask,\n    }"), "{src}");
        assert!(src.contains("\"ask\" => Some(Handler::Ask),"), "{src}");
    }

    /// A control with a `behavior` line reports its value under the name
    /// of the instance, as a Slider does under its own, and each signal
    /// it declares under the signal's name. With `on:change=` written on
    /// the instance, the value goes to that name alone.
    #[test]
    fn a_control_with_a_behavior_gets_a_handler_name_and_its_signals() {
        const FADER: &str = "control Fader $value=0, $min=0, $max=100 {\n    behavior slider by $value in $min..$max\n    state_builtin held\n    signal drag_started, drag_ended\n    Layers {\n        slot track width=fill, height=24 {\n            slot thumb automatic=position by $value in $min..$max { Circle 12x12 }\n        }\n    }\n}\ncontrol Quiet $value=0, $min=0, $max=100 {\n    behavior slider by $value in $min..$max\n    Layers {\n        slot track width=fill, height=24 {\n            slot thumb automatic=position by $value in $min..$max { Circle 12x12 }\n        }\n    }\n}\n";
        assert_eq!(sorted_handler_names(FADER, "        Fader gain value=25\n"), ["drag_ended", "drag_started", "gain"]);
        assert_eq!(sorted_handler_names(FADER, "        Quiet low value=25\n"), ["low"]);
        assert_eq!(sorted_handler_names(FADER, "        Quiet low value=25, on:change=LowChanged\n"), ["LowChanged"]);
    }

    /// A control that claims keys and takes its input in code reports a
    /// claimed key under the name of the instance.
    #[test]
    fn a_control_that_claims_keys_gets_a_handler_name() {
        const KNOB: &str = "control Knob {\n    keys enter\n    Layers width=40, height=40 {\n        Rectangle plate width=fill, height=fill, color=#333333FF, interactive=true, on:pointer=KnobPointer\n    }\n}\ncontrol Still {\n    Layers width=40, height=40 {\n        Rectangle plate width=fill, height=fill, color=#333333FF, interactive=true, on:pointer=StillPointer\n    }\n}\n";
        assert_eq!(sorted_handler_names(KNOB, "        Knob knob\n"), ["KnobPointer", "knob"]);
        assert_eq!(sorted_handler_names(KNOB, "        Still still\n"), ["StillPointer"]);
    }

    /// A page's anims are a typed handle too: `enum Anim { Appear = <str id> }`
    /// + `AnimName`, so `rt.play(page, Anim::Appear, …)` cannot misspell;
    /// a widget's own anims (`inst/x`) stay out.
    #[test]
    fn anims_emit_a_typed_enum() {
        let c = compile("View v { Panel p width=10, opacity=1 }\nanim appear { p.opacity from=0, to=1, duration=0.2 }\nanim slideOut { p.translate.x to=10, duration=0.1 }\n", &|_| None);
        assert!(!crate::has_errors(&c.diags), "{:#?}", c.diags);
        let src = emit_rust(&c.page, "t").unwrap();
        let appear = c.page.intern_lookup("appear");
        assert!(src.contains(&format!("pub enum Anim {{\n        Appear = {},\n        SlideOut = {},\n    }}", appear.index(), c.page.intern_lookup("slideOut").index())), "{src}");
        assert!(src.contains("impl gogo_ui::AnimName for Anim"), "{src}");
        assert!(!emit_rust(&compile("View v { Panel p width=10 }", &|_| None).page, "t").unwrap().contains("enum Anim"), "no anims, no enum");
    }

    /// A repeat's template becomes `<Repeat>Slot`: the hidden template's
    /// parts as typed fields with `TEMPLATE` and `at(slot_root)`, so a
    /// code-behind over a pooled row never formats a name path; a widget
    /// placed only inside the template still gets its build-wide struct.
    #[test]
    fn repeat_templates_emit_slot_names() {
        let src = "widget Bar { Column { Label caption text=\"c\"; Rectangle track width=fill, height=4 } }\nView v { Column c { repeat rows capacity=2 { Layers row { Bar vol; Label note text=\"n\" } } } }";
        let c = compile(src, &|_| None);
        assert!(!crate::has_errors(&c.diags), "{:#?}", c.diags);
        let code = emit_rust(&c.page, "t").unwrap();
        assert!(code.contains("pub struct RowsSlot {\n        pub root: NodeId,\n        pub vol: super::BarNames,\n        pub note: NodeId,\n    }"), "{code}");
        let template = c.page.repeats[0].template;
        assert_eq!(c.page.str(c.page.nodes[template].name), "rows/item/row", "the template root carries the prefix");
        assert!(code.contains(&format!("pub const TEMPLATE: RowsSlot = RowsSlot {{\n            root: {template:?},")), "{code}");
        assert!(code.contains("pub const fn at(slot_root: NodeId) -> RowsSlot"), "{code}");
        assert!(code.contains("impl RowsSlot {\n        /// The same names under a subtree cloned"), "{code}");
        let structs = emit_widget_names(&[("t", &c.page)]).unwrap();
        assert!(structs.contains("pub struct BarNames"), "a template-only widget still gets its struct: {structs}");
        assert!(structs.contains("impl BarNames {\n    /// The same names under a subtree cloned"), "{structs}");
        // The rebase arithmetic the generated `at` runs: a cloned slot's
        // node is the template's node moved by the roots' distance.
        let mut page = c.page.clone();
        let mut rt = crate::Runtime::new(&page);
        rt.set_count(&mut page, c.page.find("rows").unwrap(), 2);
        let slots = rt.items(&page, c.page.find("rows").unwrap());
        let track = c.page.find("rows/item/vol/track").unwrap();
        assert_eq!(track.rebase(template, slots[1]), page.find("rows/1/vol/track").unwrap());
    }

    /// A widget instance is a nested names struct: `NAMES.vol.track` is the
    /// instance's body node, `NAMES.vol.root` the instance; one
    /// `<Widget>Names` struct per widget for the whole build, so a
    /// code-behind takes the struct whichever page the instance sits on.
    #[test]
    fn widget_instances_emit_nested_names() {
        let src_a = "widget Bar { Column { Label caption text=\"c\"; Rectangle track width=fill, height=4 } }\nView v { Bar vol; Bar tone; Button ok text=\"k\" }";
        let src_b = "widget Bar { Column { Label caption text=\"c\"; Rectangle track width=fill, height=4 } }\nView w { Bar depth }";
        let a = compile(src_a, &|_| None);
        let b = compile(src_b, &|_| None);
        assert!(!crate::has_errors(&a.diags), "{:#?}", a.diags);
        let src = emit_rust(&a.page, "a").unwrap();
        assert!(src.contains("pub vol: super::BarNames,"), "{src}");
        assert!(src.contains("pub tone: super::BarNames,"), "{src}");
        assert!(src.contains("pub ok: NodeId,"), "{src}");
        let vol = a.page.find("vol").unwrap();
        let track = a.page.find("vol/track").unwrap();
        assert!(src.contains(&format!("vol: super::BarNames {{\n            root: {vol:?},\n            caption: {:?},\n            track: {track:?},\n        }}", a.page.find("vol/caption").unwrap())), "{src}");
        let structs = emit_widget_names(&[("a", &a.page), ("b", &b.page)]).unwrap();
        assert_eq!(structs.matches("pub struct BarNames").count(), 1, "one struct for the build: {structs}");
        assert!(structs.contains("pub struct BarNames {\n    pub root: NodeId,\n    pub caption: NodeId,\n    pub track: NodeId,\n}"), "{structs}");
        // A body node named `root` cannot share the struct with the instance field.
        let clash = compile("widget Bar { Column { Label root text=\"c\" } }\nView v { Bar vol }", &|_| None);
        assert!(emit_rust(&clash.page, "v").unwrap_err().contains("named `root`"));
    }

    #[test]
    fn case_helpers() {
        assert_eq!(snake_case("saveButton"), "save_button");
        assert_eq!(snake_case("save_button"), "save_button");
        assert_eq!(field_name("box"), "r#box");
        assert_eq!(field_name("type"), "r#type");
        assert_eq!(field_name("caption"), "caption");
        assert_eq!(snake_case("HTTPServer2x"), "httpserver2x");
        assert_eq!(pascal_case("clear_all"), "ClearAll");
        assert_eq!(pascal_case("saveButton"), "SaveButton");
    }
}
