"""Minimal stand-in for Vectorworks' `vs` module, for testing outside VW.

Models a document with two layers, a few objects, classes and a resource
library on disk, enough to exercise vwmcp's pump and `vw` helpers.
"""
import os
import uuid as _uuid


class Handle:
    def __init__(self, t, name='', cls='None', layer=None, bbox=((0, 1), (1, 0))):
        self.t, self.name, self.cls, self.layer, self.bbox = t, name, cls, layer, bbox
        self.uuid = '{%s}' % str(_uuid.uuid4()).upper()


_layers = [Handle(31, 'Design Layer-1'), Handle(31, 'Sheet-1')]
_objects = [
    Handle(3, 'Stage', 'Set', _layers[0], ((0, 4), (8, 0))),
    Handle(15, '', 'Lighting', _layers[0], ((1, 3), (2, 2))),
]
_resources = {'Truss 3m': Handle(16, 'Truss 3m')}
_last = [None]
_lists = {}


def GetObjectUuid(h): return h.uuid
def GetObjectByUuid(u): return next((o for o in _objects + _layers if o.uuid == u), None)
def GetTypeN(h): return h.t if h is not None else 0
def GetName(h): return h.name
def SetName(h, n):
    _resources.pop(h.name, None)
    h.name = n
    if h.t == 16:
        _resources[n] = h
def GetClass(h): return h.cls
def GetLayer(h): return h.layer
def GetLName(h): return h.name
def GetBBox(h): return h.bbox
def GetSymName(h): return 'Spot Fixture'
def Selected(h): return False
def ForEachObject(cb, criteria):
    for o in list(_objects):
        if criteria == '(SEL=TRUE)':
            continue
        if criteria == '(T=RECT)' and o.t != 3:
            continue
        cb(o)
def Count(criteria):
    name = criteria.split("'")[1]
    return sum(1 for o in _objects if o.layer and o.layer.name == name)
def FLayer(): return _layers[0]
def NextLayer(h):
    i = _layers.index(h) + 1
    return _layers[i] if i < len(_layers) else None
def NumLayers(): return len(_layers)
def GetObjectVariableInt(h, idx): return 1 if h is _layers[0] else 2
def ClassNum(): return 3
def ClassList(i): return ['None', 'Set', 'Lighting'][i - 1]
def GetFPathName(): return r'C:\Shows\Rent.vwx'
def GetFName(): return 'Rent.vwx'
def ActLayer(): return _layers[0]
def ActiveClass(): return 'None'
def GetVersion(): return (31, 0, 1, 0)
def Oval(l, t, r, b):
    h = Handle(4, '', 'None', _layers[0], ((l, t), (r, b)))
    _objects.append(h)
    _last[0] = h
def LNewObj(): return _last[0]
def GetObject(name):
    return _resources.get(name) or next((o for o in _objects if o.name == name), Handle(0))
def GetFolderPath(i):
    base = os.environ['FAKE_VW_USER']
    return {-13: os.path.join(base, 'Libraries'), -23: os.path.join(base, 'Favorites'),
            -14: os.path.join(base, 'Defaults'), 13: os.path.join(base, 'App', 'Libraries')}.get(i, '')
def BuildResourceListN(t, path):
    names = [l.strip() for l in open(path, encoding='utf-8') if l.strip()]
    lid = len(_lists) + 1
    _lists[lid] = names
    return lid, len(names)
def BuildResourceList(t, where, folder):
    lid = len(_lists) + 1
    _lists[lid] = sorted(_resources)
    return lid, len(_lists[lid])
def GetActualNameFromResourceList(lid, i): return _lists[lid][i - 1]
def ImportResToCurFileN(lid, i, cb):
    # Real Vectorworks rejects callbacks without exactly one named parameter.
    if getattr(getattr(cb, '__code__', None), 'co_argcount', None) != 1:
        raise RuntimeError('Invalid number of parameters to the callback function. : index = 0 name=strParam')
    name = _lists[lid][i - 1]
    if name in _resources:
        if cb(name) == 1:
            _resources[name] = Handle(16, name)
        return _resources[name]
    _resources[name] = Handle(16, name)
    return _resources[name]
