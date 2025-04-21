//! `collage` is a template engine for Rust, designed for writing HTML and similar markup languages.
//!
//! Rendering is performed by the [`markup!`] macro and custom structs and enums can be rendered by implementing the
//! [`Render`] trait.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
use alloc::string::String;
pub use html_escape;

/// Parses an alternative markup syntax into a [`FnOnce`](`&mut` [`String`]) function.
///
/// # Syntax
///
/// Instead of writing elements with angle brackets, they are written with curly braces.
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! {
///     h1 { "Lorem ipsum" }
///     p { "Quae occaecati corrupti perferendis officia eos quidem" }
/// };
/// assert_eq!(
///     "<h1>Lorem ipsum</h1><p>Quae occaecati corrupti perferendis officia eos quidem</p>",
///     markup.render()
/// );
/// ```
///
/// [Void elements](https://developer.mozilla.org/en-US/docs/Glossary/Void_element) are closed using a semicolon.
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! {
///     meta charset="utf-8";
///     link rel="stylesheet" href="/style.css";
/// };
/// assert_eq!(
///     r#"<meta charset="utf-8"><link rel="stylesheet" href="/style.css">"#,
///     markup.render()
/// );
/// ```
///
/// # Interpolation
///
/// Many Rust literals can be used plainly.
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! {
///     "text" ' ' b"bytes" ' ' 'c' ' ' 42 ' ' 3.14 ' ' true
/// };
/// assert_eq!("text bytes c 42 3.14 true", markup.render());
/// ```
///
/// Rust code can be inserted by surrounding it with parenthesis.
///
/// ```
/// use collage::Render;
/// let (x, y) = (10, 30);
/// let markup = collage::markup! { p { "x: " (x) ", y: " (y) } };
/// assert_eq!("<p>x: 10, y: 30</p>", markup.render());
/// ```
///
/// It works the same for attribute values.
///
/// ```
/// use collage::Render;
/// let val = "my_div";
/// let markup = collage::markup! { div "id"=(val) {} };
/// assert_eq!(r#"<div id="my_div"></div>"#, markup.render());
/// ```
///
/// # Control flow
///
/// `if` statements can be used to conditionally render content.
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! { div { (if true { "shown" } else { "hidden" }) } };
/// assert_eq!("<div>shown</div>", markup.render());
/// ```
///
/// This is especially useful for conditionally rendering attributes.
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! {
///     div (if true { "hidden" }) (if false { "autofocus" }) {}
/// };
/// assert_eq!("<div hidden></div>", markup.render());
/// ```
///
/// `match` can be used as expected. If this macro is used again in the match arm bodies, not all cases have to be covered. Instead nothing will be rendered if there is no match.
///
/// ```
/// use collage::Render;
/// let role = "user";
/// let markup = collage::markup! {
///     (match role {
///         "admin" => collage::markup! { p { "foo" } },
///         "user" => collage::markup! { p { "bar" } },
///     })
/// };
/// assert_eq!("<p>bar</p>", markup.render());
/// ```
///
/// `for` combined with additional macro use can be used to rendering content in a loop.
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! {
///     ul { (for item in ["one", "two", "three"] { markup! { li { (item) } } }) }
/// };
/// assert_eq!("<ul><li>one</li><li>two</li><li>three</li></ul>", markup.render());
/// ```
pub use collage_macros::markup;

/// This is mostly used automatically by the [`markup!`] macro to render partial content. It can be used when manual
/// control is required.
///
/// The [`markup!`] macro wraps the resulting Rust tokens in the following code, while this macro parses the tokens
/// verbatim.
///
/// ```ignore
/// quote::quote! {{
///     extern crate alloc;
///     extern crate collage;
///     &|__collage_buffer: &mut alloc::string::String| {
///         __collage_buffer.reserve(...);
///         // Macro content is inserted here
///     }
/// }}
/// ```
///
/// # Example
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! { form { (collage::markup_part! { input; }) } };
/// assert_eq!("<form><input></form>", markup.render());
/// ```
pub use collage_macros::markup_part;

/// The [`markup!`] macro will render interpolated content by wrapping it in this trait's [`Render::render_to`]
/// function.
///
/// Implementing this trait for types will allow them them to be used directly in the [`markup!`] macro.
///
/// # Example
///
/// ```
/// let markup = collage::markup! { (42) };
/// // Is equivalent to
/// let markup = {
///     extern crate alloc;
///     extern crate collage;
///     &|__collage_buffer: &mut alloc::string::String| {
///         __collage_buffer.reserve(4usize);
///         collage::Render::render_to(&42, __collage_buffer);
///     }
/// };
/// ```
pub trait Render {
    /// Renders self to a mutable [`String`] buffer.
    fn render_to(&self, buffer: &mut String);
    /// Renders self to a [`String`].
    fn render(&self) -> String {
        let mut buffer = String::new();
        self.render_to(&mut buffer);
        buffer
    }
}

impl<F: FnOnce(&mut String) + Copy> Render for F {
    fn render_to(&self, output: &mut String) {
        self(output);
    }
}

impl Render for bool {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(if *self { "true" } else { "false" });
    }
}

impl Render for char {
    fn render_to(&self, buffer: &mut String) {
        match self {
            '&' => buffer.push_str("&amp;"),
            '<' => buffer.push_str("&lt;"),
            '>' => buffer.push_str("&gt;"),
            '"' => buffer.push_str("&quot;"),
            '\'' => buffer.push_str("&#x27;"),
            '/' => buffer.push_str("&#x2F;"),
            _ => buffer.push(*self),
        }
    }
}

impl Render for str {
    fn render_to(&self, buffer: &mut String) {
        html_escape::encode_safe_to_string(self, buffer);
    }
}

impl Render for &str {
    fn render_to(&self, buffer: &mut String) {
        str::render_to(*self, buffer);
    }
}

impl Render for String {
    fn render_to(&self, buffer: &mut String) {
        self.as_str().render_to(buffer);
    }
}

impl Render for alloc::borrow::Cow<'_, str> {
    fn render_to(&self, buffer: &mut String) {
        self.as_ref().render_to(buffer);
    }
}

impl<T: Render> Render for Option<T> {
    fn render_to(&self, buffer: &mut String) {
        if let Some(t) = self {
            t.render_to(buffer);
        }
    }
}

impl<T: Render> Render for alloc::sync::Arc<T> {
    fn render_to(&self, buffer: &mut String) {
        T::render_to(self, buffer);
    }
}

impl<T: Render> Render for alloc::rc::Rc<T> {
    fn render_to(&self, buffer: &mut String) {
        T::render_to(self, buffer);
    }
}

macro_rules! impl_render_itoa {
    ($($ty:ty)*) => {
        $(
            impl Render for $ty {
                fn render_to(&self, buffer: &mut String) {
                    cfg_if::cfg_if! {
                        if #[cfg(feature = "itoa")] {
                            buffer.push_str(itoa::Buffer::new().format(*self));
                        } else {
                            buffer.push_str(self.to_string());
                        }
                    }
                }
            }
        )*
    };
}

impl_render_itoa! {
    i8 i16 i32 i64 i128 isize
    u8 u16 u32 u64 u128 usize
}

macro_rules! impl_render_ryu {
    ($($ty:ty)*) => {
        $(
            impl Render for $ty {
                fn render_to(&self, buffer: &mut String) {
                    cfg_if::cfg_if! {
                        if #[cfg(feature = "ryu")] {
                            buffer.push_str(ryu::Buffer::new().format(*self));
                        } else {
                            buffer.push_str(self.to_string());
                        }
                    }
                }
            }
        )*
    };
}

impl_render_ryu! { f32 f64 }

/// The literal `<!DOCTYPE html>` element.
///
/// # Examples
///
/// ```
/// use collage::Render;
/// let markup = collage::markup! { (collage::doctype) };
/// assert_eq!("<!DOCTYPE html>", markup.render());
/// ```
pub fn doctype(buffer: &mut String) {
    buffer.push_str("<!DOCTYPE html>");
}
