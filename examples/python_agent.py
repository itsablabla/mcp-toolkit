# OpenAI Agents SDK / Python MCP Client
#
# pip install openai-agents mcp
#
# This shows how to connect any Python-based agent to the MCP toolkit
# using the standard Streamable HTTP transport.

from agents.mcp import MCPServerStreamableHttp

# Connect to the remote MCP toolkit
mcp_server = MCPServerStreamableHttp(
    name="mcp-toolkit",
    url="https://mcp.garzaos.cloud/mcp",
)

# Use with OpenAI Agents SDK
from agents import Agent

agent = Agent(
    name="my-agent",
    instructions="You have access to MCP tools.",
    mcp_servers=[mcp_server],
)

# Or use the raw Python MCP client
from mcp.client.streamable_http import streamablehttp_client
import asyncio

async def main():
    async with streamablehttp_client("https://mcp.garzaos.cloud/mcp") as (read, write, _):
        from mcp import ClientSession
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            for tool in tools.tools:
                print(f"{tool.name}: {tool.description}")

asyncio.run(main())
