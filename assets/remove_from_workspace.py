# vwmcp: remove the "vwmcp" command from the current workspace.
import vs

TOOLS = '6C98F4E0-9E8E-11DB-B5DD-000F1F82BA93'


def walk(path, depth=0):
    try:
        n = vs.ws2GetMenusCnt(path)
    except Exception:
        return
    for i in range(n):
        u = vs.ws2GetMenuAt(path, i)
        p = path + '/' + u
        yield p, u
        if depth < 3:
            yield from walk(p, depth + 1)


for p, u in list(walk(TOOLS)):
    if u == 'vwmcp':
        vs.ws2DelMenu(p)
vs.ws2CommitChanges(False, True)
