import requests as requests
from ollama import chat
from ollama import ChatResponse
from ollama import Client

from tools.tool_match import default_tooldefs_claude

#
# c = Client("http://server:7869")
#

tooldefs = default_tooldefs_claude()


def convert_to_ollama_format(claude_tooldef: dict) -> dict:
    """Convert Claude-style tooldef to Ollama format"""
    return {
        "name": claude_tooldef["name"],
        "description": claude_tooldef["description"],
        "parameters": claude_tooldef.get("input_schema", {})
    }

ollama_tooldefs = [convert_to_ollama_format(tool) for tool in tooldefs]

def deepseek_message(messages: list, tooldefs: list) -> ChatResponse:
    """Send a message to the Deepseek model with tools"""
    response: ChatResponse = chat(model='deepseek-r1:14b', messages=messages, tools=ollama_tooldefs)
    return response
# Use with your existing tooldefs:
ollama_tools = [convert_to_ollama_format(tool) for tool in tooldefs]
print(tooldefs)

response: ChatResponse = chat(model='deepseek-r1:14b', messages=[
    {
        'role': 'user',
        'content': 'Why is the sky blue?',
    },
])
print(response['message']['content'])
# or access fields directly from the response object
print(response.message.content)
