//! The receivers of method calls, typed where the source says their type (ESC-297).
//!
//! Ported from tokensave (github.com/aovestdipaperino/tokensave, commit 580936b, MIT, © 2025 Enzo
//! Lombardi), `src/extraction/rust_extractor.rs` (`collect_var_types`, `collect_let_types`,
//! `normalize_type_name`, `infer_expr_type`, `extract_typed_method_calls`, `CONSTRUCTOR_NAMES`),
//! `src/extraction/typescript_extractor.rs` (`collect_var_types`, `collect_field_types`,
//! `collect_constructor_param_properties`, `collect_let_types`, `emit_typed_method_calls`) and
//! `src/extraction/python_extractor.rs` (`collect_var_types`, `collect_assignment_types`,
//! `emit_typed_method_calls`): `recv.method()` whose receiver is `self`/`this`, a typed parameter, a
//! typed or constructed binding (or, in TypeScript, `this.<field>`) is also given as `Type::method`,
//! which the resolver binds through the qualified names.
//!
//! One thing is ours (written in `docs/CODEINDEX-PROBES.md`): in Rust, `a.field.method()` and
//! `self.field.method()` are given as `field::method` with `via` 2, the field's type found at
//! resolution (`resolve::field_type`).

use std::collections::HashMap;
use tree_sitter::Node;

/// A typed call found: the reference name and where the call is.
pub struct Typed {
    pub name: String,
    pub line: u32,
    pub col: u32,
    /// 1: `Type::method`; 2: `field::method`, the field's type to find.
    pub via: u8,
}

fn text<'a>(src: &'a [u8], n: Node) -> &'a str {
    n.utf8_text(src).unwrap_or("")
}

fn child_of_kind<'t>(n: Node<'t>, kind: &str) -> Option<Node<'t>> {
    let mut cur = n.walk();
    let found = n.children(&mut cur).find(|c| c.kind() == kind);
    found
}

fn pos(n: Node) -> (u32, u32) {
    (
        n.start_position().row as u32 + 1,
        n.start_position().column as u32,
    )
}

// ---- Rust ----

/// `rust_extractor.rs::CONSTRUCTOR_NAMES`.
const CONSTRUCTOR_NAMES: &[&str] = &[
    "new",
    "default",
    "with_capacity",
    "from",
    "try_from",
    "build",
    "create",
    "open",
    "init",
    "builder",
    "from_str",
];

/// `rust_extractor.rs::normalize_type_name`: `&mut MermaidRenderer` is `MermaidRenderer`,
/// `std::vec::Vec<u8>` is `Vec`; none for tuples, slices, `impl Trait`.
pub fn rust_type_name(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    loop {
        if let Some(r) = s.strip_prefix('&') {
            s = r.trim_start();
        } else if let Some(r) = s.strip_prefix("mut ") {
            s = r.trim_start();
        } else if s.starts_with('\'') {
            let pos = s.find(char::is_whitespace)?;
            s = s[pos..].trim_start();
        } else {
            break;
        }
    }
    if let Some(r) = s.strip_prefix("dyn ") {
        s = r.trim_start();
    }
    if s.starts_with("impl ") {
        return None;
    }
    if let Some(pos) = s.find('<') {
        s = s[..pos].trim_end();
    }
    let seg = s.rsplit("::").next().unwrap_or(s).trim();
    let first = seg.chars().next()?;
    if !first.is_alphabetic() && first != '_' {
        return None;
    }
    Some(seg.to_string())
}

/// What holds a value without changing what is called on it: `Arc<Runtime>` calls `Runtime`'s
/// methods. Ours, for the type of a field (`via` 2).
const WRAPPERS: &[&str] = &[
    "Arc", "Rc", "Box", "Weak", "Mutex", "RwLock", "RefCell", "Cell", "Option", "OnceLock",
    "OnceCell", "LazyLock", "Lazy", "ArcSwap",
];

/// The type a field's declared type calls the methods of: `Arc<crate::a::Runtime>` is `Runtime`.
pub fn rust_field_type(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    loop {
        let name = rust_type_name(s)?;
        if !WRAPPERS.contains(&name.as_str()) {
            return Some(name);
        }
        let open = s.find('<')?;
        let close = s.rfind('>')?;
        if close <= open {
            return None;
        }
        s = s[open + 1..close].trim();
    }
}

fn rust_binding_ident(src: &[u8], pat: Node) -> Option<String> {
    match pat.kind() {
        "identifier" => Some(text(src, pat).to_string()),
        "mut_pattern" => pat
            .named_child(0)
            .filter(|c| c.kind() == "identifier")
            .map(|c| text(src, c).to_string()),
        _ => None,
    }
}

fn rust_infer_expr_type(src: &[u8], value: Node, self_type: Option<&str>) -> Option<String> {
    match value.kind() {
        "call_expression" => {
            let func = value.child_by_field_name("function")?;
            if func.kind() != "scoped_identifier" {
                return None;
            }
            let name = text(src, func.child_by_field_name("name")?);
            if !CONSTRUCTOR_NAMES.contains(&name) {
                return None;
            }
            let ty = rust_type_name(text(src, func.child_by_field_name("path")?))?;
            if ty == "Self" {
                self_type.map(str::to_string)
            } else {
                Some(ty)
            }
        }
        "struct_expression" => rust_type_name(text(src, value.child_by_field_name("name")?)),
        "reference_expression" | "try_expression" => value
            .named_child(0)
            .and_then(|inner| rust_infer_expr_type(src, inner, self_type)),
        _ => None,
    }
}

fn rust_let_types(
    src: &[u8],
    node: Node,
    self_type: Option<&str>,
    map: &mut HashMap<String, String>,
) {
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        if c.kind() == "let_declaration" {
            if let Some(var) = c
                .child_by_field_name("pattern")
                .and_then(|p| rust_binding_ident(src, p))
            {
                let ty = c
                    .child_by_field_name("type")
                    .and_then(|t| rust_type_name(text(src, t)))
                    .or_else(|| {
                        c.child_by_field_name("value")
                            .and_then(|v| rust_infer_expr_type(src, v, self_type))
                    });
                if let Some(t) = ty {
                    map.insert(var, t);
                }
            }
        }
        if c.kind() != "function_item" {
            rust_let_types(src, c, self_type, map);
        }
    }
}

/// The typed calls of a Rust function: `self` is the type of the `impl` it is in.
pub fn rust(src: &[u8], func: Node, self_type: Option<&str>, out: &mut Vec<Typed>) {
    let mut vars: HashMap<String, String> = HashMap::new();
    if let Some(t) = self_type {
        vars.insert("self".into(), t.to_string());
    }
    if let Some(params) = func.child_by_field_name("parameters") {
        let mut cur = params.walk();
        for p in params.children(&mut cur) {
            if p.kind() != "parameter" {
                continue;
            }
            if let (Some(pat), Some(ty)) = (
                p.child_by_field_name("pattern"),
                p.child_by_field_name("type"),
            ) {
                if pat.kind() == "identifier" {
                    if let Some(tn) = rust_type_name(text(src, ty)) {
                        vars.insert(text(src, pat).to_string(), tn);
                    }
                }
            }
        }
    }
    let Some(body) = func.child_by_field_name("body") else {
        return;
    };
    rust_let_types(src, body, self_type, &mut vars);
    rust_calls(src, body, &vars, out);
}

fn rust_calls(src: &[u8], node: Node, vars: &HashMap<String, String>, out: &mut Vec<Typed>) {
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() == "call_expression" {
            if let Some(func) = child
                .child_by_field_name("function")
                .filter(|f| f.kind() == "field_expression")
            {
                if let (Some(recv), Some(field)) = (
                    func.child_by_field_name("value"),
                    func.child_by_field_name("field"),
                ) {
                    if field.kind() == "field_identifier" {
                        let method = text(src, field);
                        let (line, col) = pos(child);
                        let ty = match recv.kind() {
                            "self" => vars.get("self").cloned(),
                            "identifier" => vars.get(text(src, recv)).cloned(),
                            _ => None,
                        };
                        if let Some(ty) = ty {
                            out.push(Typed {
                                name: format!("{ty}::{method}"),
                                line,
                                col,
                                via: 1,
                            });
                        } else if recv.kind() == "field_expression" {
                            // Ours: `a.field.method()`, the field's type found at resolution.
                            let inner = recv.child_by_field_name("value");
                            let name = recv
                                .child_by_field_name("field")
                                .filter(|f| f.kind() == "field_identifier");
                            if let (Some(inner), Some(name)) = (inner, name) {
                                if matches!(inner.kind(), "self" | "identifier") {
                                    out.push(Typed {
                                        name: format!("{}::{method}", text(src, name)),
                                        line,
                                        col,
                                        via: 2,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        if child.kind() != "function_item" {
            rust_calls(src, child, vars, out);
        }
    }
}

// ---- TypeScript, JavaScript (and the scripts of Svelte) ----

/// `typescript_extractor.rs::normalize_type_name`: `: Service<T>` is `Service`, `ns.Foo[]` is `Foo`.
pub fn ts_type_name(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    s = s.trim_start_matches(':').trim();
    if let Some(r) = s.strip_prefix("new ") {
        s = r.trim_start();
    }
    if let Some(p) = s.find(['<', '[', '(', '|', '&', ' ', '?']) {
        s = &s[..p];
    }
    let seg = s.rsplit('.').next().unwrap_or(s).trim();
    let first = seg.chars().next()?;
    if !first.is_alphabetic() && first != '_' {
        return None;
    }
    Some(seg.to_string())
}

fn ts_value_type(src: &[u8], value: Node) -> Option<String> {
    if value.kind() == "new_expression" {
        let ctor = value
            .child_by_field_name("constructor")
            .unwrap_or_else(|| value.named_child(0).unwrap_or(value));
        return ts_type_name(text(src, ctor));
    }
    None
}

fn ts_let_types(src: &[u8], node: Node, map: &mut HashMap<String, String>) {
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        if c.kind() == "variable_declarator" {
            if let Some(name) = c
                .child_by_field_name("name")
                .filter(|n| n.kind() == "identifier")
            {
                let ty = c
                    .child_by_field_name("type")
                    .and_then(|t| ts_type_name(text(src, t)))
                    .or_else(|| {
                        c.child_by_field_name("value")
                            .and_then(|v| ts_value_type(src, v))
                    });
                if let Some(t) = ty {
                    map.insert(text(src, name).to_string(), t);
                }
            }
        }
        if !matches!(
            c.kind(),
            "arrow_function" | "function" | "function_declaration" | "method_definition"
        ) {
            ts_let_types(src, c, map);
        }
    }
}

fn ts_ctor_properties(src: &[u8], ctor: Node, map: &mut HashMap<String, String>) {
    let Some(params) = child_of_kind(ctor, "formal_parameters") else {
        return;
    };
    let mut cur = params.walk();
    for param in params.children(&mut cur) {
        let is_property = param.kind() == "required_parameter"
            && (child_of_kind(param, "accessibility_modifier").is_some()
                || child_of_kind(param, "override_modifier").is_some()
                || {
                    let mut c2 = param.walk();
                    let ro = param.children(&mut c2).any(|c| c.kind() == "readonly");
                    ro
                });
        if !is_property {
            continue;
        }
        if let (Some(pat), Some(ty)) = (
            param.child_by_field_name("pattern"),
            param.child_by_field_name("type"),
        ) {
            if pat.kind() == "identifier" {
                if let Some(tn) = ts_type_name(text(src, ty)) {
                    map.insert(format!("this.{}", text(src, pat)), tn);
                }
            }
        }
    }
}

fn ts_field_types(src: &[u8], class_body: Node, map: &mut HashMap<String, String>) {
    let mut cur = class_body.walk();
    for member in class_body.children(&mut cur) {
        if member.kind() == "method_definition"
            && member
                .child_by_field_name("name")
                .is_some_and(|n| text(src, n) == "constructor")
        {
            ts_ctor_properties(src, member, map);
        }
        if matches!(
            member.kind(),
            "public_field_definition" | "property_signature" | "field_definition"
        ) {
            if let Some(name) = member.child_by_field_name("name") {
                let ty = member
                    .child_by_field_name("type")
                    .and_then(|t| ts_type_name(text(src, t)))
                    .or_else(|| {
                        member
                            .child_by_field_name("value")
                            .and_then(|v| ts_value_type(src, v))
                    });
                if let Some(ty) = ty {
                    map.insert(format!("this.{}", text(src, name)), ty);
                }
            }
        }
    }
}

/// The typed calls of a TypeScript function: `this` is the class it is in. `func` is the function's
/// node (a declaration, a method, an arrow function).
pub fn ts(src: &[u8], func: Node, self_type: Option<&str>, out: &mut Vec<Typed>) {
    let mut vars: HashMap<String, String> = HashMap::new();
    if let Some(t) = self_type {
        vars.insert("this".into(), t.to_string());
    }
    if let Some(params) = child_of_kind(func, "formal_parameters") {
        let mut cur = params.walk();
        for p in params.children(&mut cur) {
            if !matches!(p.kind(), "required_parameter" | "optional_parameter") {
                continue;
            }
            if let (Some(pat), Some(ty)) = (
                p.child_by_field_name("pattern"),
                p.child_by_field_name("type"),
            ) {
                if pat.kind() == "identifier" {
                    if let Some(tn) = ts_type_name(text(src, ty)) {
                        vars.insert(text(src, pat).to_string(), tn);
                    }
                }
            }
        }
    }
    if let Some(body) = child_of_kind(func, "statement_block") {
        ts_let_types(src, body, &mut vars);
    }
    if let Some(class_body) = func.parent().filter(|p| p.kind() == "class_body") {
        ts_field_types(src, class_body, &mut vars);
    }
    let Some(body) = func.child_by_field_name("body") else {
        return;
    };
    ts_calls(src, body, &vars, out);
}

fn ts_calls(src: &[u8], node: Node, vars: &HashMap<String, String>, out: &mut Vec<Typed>) {
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() == "call_expression" {
            if let Some(func) = child
                .child_by_field_name("function")
                .filter(|f| f.kind() == "member_expression")
            {
                if let (Some(obj), Some(prop)) = (
                    func.child_by_field_name("object"),
                    func.child_by_field_name("property"),
                ) {
                    let ty = match obj.kind() {
                        "this" => vars.get("this").cloned(),
                        "identifier" => vars.get(text(src, obj)).cloned(),
                        "member_expression" => obj
                            .child_by_field_name("object")
                            .filter(|inner| inner.kind() == "this")
                            .and_then(|_| obj.child_by_field_name("property"))
                            .and_then(|f| vars.get(&format!("this.{}", text(src, f))).cloned()),
                        _ => None,
                    };
                    if let Some(ty) = ty {
                        let (line, col) = pos(child);
                        out.push(Typed {
                            name: format!("{ty}::{}", text(src, prop)),
                            line,
                            col,
                            via: 1,
                        });
                    }
                }
            }
        }
        if !matches!(child.kind(), "function_declaration" | "method_definition") {
            ts_calls(src, child, vars, out);
        }
    }
}

// ---- Python ----

/// `python_extractor.rs::normalize_type_name`: `mod.Service` is `Service`.
pub fn py_type_name(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    if let Some(p) = s.find(['[', '(', ' ']) {
        s = &s[..p];
    }
    let seg = s.rsplit('.').next().unwrap_or(s).trim();
    let first = seg.chars().next()?;
    if !first.is_alphabetic() && first != '_' {
        return None;
    }
    Some(seg.to_string())
}

fn py_assignment_types(src: &[u8], node: Node, map: &mut HashMap<String, String>) {
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        if c.kind() == "assignment" {
            if let Some(lhs) = c
                .child_by_field_name("left")
                .filter(|l| l.kind() == "identifier")
            {
                let ty = c
                    .child_by_field_name("type")
                    .and_then(|t| py_type_name(text(src, t)))
                    .or_else(|| {
                        let rhs = c.child_by_field_name("right")?;
                        if rhs.kind() != "call" {
                            return None;
                        }
                        let callee = rhs.child_by_field_name("function")?;
                        if callee.kind() != "identifier" {
                            return None;
                        }
                        let name = text(src, callee);
                        // PEP 8's CapWords: the only cheap sign that `Service()` builds one.
                        name.chars()
                            .next()
                            .is_some_and(char::is_uppercase)
                            .then(|| name.to_string())
                    });
                if let Some(t) = ty {
                    map.insert(text(src, lhs).to_string(), t);
                }
            }
        }
        if !matches!(c.kind(), "function_definition" | "class_definition") {
            py_assignment_types(src, c, map);
        }
    }
}

/// The typed calls of a Python function: `self` is the class it is in.
pub fn python(src: &[u8], func: Node, self_type: Option<&str>, out: &mut Vec<Typed>) {
    let mut vars: HashMap<String, String> = HashMap::new();
    if let Some(t) = self_type {
        vars.insert("self".into(), t.to_string());
    }
    if let Some(params) = func.child_by_field_name("parameters") {
        let mut cur = params.walk();
        for p in params.children(&mut cur) {
            if p.kind() != "typed_parameter" {
                continue;
            }
            let ident = p.named_child(0).filter(|n| n.kind() == "identifier");
            if let (Some(ident), Some(ty)) = (ident, p.child_by_field_name("type")) {
                if let Some(tn) = py_type_name(text(src, ty)) {
                    vars.insert(text(src, ident).to_string(), tn);
                }
            }
        }
    }
    let Some(body) = child_of_kind(func, "block") else {
        return;
    };
    py_assignment_types(src, body, &mut vars);
    py_calls(src, body, &vars, out);
}

fn py_calls(src: &[u8], node: Node, vars: &HashMap<String, String>, out: &mut Vec<Typed>) {
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() == "call" {
            if let Some(func) = child
                .child_by_field_name("function")
                .filter(|f| f.kind() == "attribute")
            {
                if let (Some(obj), Some(attr)) = (
                    func.child_by_field_name("object"),
                    func.child_by_field_name("attribute"),
                ) {
                    if obj.kind() == "identifier" {
                        if let Some(ty) = vars.get(text(src, obj)) {
                            let (line, col) = pos(child);
                            out.push(Typed {
                                name: format!("{ty}::{}", text(src, attr)),
                                line,
                                col,
                                via: 1,
                            });
                        }
                    }
                }
            }
        }
        if !matches!(child.kind(), "function_definition" | "class_definition") {
            py_calls(src, child, vars, out);
        }
    }
}
