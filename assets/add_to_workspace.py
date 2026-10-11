# vwmcp: put the "vwmcp" command in the current workspace and give it its
# shortcut. Run by Vectorworks' Install Partner Products after it has copied
# the plug-in. KEY and MODS are filled in by `vwmcp install`.
import vs

TOOLS = '6C98F4E0-9E8E-11DB-B5DD-000F1F82BA93'  # the Tools menu
NAME = 'vwmcp'
KEY = '{{KEY}}'
MODS = {{MODS}}  # 1 = Ctrl, 2 = Shift, 4 = Alt
# Earlier bridge commands that may hold the same shortcut.
OLD_NAMES = ('VW MCP', 'VWX Bridge Start')


def children(path):
    try:
        return [vs.ws2GetMenuAt(path, i) for i in range(vs.ws2GetMenusCnt(path))]
    except Exception:
        return []


def walk(path, depth=0):
    for u in children(path):
        p = path + '/' + u
        yield p, u
        if depth < 3:
            yield from walk(p, depth + 1)


def find(name):
    for p, u in walk(TOOLS):
        if u == name:
            return p
    return None


# vwmcp's own earlier name: the menu item points at a plug-in that no longer
# exists, so remove it.
for p, u in list(walk(TOOLS)):
    if u == 'VW MCP':
        vs.ws2DelMenu(p)

# Free the shortcut from vwx-mcp's command, so the keys aren't shared.
for p, u in walk(TOOLS):
    if u in OLD_NAMES:
        try:
            title, has, key, mods = vs.ws2GetMenuInfo(p)
            if has and str(key).upper() == KEY and mods == MODS:
                vs.ws2SetMenuInfo(p, title, False, '', 0)
        except Exception:
            pass

path = find(NAME)
if path is None:
    parent = TOOLS + '/Utilities' if 'Utilities' in children(TOOLS) else TOOLS
    if vs.ws2AddMenuItem(parent, NAME, -1):
        path = parent + '/' + NAME
if path is None:
    # Fallback: Tools > Third-party, the documented partner location.
    vs.wsEditBegin('vwmcp')
    vs.wsEditAddMenu(NAME)
    vs.wsEditEnd(False)
    path = find(NAME)
if path is not None:
    vs.ws2SetMenuInfo(path, NAME, True, KEY, MODS)
vs.ws2CommitChanges(False, True)
