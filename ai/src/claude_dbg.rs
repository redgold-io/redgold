


#[tokio::test]
async fn debug() {
    let mut messages = Vec::new();

    let user_role = "user";
    let ast_role = "assistant";

    let ast_msg =
            messages.push(ClaudeMessage { role: role.into(), content: c.to_string() });
        });

    let completion = ClaudeCompletion {
        model: model.into(),
        tools: function,
        system: if system.is_empty() { None } else { Some(system.to_string()) },
        messages,
        temperature,
        max_tokens: 4096
    };

    call_claude_completion(&completion).await


}