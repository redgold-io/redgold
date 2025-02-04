from typing import Annotated, List
from langchain.tools import tool

from agent_loop import CLAUDE_HAIKU_LATEST


@tool
def multiply_by_max(
    a: Annotated[int, "scale factor"],
    b: Annotated[List[int], "list of ints over which to take maximum"],
) -> int:
    """Multiply a by the maximum of b."""
    return a * max(b)

tools = [multiply_by_max]

# This gives you the JSON spec OpenAI expects
# openai_function = multiply_by_max.to_openai_function()
from typing import Literal

from langchain_anthropic import ChatAnthropic
from langgraph.graph import StateGraph, MessagesState
from langgraph.prebuilt import ToolNode


model_with_tools = ChatAnthropic(
    model=CLAUDE_HAIKU_LATEST, temperature=0
).bind_tools(tools)




