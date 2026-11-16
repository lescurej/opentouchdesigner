use super::*;

    #[test]
    fn a_kernel_shader_pipeline_builds() {
        let ctx = crate::GpuContext::headless().expect("gpu");
        let mut engine = TopEngine::new(ctx);
        let src = "\
layout(location = 0) out vec4 fragColor;
const float K[2][2] = float[2][2](float[2](1.0, 0.0), float[2](0.0, 4.0));
void main() {
    fragColor = TDOutputSwizzle(vec4(K[0][1], 0.0, 0.0, 1.0));
}
";
        engine.user_pipeline(src, true).expect("pipeline");
    }
