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
from langchain_core.messages import AIMessage

# tools = [get_weather, get_coolest_cities]
tool_node = ToolNode(tools)

message_with_single_tool_call = AIMessage(
    content="",
    tool_calls=[
        {
            "name": "multiply_by_max",
            "args": {"a": 5, "b": [1, 2]},
            "id": "tool_call_id",
            "type": "tool_call",
        }
    ],
)

result = tool_node.invoke({"messages": [message_with_single_tool_call]})
print(result)

#
# model_with_tools = ChatAnthropic(
#     model=CLAUDE_HAIKU_LATEST, temperature=0
# ).bind_tools(tools)
#





