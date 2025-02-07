from langchain_anthropic import ChatAnthropic
from agent_loop import CLAUDE_HAIKU_LATEST
from tools.lc_tools_v2 import TOOLS
from prompts import UNSUPERVISED_PROMPT
from langchain_core.messages import HumanMessage, AIMessage, SystemMessage

def main():
    model_with_tools = ChatAnthropic(
        model=CLAUDE_HAIKU_LATEST, temperature=0
    ).bind_tools(TOOLS)

    # Initialize with system message and starting prompt
    messages = [
        SystemMessage(content=UNSUPERVISED_PROMPT),
        HumanMessage(content="Be a programmer, write code, and commit it to the repository.")
    ]
    
    i = 0
    max_model_runs = 100

    while i < max_model_runs:
        print(f"\n=== Turn {i} ===")
        response = model_with_tools.invoke(messages)
        print(response)
        
        # Add the assistant's response to history
        messages.append(AIMessage(content=response.content))
        
        # Check for completion conditions
        if "task completed" in response.content.lower() or "finished" in response.content.lower():
            print("Task completed naturally.")
            break
            
        # If the model used tools, we should continue the conversation
        if hasattr(response, 'tool_calls') and response.tool_calls:
            tool_results = []
            for tool_call in response.tool_calls:
                # Here you would handle tool results, but for now we'll just note it
                tool_results.append(f"Tool {tool_call.name} was called")
            
            # Add tool results to history
            messages.append(HumanMessage(content=f"Tool results: {', '.join(tool_results)}"))
        else:
            # No tools used, ask for next steps
            messages.append(HumanMessage(content="What would you like to do next?"))
        
        i += 1

    if i >= max_model_runs:
        print("Reached maximum number of turns.")

if __name__ == "__main__":
    main()
