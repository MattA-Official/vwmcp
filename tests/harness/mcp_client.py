"""Tiny MCP stdio client: runs a list of tool calls and prints the results."""
import json, subprocess, sys

proc = subprocess.Popen([sys.argv[1], 'serve'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
_id = 0
def rpc(method, params=None, notify=False):
    global _id
    msg = {'jsonrpc': '2.0', 'method': method}
    if params is not None: msg['params'] = params
    if not notify:
        _id += 1; msg['id'] = _id
    proc.stdin.write(json.dumps(msg) + '\n'); proc.stdin.flush()
    if notify: return None
    while True:
        line = proc.stdout.readline()
        if not line: raise SystemExit('server closed stdout')
        r = json.loads(line)
        if r.get('id') == _id: return r

init = rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'harness', 'version': '0'}})
print('SERVER', init['result']['serverInfo'], init['result']['protocolVersion'])
rpc('notifications/initialized', notify=True)
tools = rpc('tools/list', {})['result']['tools']
print('TOOLS', len(tools), sorted(t['name'] for t in tools))
for call in json.loads(sys.argv[2]):
    r = rpc('tools/call', {'name': call[0], 'arguments': call[1]})
    res = r.get('result') or r
    text = '\n'.join(c.get('text', '<%s %d bytes>' % (c['type'], len(c.get('data', '')))) for c in res.get('content', []))
    print('=== %s %s error=%s\n%s' % (call[0], json.dumps(call[1])[:80], res.get('isError'), text[:1500]))
proc.stdin.close(); proc.wait(timeout=5)
