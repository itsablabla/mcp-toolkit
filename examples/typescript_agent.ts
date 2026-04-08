// TypeScript / Node.js MCP Client
//
// npm install @modelcontextprotocol/sdk
//
// Connect any TypeScript agent to the MCP toolkit using Streamable HTTP.

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

async function main() {
  const transport = new StreamableHTTPClientTransport(
    new URL("https://mcp.garzaos.cloud/mcp")
  );

  const client = new Client({
    name: "my-agent",
    version: "1.0.0",
  });

  await client.connect(transport);

  // List available tools
  const tools = await client.listTools();
  console.log("Available tools:");
  for (const tool of tools.tools) {
    console.log(`  ${tool.name}: ${tool.description}`);
  }

  // Call a tool
  // const result = await client.callTool({
  //   name: "tool-name",
  //   arguments: { key: "value" },
  // });

  await client.close();
}

main().catch(console.error);
