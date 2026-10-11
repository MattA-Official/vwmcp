"""`vw` — helpers available to every vwmcp script as the global `vw`.

They wrap Vectorworks 2026 calls whose raw forms fail silently or hang
(see the gotchas listed in the run_python tool description). Every helper
works on plain values: handles in, handles or dicts out.
"""
import os

import vs

# --- object types ---------------------------------------------------------

TYPE_NAMES = {
    2: 'line', 3: 'rectangle', 4: 'oval', 5: 'polygon', 6: 'arc', 8: 'freehand',
    9: 'locus3d', 10: 'text', 11: 'group', 13: 'rounded_rect', 14: 'bitmap',
    15: 'symbol', 16: 'symbol_definition', 17: 'locus', 18: 'worksheet',
    19: 'material', 21: 'polyline', 22: 'pict', 24: 'extrude', 25: 'polygon3d',
    29: 'layer_link', 31: 'layer', 34: 'sweep', 38: 'multiple_extrude',
    40: 'mesh', 47: 'record_format', 48: 'record', 56: 'worksheet_on_drawing',
    63: 'dimension', 66: 'hatch', 68: 'wall', 71: 'slab', 81: 'light',
    83: 'roof', 84: 'csg_solid', 86: 'plugin_object', 87: 'roof_element',
    92: 'symbol_folder', 94: 'class', 95: 'solid', 96: 'line_type',
    97: 'texture', 102: 'roof_style', 107: 'slab_style', 109: 'text_style',
    111: 'nurbs_curve', 113: 'nurbs_surface', 119: 'image_fill',
    120: 'gradient', 122: 'viewport', 127: 'wall_style',
}

# Resource types accepted by BuildResourceList / BuildResourceListN.
RESOURCE_TYPES = {
    'symbol': 16, 'style': 16, 'worksheet': 18, 'material': 19,
    'record': 47, 'hatch': 66, 'folder': 92, 'line_type': 96,
    'texture': 97, 'text_style': 109, 'image_fill': 119, 'gradient': 120,
    'wall_style': 127,
}


def _safe(fn, default=None):
    try:
        return fn()
    except Exception:
        return default


def resource_type(t):
    """Accept an alias ('symbol', 'texture', …) or a raw integer type."""
    if t is None:
        return 16
    if isinstance(t, int):
        return t
    s = str(t).strip().lower().replace('-', '_').replace(' ', '_')
    if s.isdigit():
        return int(s)
    if s.endswith('s') and s[:-1] in RESOURCE_TYPES:
        s = s[:-1]
    if s not in RESOURCE_TYPES:
        raise ValueError('unknown resource type %r; use one of %s or an integer'
                         % (t, ', '.join(sorted(RESOURCE_TYPES))))
    return RESOURCE_TYPES[s]


# --- addressing -----------------------------------------------------------

def uuid(h):
    """UUID string of a handle (the stable object address on VW2026)."""
    return _safe(lambda: vs.GetObjectUuid(h))


def obj(object_id):
    """Handle for a UUID, or None."""
    h = _safe(lambda: vs.GetObjectByUuid(object_id))
    if h is None or _safe(lambda: vs.GetTypeN(h), 0) == 0:
        return None
    return h


def named(name):
    """Handle for a named object/resource, or None.
    vs.GetObject returns a dummy type-0 handle for unknown names."""
    h = _safe(lambda: vs.GetObject(name or ''))
    if h is None or _safe(lambda: vs.GetTypeN(h), 0) == 0:
        return None
    return h


def last():
    """The last object created by this script (vs.LNewObj)."""
    return vs.LNewObj()


def find(criteria, limit=None):
    """Handles matching a criteria string, e.g. "(T=RECT) & (C='Walls')".
    Collected first, so it is safe to mutate the results afterwards."""
    out = []
    vs.ForEachObject(lambda h: out.append(h), criteria)
    return out[:limit] if limit else out


# --- description ----------------------------------------------------------

def info(h, extra=False):
    """Plain-dict description of an object."""
    t = _safe(lambda: vs.GetTypeN(h), 0)
    d = {
        'id': uuid(h),
        'type': TYPE_NAMES.get(t, t),
        'name': _safe(lambda: vs.GetName(h)) or None,
        'class': _safe(lambda: vs.GetClass(h)),
        'layer': _safe(lambda: vs.GetLName(vs.GetLayer(h))),
    }
    bb = _safe(lambda: vs.GetBBox(h))
    if bb:
        (x1, y1), (x2, y2) = bb
        d['bbox'] = [round(min(x1, x2), 6), round(min(y1, y2), 6),
                     round(max(x1, x2), 6), round(max(y1, y2), 6)]
    if t == 15:
        d['symbol'] = _safe(lambda: vs.GetSymName(h))
    if t == 86:
        d['plugin'] = _safe(lambda: vs.GetName(vs.GetParametricRecord(h)))
    if extra:
        d['selected'] = bool(_safe(lambda: vs.Selected(h), False))
    return d


def layers():
    out = []
    h = vs.FLayer()
    seen = 0
    total = _safe(vs.NumLayers, 10_000)
    while h is not None and seen < total:  # bounded: raw handle walks can spin
        lt = _safe(lambda: vs.GetObjectVariableInt(h, 154))
        out.append({'name': vs.GetLName(h),
                    'kind': {1: 'design', 2: 'sheet'}.get(lt, lt)})
        h = vs.NextLayer(h)
        seen += 1
    return out


def classes():
    return [vs.ClassList(i) for i in range(1, vs.ClassNum() + 1)]


def overview(count_objects=True):
    """Summary of the active document."""
    ls = layers()
    if count_objects:
        for l in ls:
            name = l['name'].replace("'", "''")
            l['objects'] = _safe(lambda: vs.Count("(L='%s')" % name))
    return {
        'file': _safe(vs.GetFPathName) or None,
        'name': _safe(vs.GetFName),
        'active_layer': _safe(lambda: vs.GetLName(vs.ActLayer())),
        'active_class': _safe(vs.ActiveClass),
        'layers': ls,
        'class_count': _safe(vs.ClassNum),
        'selected': len(find('(SEL=TRUE)')),
    }


def query(criteria, limit=200, extra=False):
    hs = find(criteria)
    return {'criteria': criteria, 'total': len(hs),
            'objects': [info(h, extra) for h in hs[:limit]]}


# --- geometry that is broken in its obvious form on VW2026 ---------------

def circle(cx, cy, r):
    """vs.ArcByCenter silently draws nothing on VW2026; use a bbox oval."""
    vs.Oval(cx - r, cy + r, cx + r, cy - r)
    return vs.LNewObj()


def arc(cx, cy, r, start, sweep):
    """vs.Arc's last argument is the sweep, not the end angle."""
    vs.Arc(cx - r, cy + r, cx + r, cy - r, start, sweep)
    return vs.LNewObj()


# --- resources and libraries ---------------------------------------------

def resources(rtype='symbol', path=None, filter=None):
    """Names of resources of one type, in the active document or a .vwx file."""
    t = resource_type(rtype)
    if path:
        lid, n = vs.BuildResourceListN(t, path)
    else:
        lid, n = vs.BuildResourceList(t, 0, '')
    names = [vs.GetActualNameFromResourceList(lid, i) for i in range(1, n + 1)]
    if filter:
        f = filter.lower()
        names = [x for x in names if f in x.lower()]
    return names


def library_folders():
    """Where resource libraries live. Negative indexes are the user's folders
    (writable); positive ones are the application's (read-only)."""
    return {
        'user_libraries': _safe(lambda: vs.GetFolderPath(-13)),
        'user_favorites': _safe(lambda: vs.GetFolderPath(-23)),
        'user_defaults': _safe(lambda: vs.GetFolderPath(-14)),
        'app_libraries': _safe(lambda: vs.GetFolderPath(13)),
    }


def library_files(max_files=500):
    """.vwx files in the user library and favourites folders."""
    out = {}
    for key in ('user_libraries', 'user_favorites'):
        root = library_folders().get(key)
        files = []
        if root and os.path.isdir(root):
            for dp, _, fs in os.walk(root):
                for f in fs:
                    if f.lower().endswith('.vwx'):
                        files.append(os.path.join(dp, f))
                        if len(files) >= max_files:
                            break
        out[key] = {'folder': root, 'files': sorted(files)}
    return out


def import_resources(path, names, rtype='symbol', on_conflict='skip'):
    """Import named resources from a saved .vwx into the active document.

    on_conflict: 'skip'    keep the document's version (VW also skips exact copies)
                 'replace' overwrite the document's version
                 'rename'  keep both; the import gets '<name>-2' (or -3, …)
    """
    t = resource_type(rtype)
    if on_conflict not in ('skip', 'replace', 'rename'):
        raise ValueError("on_conflict must be 'skip', 'replace' or 'rename'")
    lid, n = vs.BuildResourceListN(t, path)
    available = [vs.GetActualNameFromResourceList(lid, i) for i in range(1, n + 1)]
    results = []
    for name in names:
        if name not in available:
            results.append({'name': name, 'status': 'not_found'})
            continue
        idx = available.index(name) + 1
        existing = named(name)
        # Vectorworks counts the callback's named parameters and expects
        # exactly one (the resource name); `lambda *a` is rejected.
        if existing is not None and on_conflict == 'rename':
            new_name = _free_name(name)
            # Temp-rename the document's copy so the import cannot conflict,
            # then swap names back. Python's VAR-parameter rename callback
            # is undocumented, so this avoids relying on it.
            tmp = '%s__vwmcp_tmp' % name
            vs.SetName(existing, tmp)
            try:
                h = vs.ImportResToCurFileN(lid, idx, _skip)
                if h is not None:
                    vs.SetName(h, new_name)
            finally:
                vs.SetName(existing, name)
            results.append({'name': name, 'status': 'imported' if h else 'failed',
                            'imported_as': new_name if h else None})
            continue
        h = vs.ImportResToCurFileN(lid, idx, _replace if on_conflict == 'replace' else _skip)
        if existing is None:
            status = 'imported' if h is not None else 'failed'
        else:
            # VW skips exact duplicates without asking, so 'replace' may
            # also mean "already identical".
            status = 'replaced_or_identical' if on_conflict == 'replace' else 'kept_existing'
        results.append({'name': name, 'status': status})
    return results


def _skip(strParam):
    return 0  # eImportResourceConflictResult_DoNotImport


def _replace(strParam):
    return 1  # eImportResourceConflictResult_Replace


def _free_name(base):
    i = 2
    while named('%s-%d' % (base, i)) is not None:
        i += 1
    return '%s-%d' % (base, i)
