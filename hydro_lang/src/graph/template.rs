/// HTML template for JSON graph visualization
pub const JSON_TEMPLATE: &str = include_str!("template.html");

pub fn get_template() -> &'static str {
    JSON_TEMPLATE
}
