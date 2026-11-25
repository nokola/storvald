//! The parse tree, one struct per spec "Node anatomy": type, name, styles,
//! flags, args, props, params, children, line. Directives (`style`,
//! `theme`, `when`, `transition`, …) are ordinary nodes here; `compile.rs`
//! gives them meaning.

#[derive(Clone, Debug, PartialEq)]
pub enum AstValue {
    Bool(bool),
    Num { value: f32, unit: String },
    Color { rgba: [u8; 4], opacity: Option<f32>, pos: Option<f32> },
    /// `line` is the string's OWN source line — an `areas=` row string
    /// must point its errors at itself, not at the `areas=` word
    /// (GPAR99).
    Str { text: String, key: Option<String>, line: u32 },
    Ref { name: String, opacity: Option<f32>, pos: Option<f32> },
    /// A named call argument (`arc(from=-135, sweep=270)`) — the grammar
    /// production (GPAR32); which calls READ the names is each call
    /// site's own business (GP40/GP33 track the features).
    Named { name: String, value: Box<AstValue> },
    Ident { word: String, opacity: Option<f32>, pos: Option<f32> },
    /// `animals[12]` — one cell of a grid atlas.
    Index { word: String, index: u32 },
    Size { w: Option<f32>, h: Option<f32> },
    Call { name: String, args: Vec<AstValue> },
    List(Vec<AstValue>),
    /// `a..b` (`in $min..$max`).
    Range(Box<AstValue>, Box<AstValue>),
}

impl AstValue {
    pub fn as_ident(&self) -> Option<&str> {
        match self {
            AstValue::Ident { word, .. } => Some(word),
            _ => None,
        }
    }
    pub fn as_num(&self) -> Option<f32> {
        match self {
            AstValue::Num { value, .. } => Some(*value),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AstValue::Str { text, .. } => Some(text),
            _ => None,
        }
    }
}

/// In `AstNode::origin` and `AstProp::origin`: not said. The compiler
/// reads such a line as one of the file it is working in.
pub const ORIGIN_UNSET: u16 = u16::MAX;

#[derive(Clone, Debug, PartialEq)]
pub struct AstProp {
    pub key: String,
    pub value: AstValue,
    pub line: u32,
    /// Which file `line` is in: 0 = the file that is compiled, k = the
    /// k-th `uses` file the compile read (`Compiled::uses[k - 1]`),
    /// `ORIGIN_UNSET` = not said.
    ///
    /// Needed because a node's properties come from several files (its
    /// own line, a style, the line that uses a widget) and a diagnostic
    /// must name the file its line is in.
    pub origin: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AstNode {
    pub type_word: String,
    pub name: Option<String>,
    pub styles: Vec<String>,
    pub flags: Vec<String>,
    pub args: Vec<AstValue>,
    pub props: Vec<AstProp>,
    /// `$name=value` declarations (widget params).
    pub params: Vec<AstProp>,
    /// `-> endpoint` words (transition rules).
    pub arrows: Vec<String>,
    /// `as $b` / `by $b.field` (repeat lines).
    pub clauses: Vec<(String, AstValue)>,
    pub children: Vec<AstItem>,
    pub line: u32,
    /// Which file `line` is in (`AstProp::origin`).
    pub origin: u16,
}

/// Written by hand so a node the compiler makes up starts with no file
/// said (`ORIGIN_UNSET`), not as a line of the file that is compiled.
impl Default for AstNode {
    fn default() -> AstNode {
        AstNode { type_word: String::new(), name: None, styles: Vec::new(), flags: Vec::new(), args: Vec::new(), props: Vec::new(), params: Vec::new(), arrows: Vec::new(), clauses: Vec::new(), children: Vec::new(), line: 0, origin: ORIGIN_UNSET }
    }
}

impl AstNode {
    /// Says which file this node, its properties and everything written
    /// under it are in.
    fn stamp_origin(&mut self, origin: u16) {
        self.origin = origin;
        for p in self.props.iter_mut().chain(self.params.iter_mut()) {
            p.origin = origin;
        }
        for item in &mut self.children {
            if let AstItem::Node(child) = item {
                child.stamp_origin(origin);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AstItem {
    Node(AstNode),
    /// `$name = value` (theme value) or `node.prop = value` (a `when`
    /// response); `dollar` tells which.
    Assign { key: String, dollar: bool, value: AstValue, line: u32 },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub items: Vec<AstItem>,
}

impl Document {
    /// Says which file every node and property of this document is in
    /// (`AstProp::origin`).
    pub fn stamp_origin(&mut self, origin: u16) {
        for item in &mut self.items {
            if let AstItem::Node(node) = item {
                node.stamp_origin(origin);
            }
        }
    }
}
