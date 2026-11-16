use otd_core::{EvalContext, Node, Value};

fn val(node: &Node, ctx: &EvalContext, key: &str) -> Value {
    node.param(key)
        .map(|p| p.eval(ctx))
        .unwrap_or(Value::Float(0.0))
}

pub fn f(node: &Node, ctx: &EvalContext, key: &str) -> f32 {
    val(node, ctx, key).as_f32()
}

pub fn b(node: &Node, ctx: &EvalContext, key: &str) -> bool {
    val(node, ctx, key).as_bool()
}

pub fn s(node: &Node, ctx: &EvalContext, key: &str) -> String {
    val(node, ctx, key).as_str()
}

pub fn v3(node: &Node, ctx: &EvalContext, key: &str) -> [f32; 3] {
    let v = val(node, ctx, key).as_vec4_f32();
    [v[0], v[1], v[2]]
}

pub fn v4(node: &Node, ctx: &EvalContext, key: &str) -> [f32; 4] {
    val(node, ctx, key).as_vec4_f32()
}

pub fn menu(node: &Node, ctx: &EvalContext, key: &str) -> usize {
    let Some(p) = node.param(key) else { return 0 };
    let chosen = p.eval(ctx).as_str();
    p.menu_index(&chosen)
}
