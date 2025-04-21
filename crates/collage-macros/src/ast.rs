use itertools::Itertools;
use syn::{
    Expr, ExprLit, Ident, Lit, LitBool, LitStr, Stmt, Token, braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::{Part, PartBuilder, Parts};

const UNSUPPORTED_ATTR_KEY: &str = "unsupported attribute key expression";
const INVALID_ATTR_KEY: &str =
    "attribute keys must start with alphanumeric, `_`, or `:` and only contain those tokens; `-` or `.`";

pub struct Markup(Vec<Element>);

impl Parse for Markup {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attr = Vec::new();
        while !input.is_empty() {
            attr.push(input.parse()?);
        }
        Ok(Self(attr))
    }
}

impl PartBuilder for Markup {
    fn append(&self, parts: &mut Parts) {
        for element in &self.0 {
            element.append(parts);
        }
    }
}

enum Element {
    Container { tag: Ident, attr: Attributes, m: Markup },
    Void { tag: Ident, attr: Attributes },
    Lit(Lit),
    Expr(Expr),
    Nothing,
}

impl Parse for Element {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            let tag = input.parse()?;
            let attr = input.parse()?;
            if crate::is_void(&tag) {
                input.parse::<Token![;]>()?;
                Ok(Self::Void { tag, attr })
            } else {
                let m;
                braced!(m in input);
                let m = m.parse()?;
                Ok(Self::Container { tag, attr, m })
            }
        } else if input.peek(Lit) {
            let lit: Lit = input.parse()?;
            if matches!(lit, Lit::CStr(_) | Lit::Byte(_) | Lit::Verbatim(_)) {
                proc_macro_error2::abort!(lit, "CStr, Byte, and Verbatim literals are not supported");
            } else {
                Ok(Self::Lit(lit))
            }
        } else if input.peek(Paren) {
            let expr;
            parenthesized!(expr in input);
            Ok(Self::Expr(expr.parse()?))
        } else {
            Ok(Self::Nothing)
        }
    }
}

impl PartBuilder for Element {
    fn append(&self, parts: &mut Parts) {
        match self {
            Element::Container { tag, attr, m } => {
                let tag = tag.to_string();
                parts.push(format!("<{tag}").into());
                attr.append(parts);
                parts.push(">".into());
                m.append(parts);
                parts.push(format!("</{tag}>").into());
            }
            Element::Void { tag, attr } => {
                let tag = tag.to_string();
                parts.push(format!("<{tag}").into());
                attr.append(parts);
                parts.push(">".into());
            }
            Element::Lit(lit) => parts.push(match lit {
                Lit::Str(lit) => html_escape::encode_text(&lit.value()).into(),
                Lit::ByteStr(lit) => html_escape::encode_text(&String::from_utf8_lossy(&lit.value())).into(),
                Lit::Char(lit) => match lit.value() {
                    '&' => "&amp;".into(),
                    '<' => "&lt;".into(),
                    '>' => "&gt;".into(),
                    '"' => "&quot;".into(),
                    '\'' => "&#x27;".into(),
                    '/' => "&#x2F;".into(),
                    c => c.to_string().into(),
                },
                Lit::Int(lit) => lit.base10_digits().into(),
                Lit::Float(lit) => lit.base10_digits().into(),
                Lit::Bool(lit) => lit.value.to_string().into(),
                _ => unreachable!(),
            }),
            Element::Expr(expr) => {
                parts.push(expr.clone().into());
            }
            Element::Nothing => {}
        }
    }
}

struct Attributes(Vec<Attribute>);

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attr = Vec::new();
        while input.peek(LitStr) || input.peek(Ident::peek_any) || input.peek(Paren) {
            attr.push(input.parse()?);
        }
        Ok(Self(attr))
    }
}

impl PartBuilder for Attributes {
    fn append(&self, parts: &mut Parts) {
        for attribute in &self.0 {
            attribute.append(parts);
        }
    }
}

struct Attribute {
    key: AttributeKey,
    val: Option<AttributeVal>,
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        let val = input.parse::<Token![=]>().and_then(|_| input.parse()).ok();
        Ok(Self { key, val })
    }
}

impl PartBuilder for Attribute {
    fn append(&self, parts: &mut Parts) {
        self.key.append(parts);
        if let Some(val) = &self.val {
            parts.push("=".into());
            val.append(parts);
        }
    }
}

enum AttributeKey {
    Idents(Vec<Ident>),
    Lit(LitStr),
    Expr(Expr),
}

impl Parse for AttributeKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            let lit_str: LitStr = input.parse()?;
            if !collage_core::is_valid_attr_key(&lit_str.value()) {
                proc_macro_error2::abort!(lit_str, INVALID_ATTR_KEY);
            } else {
                Ok(Self::Lit(lit_str))
            }
        } else if input.peek(Paren) {
            let expr;
            parenthesized!(expr in input);
            let expr: Expr = expr.parse()?;
            abort_attr_key(&expr);
            Ok(Self::Expr(expr))
        } else {
            let mut idents = Vec::new();
            while input.peek(Ident::peek_any) {
                idents.push(input.call(Ident::parse_any)?);
                _ = input.parse::<Token![-]>();
            }
            Ok(Self::Idents(idents))
        }
    }
}

fn abort_attr_key(expr: &Expr) {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit_str), ..
        }) => {
            if !collage_core::is_valid_attr_key(&lit_str.value()) {
                proc_macro_error2::abort!(lit_str, INVALID_ATTR_KEY);
            }
        }
        Expr::Block(expr) => {
            if let Some(Stmt::Expr(expr, _)) = expr.block.stmts.last() {
                abort_attr_key(expr);
            }
        }
        Expr::If(expr) => {
            if let Some(Stmt::Expr(expr, _)) = expr.then_branch.stmts.last() {
                abort_attr_key(expr);
            }
            if let Some((_, expr)) = &expr.else_branch {
                abort_attr_key(expr);
            }
        }
        Expr::Match(expr) => {
            for arm in &expr.arms {
                abort_attr_key(&arm.body);
            }
        }
        expr => {
            proc_macro_error2::abort!(expr, UNSUPPORTED_ATTR_KEY);
        }
    }
}

impl PartBuilder for AttributeKey {
    fn append(&self, parts: &mut Parts) {
        match self {
            AttributeKey::Idents(idents) => {
                parts.push(" ".into());
                parts.push(idents.iter().map(|ident| ident.to_string()).join("-").into());
            }
            AttributeKey::Lit(lit) => {
                parts.push(" ".into());
                parts.push(lit.value().into());
            }
            AttributeKey::Expr(expr) => parts.push(Part::Dynamic((expr.clone(), true))),
        };
    }
}

enum AttributeVal {
    Lit(Lit),
    Expr(Expr),
}

impl Parse for AttributeVal {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Lit) {
            let lit: Lit = input.parse()?;
            if matches!(lit, Lit::CStr(_) | Lit::Byte(_) | Lit::Verbatim(_)) {
                proc_macro_error2::abort!(lit, "CStr, Byte, and Verbatim literals are not supported");
            } else {
                Ok(Self::Lit(lit))
            }
        } else {
            let expr;
            parenthesized!(expr in input);
            expr.parse().map(Self::Expr)
        }
    }
}

impl PartBuilder for AttributeVal {
    fn append(&self, parts: &mut Parts) {
        parts.push("\"".into());
        parts.push(match self {
            AttributeVal::Lit(Lit::Str(lit)) => html_escape::encode_double_quoted_attribute(&lit.value()).into(),
            AttributeVal::Lit(Lit::ByteStr(lit)) => {
                html_escape::encode_double_quoted_attribute(&String::from_utf8_lossy(&lit.value())).into()
            }
            AttributeVal::Lit(Lit::Char(lit)) => match lit.value() {
                '&' => "&amp;".to_string(),
                '<' => "&lt;".to_string(),
                '>' => "&gt;".to_string(),
                '"' => "&quot;".to_string(),
                '\'' => "&#x27;".to_string(),
                '/' => "&#x2F;".to_string(),
                c => c.to_string(),
            }
            .into(),
            AttributeVal::Lit(Lit::Int(lit)) => lit.base10_digits().into(),
            AttributeVal::Lit(Lit::Float(lit)) => lit.base10_digits().into(),
            AttributeVal::Lit(Lit::Bool(LitBool { value: true, .. })) => "true".into(),
            AttributeVal::Lit(Lit::Bool(LitBool { value: false, .. })) => "false".into(),
            AttributeVal::Expr(expr) => expr.clone().into(),
            _ => unreachable!(),
        });
        parts.push("\"".into());
    }
}
