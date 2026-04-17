# Testing SurfSense E2B MCP Integration

## Overview
SurfSense (jada2.garzaos.online) integrates E2B Code Interpreter as an MCP connector, providing 10 tools for cloud-based Python execution, file ops, and shell commands. The MCP server runs as a Docker container (`e2b-mcp`) on port 8101.

## Architecture
```
SurfSense UI -> LLM (via OpenRouter/Anthropic) -> MCP tool call -> E2B MCP server (port 8101) -> E2B cloud sandbox -> result
```

## Devin Secrets Needed
- `VPS_ROOT_PASSWORD` - SSH access to main VPS (${VPS_IP})
- `E2B_API_KEY` - E2B API key for sandbox creation
- SurfSense `SECRET_KEY` - for JWT token generation (stored in `/opt/surfsense/.env`)

## Authentication

### JWT Token Generation
SurfSense uses FastAPI-Users with JWT auth. Generate a token:
```python
import jwt, time
secret_key = "<from /opt/surfsense/.env SECRET_KEY>"
user_id = "<user UUID from surfsense DB>"
payload = {
    "sub": user_id,
    "aud": "fastapi-users:auth",
    "exp": int(time.time()) + 86400  # 24h
}
token = jwt.encode(payload, secret_key, algorithm="HS256")
```

### Browser Login
Inject JWT via URL: `https://jada2.garzaos.online/auth/callback?token=JWT&refresh_token=REFRESH`

Alternatively, use Nextcloud OAuth flow (requires Nextcloud credentials).

## Sending Chat Messages

### API Approach (More Reliable)
```bash
# 1. Create a thread
curl -s -L -X POST "https://jada2-api.garzaos.online/api/v1/threads" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"title":"Test Chat","search_space_id":8}'

# 2. Send message (SSE streaming response)
curl -s -L -N -X POST "https://jada2-api.garzaos.online/api/v1/new_chat" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"chat_id": THREAD_ID, "user_query": "your message", "search_space_id": 8}'
```

The response is SSE format with these key event types:
- `tool-input-available` - tool name + input args (proves MCP call was made)
- `tool-output-available` - status + result_length (proves sandbox executed)
- `text-delta` - LLM's text response chunks
- `data-thinking-step` - thinking/processing steps
- `finish` / `[DONE]` - stream complete

### Browser Approach
The SurfSense chat input is a `contenteditable` div. Standard `browser.type()` may not work reliably. **Workaround**: Use JavaScript `document.execCommand('insertText')` via browser console:
```javascript
const input = document.querySelector('[aria-label="Message input with inline mentions"]');
input.focus();
document.execCommand('insertText', false, 'your message here');
```
Then click the Send button (look for `aria-label="Send message"`).

Note: The first message in a new chat sometimes works with `browser.type()`, but follow-up messages in the same thread often fail. Opening a fresh new chat page for each test message is more reliable.

## Verifying E2B MCP Tools Directly

Test the MCP server without going through SurfSense:
```bash
# Initialize MCP session
curl -s -D /tmp/headers.txt -X POST "http://$VPS_IP:8101/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# Extract session ID
SESSION_ID=$(grep -i mcp-session-id /tmp/headers.txt | awk '{print $2}' | tr -d '\\r')

# List tools
curl -s -X POST "http://$VPS_IP:8101/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
```

Expected: 10 tools with `e2b_` prefix.

## Key Test Cases

1. **Tool Discovery**: Ask LLM to list E2B tools. Expect all 10 named.
2. **Code Execution**: `e2b_run_code` with verifiable output (e.g., `print([x**2 for x in range(1,6)])` -> `[1, 4, 9, 16, 25]`)
3. **File Workflow**: Multi-step write -> read -> exists to test sandbox persistence (5-min TTL pooling)
4. **Shell Command**: `e2b_run_command` with unique marker + computation to prove live execution

## Known Issues

- **Workspace 7 Anthropic credits might be depleted**: If you get `BadRequestError: credit balance is too low`, switch to Workspace 8 (Tech Research) which uses OpenRouter/GPT-4o.
- **E2B SDK v2.x API**: Uses `Sandbox.create()` not `Sandbox(api_key=...)`. The `E2B_API_KEY` env var is read automatically.
- **MCP connector count in UI might show fewer than expected**: The connector count badge in the sidebar may not include all connectors. Verify via database directly.
- **API-created threads may not show messages in browser**: Messages sent via POST /api/v1/new_chat stream correctly but may not appear when navigating to the thread in the browser UI.

## File Locations on VPS
- SurfSense: `/opt/surfsense/` (docker-compose)
- E2B MCP server: `/opt/e2b-mcp/` (docker-compose, port 8101)
- SurfSense DB: PostgreSQL in `surfsense-db` container
- E2B MCP internal URL: `http://172.16.28.1:8101/mcp`
- Workspace defaults: `/opt/surfsense/surfsense_backend/src/api/workspace_defaults.py`
