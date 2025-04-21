use collage::{Render, markup};
use pretty_assertions::assert_eq;

#[test]
fn num() {
    let markup = markup! { (1) };
    assert_eq!("1", markup.render());
}

#[test]
fn for_loop() {
    let markup = markup! { ul { (for item in ["One", "Two", "Three"] { markup! { li { (item) } } }) } };
    assert_eq!("<ul><li>One</li><li>Two</li><li>Three</li></ul>", markup.render());
}

#[test]
fn part() {
    let markup = markup! { div { (collage::markup! { "One" }) } };
    assert_eq!("<div>One</div>", markup.render());
}

#[test]
fn elt_some() {
    let markup = markup! { div { (if true { "true" }) } };
    assert_eq!("<div>true</div>", markup.render());
}

#[test]
fn elt_none() {
    let markup = markup! { div { (if false { "false" }) } };
    assert_eq!("<div></div>", markup.render());
}

#[test]
fn attr_key() {
    let markup = markup! { div ("id")="one" "foo"="bar" {} };
    assert_eq!(r#"<div id="one" foo="bar"></div>"#, markup.render());
}

#[test]
fn attr_val() {
    let markup = markup! { div id=(123) {} };
    assert_eq!(r#"<div id="123"></div>"#, markup.render());
}

#[test]
fn attr_some() {
    let markup = markup! { div (if true { "id" }) {} };
    assert_eq!("<div id></div>", markup.render());
}

#[test]
fn attr_none() {
    let markup = markup! { div (if false { "id" }) {} };
    assert_eq!("<div></div>", markup.render());
}

#[test]
fn r#match() {
    let role = "user";
    let markup = markup! {
        (match role {
            "admin" => collage::markup! { p { "foo" } },
            "user" => collage::markup! { p { "bar" } },
        })
    };
    assert_eq!("<p>bar</p>", markup.render());
}
