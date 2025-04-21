//! Macros for [`collage`].
//!
//! [`collage`]: https://crates.io/crates/collage

#![forbid(unsafe_code)]

use std::borrow::Cow;

use ast::Markup;
use itertools::Itertools;
use proc_macro_error2::proc_macro_error;
use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{
    Expr, Macro, Pat, Stmt,
    parse::{ParseStream, Parser},
};

mod ast;

#[proc_macro]
#[proc_macro_error]
pub fn markup(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(input.into()).into()
}

#[proc_macro]
#[proc_macro_error]
pub fn markup_part(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_part(input.into()).into()
}

fn expand(input: TokenStream) -> TokenStream {
    let reserve = input.to_string().len();
    match Parser::parse2(|input: ParseStream| input.parse::<Markup>(), input) {
        Ok(markup) => {
            let mut parts = Parts::new();
            markup.append(&mut parts);
            let mut tokens = TokenStream::new();
            tokens.append_all(parts.0);
            quote! {{
                extern crate alloc;
                extern crate collage;
                &|__collage_buffer: &mut alloc::string::String| {
                    __collage_buffer.reserve(#reserve);
                    #tokens
                }
            }}
        }
        Err(err) => err.into_compile_error(),
    }
}

fn expand_part(input: TokenStream) -> TokenStream {
    match Parser::parse2(|input: ParseStream| input.parse::<Markup>(), input) {
        Ok(markup) => {
            let mut parts = Parts::new();
            markup.append(&mut parts);
            let mut tokens = TokenStream::new();
            tokens.append_all(parts.0);
            tokens
        }
        Err(err) => err.into_compile_error(),
    }
}

fn is_void<T: PartialEq<str>>(tag: &T) -> bool {
    [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr",
    ]
    .iter()
    .any(|void| tag == *void)
}

trait PartBuilder {
    fn append(&self, parts: &mut Parts);
}

impl<T: PartBuilder> PartBuilder for Option<T> {
    fn append(&self, parts: &mut Parts) {
        if let Some(t) = self {
            t.append(parts);
        }
    }
}

struct Parts(Vec<Part>);

impl Parts {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, part: Part) {
        if let Some(prev) = self.0.pop() {
            match (prev, part) {
                (Part::Static(prev), Part::Static(part)) => {
                    self.0.push(format!("{prev}{part}").into());
                }
                (prev, part) => {
                    self.0.push(prev);
                    self.0.push(part);
                }
            }
        } else {
            self.0.push(part);
        }
    }
}

enum Part {
    Static(String),
    Dynamic((Expr, bool)),
}

impl From<String> for Part {
    fn from(value: String) -> Self {
        Self::Static(value)
    }
}

impl From<&str> for Part {
    fn from(value: &str) -> Self {
        Self::Static(value.into())
    }
}

impl From<Cow<'_, str>> for Part {
    fn from(value: Cow<'_, str>) -> Self {
        match value {
            Cow::Borrowed(value) => Self::Static(value.into()),
            Cow::Owned(value) => Self::Static(value),
        }
    }
}

impl From<Expr> for Part {
    fn from(value: Expr) -> Self {
        Self::Dynamic((value, false))
    }
}

impl ToTokens for Part {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Part::Static(s) => {
                quote! { __collage_buffer.push_str(#s); }.to_tokens(tokens);
            }
            Part::Dynamic((Expr::If(expr), attr)) if expr.else_branch.is_none() => {
                tokens.append_all(&expr.attrs);
                expr.if_token.to_tokens(tokens);
                expr.cond.to_tokens(tokens);
                expr.then_branch.brace_token.surround(tokens, |tokens| {
                    let stmts = stmts_replace_mac(&expr.then_branch.stmts);
                    if *attr {
                        quote! { __collage_buffer.push(' '); }.to_tokens(tokens);
                    }
                    quote! { collage::Render::render_to({ #(#stmts)* }, __collage_buffer); }.to_tokens(tokens);
                });
            }
            Part::Dynamic((Expr::Match(expr), _)) => {
                expr.match_token.to_tokens(tokens);
                expr.expr.to_tokens(tokens);
                expr.brace_token.surround(tokens, |tokens| {
                    for arm in &expr.arms {
                        tokens.append_all(&arm.attrs);
                        arm.pat.to_tokens(tokens);
                        if let Some((if_token, expr)) = &arm.guard {
                            if_token.to_tokens(tokens);
                            expr.to_tokens(tokens);
                        }
                        arm.fat_arrow_token.to_tokens(tokens);
                        match &*arm.body {
                            Expr::Macro(expr) if is_markup(&expr.mac) => {
                                tokens.append_all(&expr.attrs);
                                let mac = &expr.mac.tokens;
                                quote! {{ collage::markup_part! { #mac } }}.to_tokens(tokens);
                            }
                            expr => expr.to_tokens(tokens),
                        }
                        arm.comma.to_tokens(tokens);
                    }
                    if !expr.arms.iter().any(|arm| matches!(arm.pat, Pat::Wild(_))) {
                        quote! { _ => {} }.to_tokens(tokens);
                    }
                });
            }
            Part::Dynamic((Expr::ForLoop(expr), _)) => {
                tokens.append_all(&expr.attrs);
                expr.for_token.to_tokens(tokens);
                expr.pat.to_tokens(tokens);
                expr.in_token.to_tokens(tokens);
                expr.expr.to_tokens(tokens);
                let stmts = stmts_replace_mac(&expr.body.stmts);
                quote! {{ #(#stmts)* }}.to_tokens(tokens);
            }
            Part::Dynamic((Expr::Macro(expr), _)) if is_markup(&expr.mac) => {
                tokens.append_all(&expr.attrs);
                let mac = &expr.mac.tokens;
                quote! { collage::markup_part! { #mac } }.to_tokens(tokens);
            }
            Part::Dynamic((Expr::Macro(expr), _)) if is_markup_part(&expr.mac) => {
                tokens.append_all(&expr.attrs);
                expr.mac.to_tokens(tokens);
            }
            Part::Dynamic((expr, attr)) => {
                if *attr {
                    quote! { __collage_buffer.push(' '); }.to_tokens(tokens);
                }
                quote! { collage::Render::render_to(&#expr, __collage_buffer); }.to_tokens(tokens);
            }
        }
    }
}

fn stmts_replace_mac(stmts: &[Stmt]) -> Vec<TokenStream> {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::Macro(stmt) if is_markup(&stmt.mac) => {
                let mac = &stmt.mac.tokens;
                quote! { collage::markup_part! { #mac } }
            }
            stmt => quote! { #stmt },
        })
        .collect_vec()
}

fn is_markup(mac: &Macro) -> bool {
    mac.path.is_ident("markup")
        || mac
            .path
            .segments
            .iter()
            .rev()
            .next_tuple()
            .is_some_and(|(b, a)| a.ident == "collage" && b.ident == "markup")
}

fn is_markup_part(mac: &Macro) -> bool {
    mac.path.is_ident("markup_part")
        || mac
            .path
            .segments
            .iter()
            .rev()
            .next_tuple()
            .is_some_and(|(b, a)| a.ident == "collage" && b.ident == "markup_part")
}
