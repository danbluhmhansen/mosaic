use collage::markup;

fn main() {
    markup! { div r#"&<>""#="" {} };
    markup! { div ("!id") {} };
    markup! { div (if true { "id" } else { "!id" }) {} };
    markup! { div (match true { true => { "id" } false => { "!id" } }) {} };
    let key = "id";
    markup! { div (key) {} };
}
