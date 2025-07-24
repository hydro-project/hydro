/// HTML template for ReactFlow visualization
pub const REACTFLOW_TEMPLATE: &str = include_str!("template.html");

pub fn get_template() -> &'static str {
    REACTFLOW_TEMPLATE
}
