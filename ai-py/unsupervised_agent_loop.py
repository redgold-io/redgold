from langchain_anthropic import ChatAnthropic
from agent_loop import CLAUDE_HAIKU_LATEST, CLAUDE_SONNET
from tools.lc_tools_v2 import TOOLS
from prompts import UNSUPERVISED_PROMPT
from langchain_core.messages import HumanMessage, AIMessage, SystemMessage
from typing import List, Dict, Any
import tiktoken
from langchain.agents import AgentExecutor


# Import relevant functionality
from langchain_anthropic import ChatAnthropic
from langchain_community.tools.tavily_search import TavilySearchResults
from langchain_core.messages import HumanMessage
from langgraph.checkpoint.memory import MemorySaver
from langgraph.prebuilt import create_react_agent


def count_tokens(text: str) -> int:
    encoding = tiktoken.encoding_for_model("claude-3-haiku-20240307")
    return len(encoding.encode(text))


def get_summary(model: ChatAnthropic, messages: List[Dict[str, Any]]) -> str:
    summary_request = "Please summarize this conversation and provide information to " \
                      "the next agent invocation which will not have access to this message history"
    messages.append(HumanMessage(content=summary_request))
    response = model.invoke(messages)
    return response.content


def main():

    # Create the agent
    memory = MemorySaver()
    model = ChatAnthropic(
        model=CLAUDE_SONNET,
        temperature=0
    )
    #.bind_tools(TOOLS)

    # search = TavilySearchResults(max_results=2)
    tools = TOOLS
    # tools.append(search)
    max_model_runs = 300

    agent_executor = create_react_agent(
        model,
        tools,
        # checkpointer=memory
    )
    # agent = AgentExecutor(agent=model, tools=tools, memory=memory)

    # Use the agent
    config = {"configurable": {"thread_id": "abc123"}, "recursion_limit": max_model_runs}

    initial_messages = [
        SystemMessage(content=UNSUPERVISED_PROMPT),
        HumanMessage(content="Be a programmer, write code, and commit it to the repository. You may "
                             "have existing code in progress in your workspace, please check your current diff "
                             "to see if you want to continue or reset. You can also check on your AI user's active PRs "
                             "and continue working on one of them if you'd like.")
    ]
    messages = []
    messages.extend(initial_messages)
    i = 0

    total_token_usage = 0
    max_tokens_before_summary = 50000

    while True:

        max_runs_exceeded = False

        for chunk in agent_executor.stream(
            {"messages": messages}, config,
        ):
            if isinstance(chunk, dict) and "actions" in chunk:
                # Track tokens from the response
                for action in chunk["actions"]:
                    if isinstance(action, dict):
                        total_token_usage += count_tokens(str(action))

            end_turn_detected = False
            # Track all messages from the agent output
            if isinstance(chunk, dict) and "agent" in chunk and "messages" in chunk["agent"]:
                for msg in chunk["agent"]["messages"]:
                    messages.append(msg)
                    # Check for end turn in the message's metadata
                    if hasattr(msg, "response_metadata") and msg.response_metadata.get("stop_reason") == "end_turn":
                        print("End of turn detected")
                        end_turn_detected = True
                        # Add any additional end of turn handling here if needed

            print(chunk)
            print("----")
            i += 1
            
            if i >= max_model_runs:
                max_runs_exceeded = True
                break
                
                
            if total_token_usage > max_tokens_before_summary or end_turn_detected:
                # Get summary of conversation
                summary = get_summary(model, messages)
                
                # Reset conversation with summary
                messages = [
                    SystemMessage(content=UNSUPERVISED_PROMPT),
                    HumanMessage(content=f"Previous conversation summary: {summary}\n\nPlease continue the programming task.")
                ]
                total_token_usage = sum(count_tokens(msg.content) for msg in messages)

        if max_runs_exceeded:
            print("Max runs exceeded")
            break


if __name__ == "__main__":
    main()
