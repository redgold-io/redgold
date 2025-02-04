# server.py
import json

from fastmcp import FastMCP


# Create an MCP server
mcp = FastMCP("Demo")


# Add an addition tool
@mcp.tool()
def add(a: int, b: int) -> int:
    """Add two numbers"""
    return a + b


# Add a dynamic greeting resource
@mcp.resource("greeting://{name}")
def get_greeting(name: str) -> str:
    """Get a personalized greeting"""
    return f"Hello, {name}!"



all_schemas = {name: tool.schema for name, tool in mcp.tool.items()}
print(json.dumps(all_schemas, indent=2))


# import mcp
# from pydantic import BaseModel, Field
#
# class ExampleParams(BaseModel):
#     message: str = Field(..., description="A test message to echo")
#     count: int = Field(default=1, description="Number of times to repeat")
#
# @mcp.tool()
# def echo_message(params: ExampleParams) -> str:
#     """A simple example tool that echoes a message multiple times."""
#     return params.message * params.count
#
# # To get the OpenAPI/JSON schema directly:
# schema = echo_message.get_schema()
# print(schema)
#

