from langchain_anthropic import ChatAnthropic
from langchain_core.messages import HumanMessage
from typing import List, Dict, Any
import tiktoken

from agent_loop import CLAUDE_SONNET




def count_tokens(text: str) -> int:
    encoding = tiktoken.encoding_for_model(CLAUDE_SONNET)
    return len(encoding.encode(text))


def get_summary(model: ChatAnthropic, messages: List[Dict[str, Any]]):
    summary_request = "Please summarize this conversation and provide information to " \
                      "the next agent invocation which will not have access to this message history"
    messages.append(HumanMessage(content=summary_request))
    response = model.invoke(messages)
    return [summary_request, response]
