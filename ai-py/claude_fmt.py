from typing import Iterable

from anthropic.types import TextBlockParam, ToolResultBlockParam, MessageParam


def fmt_list(hits):
    return [TextBlockParam(
        text=k,
        type="text"
    ) for k in hits]


def tool_format(tool_response: Iterable[ToolResultBlockParam]):
    return MessageParam(
            role="user",
            content=tool_response
    )


def user_text_content(content):
    return {
        "role": "user",
        "content": [
            {
                "type": "text",
                "text": content
            }
        ]
    }
