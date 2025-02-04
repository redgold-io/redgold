import os

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


BRAVE_API = os.environ.get("BRAVE")
SERVERS = {
  "mcpServers": {
    "brave-search": {
      "command": "/usr/local/bin/docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        f"BRAVE_API_KEY={BRAVE_API}",
        "mcp/brave-search"
      ],
      "env": {
        "BRAVE_API_KEY": BRAVE_API
      }
    }
  }
}

# print(os.environ.get("BRAVE"))

mcp_id = "brave-search"
params = SERVERS["mcpServers"][mcp_id]
command = params["command"]
args = params["args"]
env = params["env"]
# Create server parameters for stdio connection
server_params = StdioServerParameters(
    command=command,
    args=args,
    env=env
)

async def run():
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            # Initialize the connection
            await session.initialize()
            #
            # # List available prompts
            # prompts = await session.list_prompts()
            #
            # # Get a prompt
            # prompt = await session.get_prompt("example-prompt", arguments={"arg1": "value"})
            #
            # # List available resources
            # resources = await session.list_resources()

            # List available tools
            tools = await session.list_tools()
            print(tools)
            #
            # # Read a resource
            # content, mime_type = await session.read_resource("file://some/path")
            #
            # # Call a tool
            # result = await session.call_tool("tool-name", arguments={"arg1": "value"})

if __name__ == "__main__":
    import asyncio
    asyncio.run(run())