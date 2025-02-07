from langchain_anthropic import ChatAnthropic
from agent_loop import CLAUDE_HAIKU_LATEST
from tools.lc_tools_v2 import TOOLS
from prompts import UNSUPERVISED_PROMPT
from langchain_core.messages import HumanMessage, AIMessage, SystemMessage


# Import relevant functionality
from langchain_anthropic import ChatAnthropic
from langchain_community.tools.tavily_search import TavilySearchResults
from langchain_core.messages import HumanMessage
from langgraph.checkpoint.memory import MemorySaver
from langgraph.prebuilt import create_react_agent


def main():

    # Create the agent
    memory = MemorySaver()
    model = ChatAnthropic(
        model=CLAUDE_HAIKU_LATEST, 
        temperature=0
    )
    #.bind_tools(TOOLS)

    search = TavilySearchResults(max_results=2)
    tools = TOOLS
    tools.append(search)
    agent_executor = create_react_agent(model, tools, checkpointer=memory)

    # Use the agent
    config = {"configurable": {"thread_id": "abc123"}}

    messages = [
        SystemMessage(content=UNSUPERVISED_PROMPT),
        HumanMessage(content="Be a programmer, write code, and commit it to the repository.")
    ]
    i = 0
    max_model_runs = 100

    for chunk in agent_executor.stream(
        {"messages": messages}, config
    ):
        print(chunk)
        print("----")
        i += 1
        if i >= max_model_runs:
            break


if __name__ == "__main__":
    main()
