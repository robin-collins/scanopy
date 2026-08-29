//! Guards the confirmed requirement that font size has a 10px floor and NO
//! application-level ceiling (user-requested tasklist item 1, commit 8146ce215).
//!
//! This asserts on the source of the validation attributes rather than
//! constructing the request structs, which carry too many unrelated required
//! fields for the assertion to stay readable. The failure this guards against
//! is a silent one: an earlier change replaced `max = 72` with `max = 1000`,
//! which still rejected valid sizes while looking like the ceiling had been
//! removed.

const VIEW_BASE: &str = include_str!("../server/custom_topology_views/impl/base.rs");
const NODE_BASE: &str = include_str!("../server/custom_view_nodes/impl/base.rs");

fn font_size_validate_attribute(src: &str) -> &str {
    let anchor = src
        .find("pub default_font_size")
        .or_else(|| src.find("pub font_size"))
        .expect("a font_size field must exist");
    let attr_start = src[..anchor]
        .rfind("#[validate(range(")
        .expect("font_size must carry a validate(range) attribute");
    let attr_end = attr_start
        + src[attr_start..]
            .find(")]")
            .expect("unterminated validate attribute")
        + 2;
    &src[attr_start..attr_end]
}

#[test]
fn font_size_keeps_its_ten_pixel_floor() {
    for src in [VIEW_BASE, NODE_BASE] {
        assert!(
            font_size_validate_attribute(src).contains("min = 10"),
            "the 10px minimum is a confirmed requirement and must stay"
        );
    }
}

#[test]
fn font_size_has_no_application_level_ceiling() {
    for src in [VIEW_BASE, NODE_BASE] {
        let attr = font_size_validate_attribute(src);
        assert!(
            !attr.contains("max"),
            "font size must have no application-level upper limit; found: {attr}"
        );
    }
}
