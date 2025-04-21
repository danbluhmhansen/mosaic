use collage::{Render, markup};
use pretty_assertions::assert_eq;

#[test]
fn text() {
    let markup = markup! { r#"&<>"'/"# };
    assert_eq!(r#"&amp;&lt;&gt;"'/"#, markup.render());
}

#[test]
fn attr_val() {
    let markup = markup! { div id=r#"&<>""# {} };
    assert_eq!(r#"<div id="&amp;&lt;&gt;&quot;"></div>"#, markup.render());
}

#[test]
fn expr() {
    let markup = markup! { (r#"&<>"'/"#) };
    assert_eq!("&amp;&lt;&gt;&quot;&#x27;&#x2F;", markup.render());
}

#[test]
fn expr_attr_val() {
    let markup = markup! { div id=(r#"&<>"'/"#) {} };
    assert_eq!(r#"<div id="&amp;&lt;&gt;&quot;&#x27;&#x2F;"></div>"#, markup.render());
}
