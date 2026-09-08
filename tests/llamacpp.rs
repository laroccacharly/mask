mod common;

#[test]
fn answers_capital_of_france() {
    let reply = common::timed("llamacpp", || {
        mask::llamacpp::complete("What is the capital of France?").expect("run llm")
    });
    assert!(
        reply.contains("Paris"),
        "expected Paris in response, got {reply:?}"
    );
}
