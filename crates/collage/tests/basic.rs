use collage::{Render, markup};
use pretty_assertions::assert_eq;

#[test]
fn div() {
    let markup = markup! { div {} };
    assert_eq!("<div></div>", markup.render());
}

#[test]
fn div_text() {
    let markup = markup! { div { "Hello, World!" } };
    assert_eq!("<div>Hello, World!</div>", markup.render());
}

#[test]
fn div_span() {
    let markup = markup! { div { span { "Hello, World!" } } };
    assert_eq!("<div><span>Hello, World!</span></div>", markup.render());
}

#[test]
fn div_mul() {
    let markup = markup! { div { "One" } div { "Two" } };
    assert_eq!("<div>One</div><div>Two</div>", markup.render());
}

#[test]
fn div_mul_str() {
    let markup = markup! { div { "One" } "Two" };
    assert_eq!("<div>One</div>Two", markup.render());
}

#[test]
fn div_attr() {
    let markup = markup! { div id {} };
    assert_eq!(r#"<div id></div>"#, markup.render());
}

#[test]
fn div_attr_val() {
    let markup = markup! { div id="foo" {} };
    assert_eq!(r#"<div id="foo"></div>"#, markup.render());
}
