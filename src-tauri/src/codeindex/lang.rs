//! The languages of the code index and what tree-sitter cuts each into: symbols (functions,
//! classes, methods, fields, Markdown headings…) and references (imports, calls, inheritance).
//!
//! The references are named as tokensave's extractors name them (github.com/aovestdipaperino/tokensave,
//! commit 580936b, MIT, © 2025 Enzo Lombardi): the call sites of `src/extraction/rust_extractor.rs`
//! (`extract_call_sites`, `extract_calls_in_token_tree`), `typescript_extractor.rs`,
//! `python_extractor.rs`, `go_extractor.rs`, `java_extractor.rs`, `csharp_extractor.rs`
//! (`extract_invocation_name`) and `php_extractor.rs`, and the names of their `Use` nodes; the
//! typed receivers are in `receivers`.

use super::receivers;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
    Svelte,
    Python,
    Go,
    Java,
    CSharp,
    Php,
    Markdown,
}

impl Lang {
    /// The language of a file, by its extension.
    pub fn of(path: &str) -> Option<Lang> {
        let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
        Some(match ext.as_str() {
            "rs" => Lang::Rust,
            "ts" | "mts" | "cts" => Lang::TypeScript,
            "tsx" => Lang::Tsx,
            "js" | "mjs" | "cjs" | "jsx" => Lang::JavaScript,
            "svelte" => Lang::Svelte,
            "py" | "pyi" => Lang::Python,
            "go" => Lang::Go,
            "java" => Lang::Java,
            "cs" => Lang::CSharp,
            "php" => Lang::Php,
            "md" | "markdown" => Lang::Markdown,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Lang::Rust => "rust",
            Lang::TypeScript => "typescript",
            Lang::Tsx => "tsx",
            Lang::JavaScript => "javascript",
            Lang::Svelte => "svelte",
            Lang::Python => "python",
            Lang::Go => "go",
            Lang::Java => "java",
            Lang::CSharp => "csharp",
            Lang::Php => "php",
            Lang::Markdown => "markdown",
        }
    }

    fn grammar(self) -> tree_sitter::Language {
        match self {
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            // TypeScript's grammar reads plain JavaScript too, and the scripts of a Svelte component.
            Lang::TypeScript | Lang::JavaScript => {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Svelte => tree_sitter_svelte_ng::LANGUAGE.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::Go => tree_sitter_go::LANGUAGE.into(),
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
            Lang::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Lang::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Lang::Markdown => tree_sitter_md::LANGUAGE.into(),
        }
    }
}

/// A definition found in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sym {
    pub name: String,
    pub kind: &'static str,
    /// The names of what holds it, `.`-joined (`Type.Inner`); empty at the top.
    pub parent: String,
    /// First and last line, from 1.
    pub line: u32,
    pub end_line: u32,
    pub sig: String,
    pub doc: String,
    /// `pub`, `crate`, `super` or `private`, read as tokensave's extractors read it (ESC-296).
    pub vis: &'static str,
    /// The type a field calls the methods of (Rust), for `a.field.method()`; empty else.
    pub ty: String,
}

/// A reference: `import`, `call`, `extends` or `implements`, from the symbol that holds it (its index
/// in `Parsed::symbols`; none at the file's top) to a name, as tokensave's extractors write it
/// (`obj.method`, `a::b`, `Type::method`…), resolved later to a symbol (`resolve`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: Option<usize>,
    pub kind: &'static str,
    pub target: String,
    pub line: u32,
    pub col: u32,
    /// 0: as written; 1: `Type::method`, the receiver's type known; 2: `field::method`, the type of
    /// the field to find (`receivers`).
    pub via: u8,
}

#[derive(Debug, Default)]
pub struct Parsed {
    pub symbols: Vec<Sym>,
    pub edges: Vec<Edge>,
}

/// Cuts `src`. A file the grammar cannot read gives what was read of it.
pub fn parse(lang: Lang, rel_path: &str, src: &str) -> Parsed {
    let mut parser = Parser::new();
    if parser.set_language(&lang.grammar()).is_err() {
        return Parsed::default();
    }
    let Some(tree) = parser.parse(src, None) else {
        return Parsed::default();
    };
    let mut w = Walker {
        lang,
        src: src.as_bytes(),
        out: Parsed::default(),
        line_off: 0,
        stack: Vec::new(),
    };
    if lang == Lang::Svelte {
        let stem = rel_path
            .rsplit('/')
            .next()
            .unwrap_or(rel_path)
            .trim_end_matches(".svelte");
        w.out.symbols.push(Sym {
            name: stem.to_string(),
            kind: "component",
            parent: String::new(),
            line: 1,
            end_line: src.lines().count().max(1) as u32,
            sig: format!("<{stem}>"),
            doc: String::new(),
            vis: "pub",
            ty: String::new(),
        });
        w.svelte(tree.root_node(), 0);
    } else {
        w.walk(tree.root_node(), None, 0);
    }
    w.out
}

/// What holds the symbols found under it.
struct Frame {
    name: String,
    kind: &'static str,
    /// A type (class, struct, trait, impl…): a function in it is a method.
    typed: bool,
}

struct Walker<'a> {
    lang: Lang,
    src: &'a [u8],
    out: Parsed,
    /// Lines to add: the script of a Svelte component starts further down its file.
    line_off: u32,
    stack: Vec<Frame>,
}

/// The deepest tree walked: deeper is generated code.
const MAX_DEPTH: usize = 160;

const IDENTS: &[&str] = &[
    "identifier",
    "type_identifier",
    "property_identifier",
    "field_identifier",
    "name",
    "variable_name",
    "shorthand_property_identifier",
    "namespace_name",
    "private_property_identifier",
];

struct Def {
    kind: &'static str,
    name: String,
    /// Replaces the holder's name (a Go method's receiver).
    parent: Option<String>,
    typed: bool,
}

impl<'a> Walker<'a> {
    fn text(&self, n: Node) -> &'a str {
        n.utf8_text(self.src).unwrap_or("")
    }

    fn line(&self, n: Node) -> u32 {
        n.start_position().row as u32 + 1 + self.line_off
    }

    fn walk(&mut self, node: Node, owner: Option<usize>, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let mut owner = owner;
        let mut pushed = false;
        if let Some(def) = self.def(node) {
            if !def.name.is_empty() {
                let parent = def.parent.clone().unwrap_or_else(|| self.parent_path());
                let idx = self.out.symbols.len();
                let ty = if def.kind == "field" && self.lang == Lang::Rust {
                    node.child_by_field_name("type")
                        .and_then(|t| receivers::rust_field_type(self.text(t)))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                self.out.symbols.push(Sym {
                    name: def.name.clone(),
                    kind: def.kind,
                    parent,
                    line: self.line(node),
                    end_line: node.end_position().row as u32 + 1 + self.line_off,
                    sig: self.signature(node),
                    doc: self.doc(node),
                    vis: self.visibility(node, &def.name),
                    ty,
                });
                self.heritage(node, idx);
                self.typed_calls(node, def.kind, idx);
                owner = Some(idx);
                self.stack.push(Frame {
                    name: def.name,
                    kind: def.kind,
                    typed: def.typed,
                });
                pushed = true;
            }
        } else if self.lang == Lang::Rust && node.kind() == "token_tree" {
            self.token_tree(node, owner);
            return;
        } else if let Some(names) = self.call(node) {
            let line = self.line(node);
            let col = node.start_position().column as u32;
            for target in names {
                self.out.edges.push(Edge {
                    from: owner,
                    kind: "call",
                    target,
                    line,
                    col,
                    via: 0,
                });
            }
        } else if let Some(names) = self.import(node) {
            let line = self.line(node);
            let col = node.start_position().column as u32;
            for target in names {
                self.out.edges.push(Edge {
                    from: owner,
                    kind: "import",
                    target,
                    line,
                    col,
                    via: 0,
                });
            }
            return;
        }
        let mut cur = node.walk();
        for child in node.named_children(&mut cur) {
            self.walk(child, owner, depth + 1);
        }
        if pushed {
            self.stack.pop();
        }
    }

    fn parent_path(&self) -> String {
        self.stack
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    fn in_type(&self) -> bool {
        self.stack.last().is_some_and(|f| f.typed)
    }

    /// The name of the nearest holder of this kind (the `impl` or class `self` is).
    fn enclosing(&self, kind: &str) -> Option<String> {
        self.stack
            .iter()
            .rev()
            .find(|f| f.kind == kind)
            .map(|f| f.name.clone())
    }

    /// The calls of a function whose receiver's type is known, as `Type::method` (`receivers`).
    fn typed_calls(&mut self, node: Node, kind: &str, idx: usize) {
        if !matches!(kind, "function" | "method") {
            return;
        }
        let mut found = Vec::new();
        match self.lang {
            Lang::Rust if node.kind() == "function_item" => {
                let own = self.enclosing("impl");
                receivers::rust(self.src, node, own.as_deref(), &mut found);
            }
            Lang::TypeScript | Lang::Tsx | Lang::JavaScript => {
                let func = if node.kind() == "variable_declarator" {
                    match node.child_by_field_name("value") {
                        Some(v) => v,
                        None => return,
                    }
                } else {
                    node
                };
                let own = self.enclosing("class");
                receivers::ts(self.src, func, own.as_deref(), &mut found);
            }
            Lang::Python => {
                let own = self.enclosing("class");
                receivers::python(self.src, node, own.as_deref(), &mut found);
            }
            _ => {}
        }
        for t in found {
            self.out.edges.push(Edge {
                from: Some(idx),
                kind: "call",
                target: t.name,
                line: t.line + self.line_off,
                col: t.col,
                via: t.via,
            });
        }
    }

    /// `rust_extractor.rs::extract_calls_in_token_tree`: in a macro's arguments, an identifier
    /// followed by a token tree is a call.
    fn token_tree(&mut self, node: Node, owner: Option<usize>) {
        let mut cur = node.walk();
        let kids: Vec<Node> = node.children(&mut cur).collect();
        let mut i = 0;
        while i < kids.len() {
            let c = kids[i];
            if c.kind() == "identifier" && kids.get(i + 1).is_some_and(|n| n.kind() == "token_tree")
            {
                self.out.edges.push(Edge {
                    from: owner,
                    kind: "call",
                    target: self.text(c).to_string(),
                    line: self.line(c),
                    col: c.start_position().column as u32,
                    via: 0,
                });
                self.token_tree(kids[i + 1], owner);
                i += 2;
                continue;
            }
            if c.kind() == "token_tree" {
                self.token_tree(c, owner);
            } else if c.kind() == "macro_invocation" {
                self.walk(c, owner, 0);
            }
            i += 1;
        }
    }

    /// The scripts of a Svelte component are read as TypeScript, the tags of other components used
    /// in its markup count as calls of them.
    fn svelte(&mut self, node: Node, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        match node.kind() {
            "script_element" => {
                let mut cur = node.walk();
                for child in node.named_children(&mut cur) {
                    if child.kind() == "raw_text" {
                        let code = self.text(child).to_string();
                        let mut p = Parser::new();
                        if p.set_language(&Lang::TypeScript.grammar()).is_err() {
                            continue;
                        }
                        let Some(tree) = p.parse(&code, None) else {
                            continue;
                        };
                        let mut inner = Walker {
                            lang: Lang::TypeScript,
                            src: code.as_bytes(),
                            out: Parsed::default(),
                            line_off: self.line_off + child.start_position().row as u32,
                            stack: Vec::new(),
                        };
                        inner.walk(tree.root_node(), None, 0);
                        let base = self.out.symbols.len();
                        self.out.symbols.extend(inner.out.symbols);
                        for mut e in inner.out.edges {
                            e.from = match e.from {
                                None => Some(0),
                                Some(i) => Some(i + base),
                            };
                            self.out.edges.push(e);
                        }
                    }
                }
                return;
            }
            "tag_name" => {
                let name = self.text(node);
                if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    self.out.edges.push(Edge {
                        from: Some(0),
                        kind: "call",
                        target: name.to_string(),
                        line: self.line(node),
                        col: node.start_position().column as u32,
                        via: 0,
                    });
                }
            }
            _ => {}
        }
        let mut cur = node.walk();
        for child in node.named_children(&mut cur) {
            self.svelte(child, depth + 1);
        }
    }

    // ---- definitions ----

    fn field_text(&self, n: Node, field: &str) -> String {
        n.child_by_field_name(field)
            .map(|c| self.text(c).to_string())
            .unwrap_or_default()
    }

    /// The first identifier under `n` (a field declared by a declarator…).
    fn first_ident(&self, n: Node, depth: usize) -> String {
        if IDENTS.contains(&n.kind()) {
            return self.text(n).to_string();
        }
        if depth > 5 {
            return String::new();
        }
        let mut cur = n.walk();
        for c in n.named_children(&mut cur) {
            let found = self.first_ident(c, depth + 1);
            if !found.is_empty() {
                return found;
            }
        }
        String::new()
    }

    /// The last name of a path or a type (`a::b::C<T>` is `C`).
    fn last_ident(&self, n: Node) -> String {
        if IDENTS.contains(&n.kind()) {
            return self.text(n).to_string();
        }
        for f in ["name", "field", "property", "attribute", "function", "type"] {
            if let Some(c) = n.child_by_field_name(f) {
                let r = self.last_ident(c);
                if !r.is_empty() {
                    return r;
                }
            }
        }
        let mut cur = n.walk();
        let kids: Vec<Node> = n.named_children(&mut cur).collect();
        for c in kids.into_iter().rev() {
            let r = self.last_ident(c);
            if !r.is_empty() {
                return r;
            }
        }
        String::new()
    }

    fn def(&self, n: Node) -> Option<Def> {
        let k = n.kind();
        let named = |kind: &'static str, typed: bool| -> Option<Def> {
            Some(Def {
                kind,
                name: self.field_text(n, "name"),
                parent: None,
                typed,
            })
        };
        let func = || if self.in_type() { "method" } else { "function" };
        match self.lang {
            Lang::Rust => match k {
                "function_item" | "function_signature_item" => named(func(), false),
                "struct_item" | "union_item" => named("struct", true),
                "enum_item" => named("enum", true),
                "trait_item" => named("trait", true),
                "type_item" => named("type", false),
                "const_item" | "static_item" => named("const", false),
                "mod_item" => named("module", false),
                "macro_definition" => named("macro", false),
                "field_declaration" => named("field", false),
                "impl_item" => Some(Def {
                    kind: "impl",
                    name: n
                        .child_by_field_name("type")
                        .map(|t| self.last_ident(t))
                        .unwrap_or_default(),
                    parent: None,
                    typed: true,
                }),
                _ => None,
            },
            Lang::TypeScript | Lang::Tsx | Lang::JavaScript => match k {
                "function_declaration" | "generator_function_declaration" => {
                    named("function", false)
                }
                "class_declaration" | "abstract_class_declaration" => named("class", true),
                "interface_declaration" => named("interface", true),
                "enum_declaration" => named("enum", true),
                "type_alias_declaration" => named("type", false),
                "internal_module" | "module" => named("module", false),
                "method_definition" | "method_signature" | "abstract_method_signature" => {
                    named("method", false)
                }
                "public_field_definition" | "field_definition" | "property_signature" => {
                    named("field", false)
                }
                "variable_declarator" => {
                    let value = n.child_by_field_name("value")?;
                    let fn_like = matches!(
                        value.kind(),
                        "arrow_function"
                            | "function_expression"
                            | "function"
                            | "generator_function"
                    );
                    let name = n.child_by_field_name("name")?;
                    if fn_like && name.kind() == "identifier" {
                        Some(Def {
                            kind: func(),
                            name: self.text(name).to_string(),
                            parent: None,
                            typed: false,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Lang::Python => match k {
                "function_definition" => named(func(), false),
                "class_definition" => named("class", true),
                _ => None,
            },
            Lang::Go => match k {
                "function_declaration" => named("function", false),
                "method_declaration" => {
                    let recv = n
                        .child_by_field_name("receiver")
                        .map(|r| self.last_ident_of_type(r))
                        .unwrap_or_default();
                    Some(Def {
                        kind: "method",
                        name: self.field_text(n, "name"),
                        parent: Some(recv),
                        typed: false,
                    })
                }
                "type_spec" => {
                    let kind = match n.child_by_field_name("type").map(|t| t.kind()) {
                        Some("struct_type") => "struct",
                        Some("interface_type") => "interface",
                        _ => "type",
                    };
                    named(kind, true)
                }
                "method_elem" | "method_spec" => named("method", false),
                "field_declaration" => Some(Def {
                    kind: "field",
                    name: self.first_ident(n, 0),
                    parent: None,
                    typed: false,
                }),
                "const_spec" | "var_spec" if self.stack.is_empty() => Some(Def {
                    kind: "const",
                    name: self.first_ident(n, 0),
                    parent: None,
                    typed: false,
                }),
                _ => None,
            },
            Lang::Java => match k {
                "class_declaration" | "record_declaration" => named("class", true),
                "interface_declaration" | "annotation_type_declaration" => named("interface", true),
                "enum_declaration" => named("enum", true),
                "method_declaration" | "constructor_declaration" => named("method", false),
                "field_declaration" => Some(Def {
                    kind: "field",
                    name: n
                        .child_by_field_name("declarator")
                        .map(|d| self.first_ident(d, 0))
                        .unwrap_or_default(),
                    parent: None,
                    typed: false,
                }),
                _ => None,
            },
            Lang::CSharp => match k {
                "class_declaration" | "record_declaration" => named("class", true),
                "struct_declaration" => named("struct", true),
                "interface_declaration" => named("interface", true),
                "enum_declaration" => named("enum", true),
                "namespace_declaration" | "file_scoped_namespace_declaration" => {
                    named("module", false)
                }
                "method_declaration" | "constructor_declaration" | "local_function_statement" => {
                    named(if self.in_type() { "method" } else { "function" }, false)
                }
                "property_declaration" => named("field", false),
                "field_declaration" => Some(Def {
                    kind: "field",
                    name: self.first_ident(n, 0),
                    parent: None,
                    typed: false,
                }),
                _ => None,
            },
            Lang::Php => match k {
                "class_declaration" => named("class", true),
                "interface_declaration" => named("interface", true),
                "trait_declaration" => named("trait", true),
                "enum_declaration" => named("enum", true),
                "function_definition" => named("function", false),
                "method_declaration" => named("method", false),
                "namespace_definition" => named("module", false),
                "property_declaration" => Some(Def {
                    kind: "field",
                    name: self.first_ident(n, 0).trim_start_matches('$').to_string(),
                    parent: None,
                    typed: false,
                }),
                "const_declaration" => Some(Def {
                    kind: "const",
                    name: self.first_ident(n, 0),
                    parent: None,
                    typed: false,
                }),
                _ => None,
            },
            Lang::Markdown => match k {
                "section" => {
                    let mut cur = n.walk();
                    let head = n
                        .named_children(&mut cur)
                        .find(|c| c.kind() == "atx_heading" || c.kind() == "setext_heading")?;
                    let text = self
                        .text(head)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_start_matches('#')
                        .trim()
                        .trim_end_matches('#')
                        .trim()
                        .to_string();
                    Some(Def {
                        kind: "heading",
                        name: text,
                        parent: None,
                        typed: false,
                    })
                }
                _ => None,
            },
            Lang::Svelte => None,
        }
    }

    /// The type named by a Go receiver (`(s *Server)` is `Server`).
    fn last_ident_of_type(&self, n: Node) -> String {
        let mut cur = n.walk();
        for c in n.named_children(&mut cur) {
            if let Some(t) = c.child_by_field_name("type") {
                return self.last_ident(t);
            }
        }
        String::new()
    }

    // ---- links ----

    /// The names a call site is referenced by, as tokensave's extractor of the language writes them.
    fn call(&self, n: Node) -> Option<Vec<String>> {
        let t = |c: Node| clip(self.text(c));
        let ts = matches!(self.lang, Lang::TypeScript | Lang::Tsx | Lang::JavaScript);
        let out = match (self.lang, n.kind()) {
            // The full callee, and for `recv.method()` the method alone too.
            (Lang::Rust, "call_expression") | (Lang::Go, "call_expression") => {
                let callee = if self.lang == Lang::Go {
                    n.named_child(0)
                } else {
                    n.child_by_field_name("function")
                }?;
                let full = t(callee);
                let mut v = Vec::new();
                if let Some(bare) = full.rsplit('.').next().filter(|b| *b != full) {
                    v.push(bare.to_string());
                }
                v.push(full);
                v
            }
            (Lang::Rust, "macro_invocation") => {
                let name = match n.child_by_field_name("macro") {
                    Some(m) => t(m),
                    None => t(n).split('!').next().unwrap_or("").trim().to_string(),
                };
                vec![name]
            }
            (_, "call_expression") if ts => vec![t(n.named_child(0)?)],
            // A JSX render is a dependency on the component, its name capitalized.
            (_, "jsx_opening_element" | "jsx_self_closing_element") if ts => {
                let raw = t(n.child_by_field_name("name")?);
                let name = raw.rsplit('.').next().unwrap_or(&raw).trim().to_string();
                if !name.chars().next().is_some_and(char::is_uppercase) {
                    return None;
                }
                vec![name]
            }
            (Lang::Python, "call") => vec![t(n
                .child_by_field_name("function")
                .or_else(|| n.named_child(0))?)],
            (Lang::Java, "method_invocation") => {
                let name = t(n.child_by_field_name("name")?);
                match n.child_by_field_name("object") {
                    Some(o) => vec![format!("{}.{name}", t(o))],
                    None => vec![name],
                }
            }
            (Lang::Java | Lang::CSharp, "object_creation_expression") => {
                vec![format!("new {}", t(n.child_by_field_name("type")?))]
            }
            (Lang::CSharp, "invocation_expression") => {
                let f = n.child_by_field_name("function")?;
                let simple = |m: Node| -> String {
                    if m.kind() == "generic_name" {
                        m.named_child(0).map(t).unwrap_or_else(|| t(m))
                    } else {
                        t(m)
                    }
                };
                let name = match f.kind() {
                    "member_access_expression" => match (
                        f.child_by_field_name("expression"),
                        f.child_by_field_name("name"),
                    ) {
                        (Some(recv), Some(name)) => format!("{}.{}", t(recv), simple(name)),
                        _ => t(f),
                    },
                    "generic_name" => simple(f),
                    _ => t(f),
                };
                vec![name
                    .strip_prefix("global::")
                    .map(str::to_string)
                    .unwrap_or(name)]
            }
            (Lang::Php, "function_call_expression") => vec![t(n.named_child(0)?)],
            (
                Lang::Php,
                "member_call_expression"
                | "nullsafe_member_call_expression"
                | "scoped_call_expression",
            ) => vec![t(n.child_by_field_name("name")?)],
            _ => return None,
        };
        let out: Vec<String> = out.into_iter().filter(|s| !s.is_empty()).collect();
        (!out.is_empty()).then_some(out)
    }

    /// The names of what an import brings, as tokensave's `Use` nodes are named.
    fn import(&self, n: Node) -> Option<Vec<String>> {
        let one = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let strip = |s: &str, kw: &[&str]| -> String {
            let mut x = one(s);
            for k in kw {
                if let Some(r) = x.strip_prefix(k) {
                    x = r.trim().to_string();
                }
            }
            x.trim_end_matches(';').trim().to_string()
        };
        let out = match (self.lang, n.kind()) {
            (Lang::Rust, "use_declaration") => vec![strip(self.text(n), &["use "])],
            (Lang::TypeScript | Lang::Tsx | Lang::JavaScript, "import_statement") => {
                let mut cur = n.walk();
                let path = n
                    .children(&mut cur)
                    .find(|c| c.kind() == "string")
                    .map(|s| {
                        self.text(s)
                            .trim()
                            .trim_matches('\'')
                            .trim_matches('"')
                            .to_string()
                    })
                    .filter(|p| !p.is_empty());
                vec![path.unwrap_or_else(|| one(self.text(n)))]
            }
            (Lang::Python, "import_statement") => {
                let mut cur = n.walk();
                let kids: Vec<Node> = n.children(&mut cur).collect();
                kids.into_iter()
                    .filter_map(|c| match c.kind() {
                        "dotted_name" => Some(self.text(c).to_string()),
                        "aliased_import" => {
                            let mut c2 = c.walk();
                            let d = c.children(&mut c2).find(|x| x.kind() == "dotted_name");
                            Some(self.text(d.unwrap_or(c)).to_string())
                        }
                        _ => None,
                    })
                    .collect()
            }
            (Lang::Python, "import_from_statement") => {
                let module = n
                    .child_by_field_name("module_name")
                    .map(|m| self.text(m).to_string())
                    .unwrap_or_default();
                let join = |name: &str| {
                    if module.is_empty() {
                        name.to_string()
                    } else {
                        format!("{module}.{name}")
                    }
                };
                let mut cur = n.walk();
                let kids: Vec<Node> = n.children(&mut cur).collect();
                if kids.iter().any(|c| c.kind() == "wildcard_import") {
                    vec![format!("{module}.*")]
                } else {
                    let mut past = false;
                    let mut v = Vec::new();
                    for c in kids {
                        if c.kind() == "import" {
                            past = true;
                        } else if past {
                            match c.kind() {
                                "dotted_name" => v.push(join(self.text(c))),
                                "aliased_import" => {
                                    let mut c2 = c.walk();
                                    let d = c.children(&mut c2).find(|x| x.kind() == "dotted_name");
                                    v.push(join(self.text(d.unwrap_or(c))));
                                }
                                _ => {}
                            }
                        }
                    }
                    if v.is_empty() {
                        v.push(module.clone());
                    }
                    v
                }
            }
            (Lang::Go, "import_spec") => {
                let path = n
                    .child_by_field_name("path")
                    .map(|p| self.text(p).trim_matches('"').trim_matches('`').to_string())?;
                match n.child_by_field_name("name") {
                    Some(a) => vec![format!("{path} as {}", self.text(a))],
                    None => vec![path],
                }
            }
            (Lang::Java, "import_declaration") => {
                vec![strip(self.text(n), &["import ", "static "])]
            }
            (Lang::CSharp, "using_directive") => {
                vec![strip(self.text(n), &["global ", "using ", "static "])]
            }
            (Lang::Php, "namespace_use_declaration") => vec![strip(self.text(n), &["use "])],
            _ => return None,
        };
        let out: Vec<String> = out
            .into_iter()
            .map(|s| s.chars().take(240).collect::<String>())
            .filter(|s| !s.is_empty())
            .collect();
        (!out.is_empty()).then_some(out)
    }

    /// `extends` and `implements` of a type.
    fn heritage(&mut self, n: Node, idx: usize) {
        let line = self.line(n);
        let mut found: Vec<(&'static str, String)> = Vec::new();
        match (self.lang, n.kind()) {
            (Lang::Rust, "impl_item") => {
                if let Some(t) = n.child_by_field_name("trait") {
                    let r = self.last_ident(t);
                    if !r.is_empty() {
                        found.push(("implements", r));
                    }
                }
            }
            (Lang::Python, "class_definition") => {
                if let Some(args) = n.child_by_field_name("superclasses") {
                    let mut cur = args.walk();
                    for c in args.named_children(&mut cur) {
                        let r = self.last_ident(c);
                        if !r.is_empty() {
                            found.push(("extends", r));
                        }
                    }
                }
            }
            (Lang::Go, "type_spec") => {
                // An embedded struct or interface stands for what it embeds.
                if let Some(t) = n.child_by_field_name("type") {
                    let mut stack = vec![t];
                    while let Some(x) = stack.pop() {
                        if x.kind() == "field_declaration"
                            && x.child_by_field_name("name").is_none()
                        {
                            if let Some(ty) = x.child_by_field_name("type") {
                                let r = self.last_ident(ty);
                                if !r.is_empty() {
                                    found.push(("extends", r));
                                }
                            }
                        } else if x.kind() == "type_elem" {
                            let r = self.last_ident(x);
                            if !r.is_empty() {
                                found.push(("extends", r));
                            }
                        } else {
                            let mut cur = x.walk();
                            stack.extend(x.named_children(&mut cur));
                        }
                    }
                }
            }
            _ => {
                const CLAUSES: &[&str] = &[
                    "class_heritage",
                    "extends_clause",
                    "implements_clause",
                    "extends_type_clause",
                    "superclass",
                    "super_interfaces",
                    "extends_interfaces",
                    "base_list",
                    "base_clause",
                    "class_interface_clause",
                ];
                let mut cur = n.walk();
                for c in n.named_children(&mut cur) {
                    if CLAUSES.contains(&c.kind()) {
                        let rel = clause_relation(c.kind());
                        self.clause(c, rel, &mut found);
                    }
                }
            }
        }
        let col = n.start_position().column as u32;
        for (kind, target) in found {
            self.out.edges.push(Edge {
                from: Some(idx),
                kind,
                target,
                line,
                col,
                via: 0,
            });
        }
    }

    fn clause(&self, n: Node, rel: &'static str, found: &mut Vec<(&'static str, String)>) {
        let mut cur = n.walk();
        for c in n.named_children(&mut cur) {
            match c.kind() {
                "extends_clause" => {
                    self.clause(c, "extends", found);
                    continue;
                }
                "implements_clause" => {
                    self.clause(c, "implements", found);
                    continue;
                }
                "type_list" => {
                    self.clause(c, rel, found);
                    continue;
                }
                _ => {}
            }
            let name = self.last_ident(c);
            if !name.is_empty() {
                found.push((rel, name));
            }
        }
    }

    // ---- visibility ----

    /// The visibility of a definition, by the rules of tokensave's extractors (commit 580936b,
    /// MIT, © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md): Rust `extract_visibility`, the
    /// `in_export` and `extract_ts_accessibility` of TypeScript, `python_visibility`,
    /// `go_visibility`, `extract_java_visibility`, `extract_csharp_visibility`, PHP's
    /// `extract_visibility`; a Markdown heading is public.
    fn visibility(&self, n: Node, name: &str) -> &'static str {
        let child_text = |kinds: &[&str]| -> Option<&'a str> {
            let mut cur = n.walk();
            let found = n
                .children(&mut cur)
                .find(|c| kinds.contains(&c.kind()))
                .map(|c| self.text(c));
            found
        };
        match self.lang {
            Lang::Rust => match child_text(&["visibility_modifier"]) {
                Some("pub") => "pub",
                Some(s) if s.contains("crate") => "crate",
                Some(s) if s.contains("super") => "super",
                Some(_) => "pub",
                None => "private",
            },
            Lang::TypeScript | Lang::Tsx | Lang::JavaScript | Lang::Svelte => {
                if matches!(
                    n.kind(),
                    "method_definition" | "public_field_definition" | "field_definition"
                ) {
                    return match child_text(&["accessibility_modifier"]) {
                        Some("private") => "private",
                        Some("protected") => "super",
                        _ => "pub",
                    };
                }
                let mut up = n.parent();
                while let Some(p) = up {
                    if p.kind() == "export_statement" {
                        return "pub";
                    }
                    up = p.parent();
                }
                "private"
            }
            Lang::Python => {
                if name.starts_with("__") && name.ends_with("__") && name.len() > 4 {
                    "pub"
                } else if name.starts_with('_') {
                    "private"
                } else {
                    "pub"
                }
            }
            Lang::Go => {
                if name.chars().next().is_some_and(char::is_uppercase) {
                    "pub"
                } else {
                    "private"
                }
            }
            Lang::Java => match child_text(&["modifiers"]) {
                Some(t) if t.contains("public") => "pub",
                Some(t) if t.contains("protected") => "crate",
                _ => "private",
            },
            Lang::CSharp => {
                let mut cur = n.walk();
                let mods: Vec<&str> = n
                    .children(&mut cur)
                    .filter(|c| c.kind() == "modifier")
                    .map(|c| self.text(c))
                    .collect();
                mods.iter()
                    .find_map(|m| match *m {
                        "public" => Some("pub"),
                        "private" => Some("private"),
                        "internal" => Some("crate"),
                        "protected" => Some("super"),
                        _ => None,
                    })
                    .unwrap_or("private")
            }
            Lang::Php => match child_text(&["visibility_modifier"]).map(str::trim) {
                Some("protected") | Some("private") => "private",
                _ => "pub",
            },
            Lang::Markdown => "pub",
        }
    }

    // ---- text ----

    fn signature(&self, n: Node) -> String {
        let first = self.text(n).lines().next().unwrap_or("").trim();
        first.chars().take(200).collect()
    }

    /// The comments above a definition, the docstring of a Python one.
    fn doc(&self, n: Node) -> String {
        if self.lang == Lang::Python {
            let Some(body) = n.child_by_field_name("body") else {
                return String::new();
            };
            let Some(first) = body.named_child(0) else {
                return String::new();
            };
            if first.kind() == "expression_statement" {
                if let Some(s) = first.named_child(0).filter(|s| s.kind() == "string") {
                    return clean_doc(self.text(s));
                }
            }
            return String::new();
        }
        let mut anchor = n;
        if let Some(p) = n.parent() {
            if matches!(
                p.kind(),
                "export_statement"
                    | "decorated_definition"
                    | "type_declaration"
                    | "const_declaration"
                    | "var_declaration"
            ) {
                anchor = p;
            }
        }
        let mut lines: Vec<String> = Vec::new();
        let mut expected_row = anchor.start_position().row;
        let mut prev = anchor.prev_named_sibling();
        while let Some(p) = prev {
            let k = p.kind();
            let is_comment = k.contains("comment");
            let skippable = matches!(k, "attribute_item" | "decorator" | "attribute_list");
            if !(is_comment || skippable) || p.end_position().row + 1 < expected_row {
                break;
            }
            if is_comment {
                lines.push(clean_doc(self.text(p)));
            }
            expected_row = p.start_position().row;
            prev = p.prev_named_sibling();
        }
        lines.reverse();
        let mut doc = lines.join("\n");
        if doc.chars().count() > 600 {
            doc = doc.chars().take(600).collect();
        }
        doc
    }
}

/// The longest reference name kept: a callee's text can hold a whole chain of calls, whose end (the
/// method called) is what resolves it.
const MAX_REF: usize = 300;

fn clip(text: &str) -> String {
    let n = text.chars().count();
    if n <= MAX_REF {
        return text.to_string();
    }
    text.chars().skip(n - MAX_REF).collect()
}

/// What a clause of a type's heritage says, by its kind: `extends` or `implements`.
fn clause_relation(kind: &str) -> &'static str {
    if kind.contains("implement") || kind.contains("interface") {
        "implements"
    } else {
        "extends"
    }
}

fn clean_doc(raw: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in raw.lines() {
        let l = line.trim();
        let l = l
            .trim_start_matches("///")
            .trim_start_matches("//!")
            .trim_start_matches("//")
            .trim_start_matches("/**")
            .trim_start_matches("/*")
            .trim_end_matches("*/")
            .trim_start_matches('*')
            .trim_start_matches('#')
            .trim_matches('"')
            .trim_matches('\'')
            .trim();
        if !l.is_empty() {
            out.push(l.to_string());
        }
    }
    out.join("\n")
}
