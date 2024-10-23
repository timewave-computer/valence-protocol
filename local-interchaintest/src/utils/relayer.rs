use localic_utils::utils::test_context::TestContext;

pub fn restart_relayer(test_ctx: &mut TestContext) {
    test_ctx.stop_relayer();
    test_ctx.start_relayer();
}
