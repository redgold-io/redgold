use llmclient::claude::{call_claude_completion, ClaudeCompletion};
use llmclient::gpt::GptMessage as ClaudeMessage;

// Switch to python for now

#[ignore]
#[tokio::test]
async fn debug() {
    let mut messages = Vec::new();

    // let input = "~/self/ai-prompts/self.txt";
    // let input = "/Volumes/self/ai-prompts/self.txt";

    let user_role = "user";
    let ast_role = "assistant";

    let system = r#"
    You are an AI agent.
    Your job is to receive chat messages from a user and then respond to them and do things.
    This prompt is just to test if you're able to call functions.
    "#;

    let model = "claude-3-5-sonnet-20240620";
    let temperature = 1.0;

    let test_user_msg = "What is the current weather like? \
    Please use your function tool to answer";

    messages.push(ClaudeMessage { role: user_role.into(), content: test_user_msg.to_string() });

    let mut functions = vec![];

    let function = Some(functions);
    let completion = ClaudeCompletion {
        model: model.into(),
        tools: function,
        system: Some(system.into()),
        messages,
        temperature,
        max_tokens: 4096
    };

    let result = call_claude_completion(&completion).await;

    let ret = result.expect("test");

    println!("{:?}", ret);


}