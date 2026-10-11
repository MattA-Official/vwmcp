# Scripting inside Vectorworks

What a `run_python` script sees, and what it can return.

## Globals

| Name | |
|---|---|
| `vs` | The Vectorworks API. |
| `params` | The `params` object passed to `run_python`, as a dict (empty if none). |
| `vw` | vwmcp's helper module (below). |
| `__result__` | Assign to return a value. Starts as `None`. |

`print()` output is returned as `stdout`, up to 200,000 characters.

## Return values

`__result__` is converted to JSON:

| Python | JSON |
|---|---|
| `None`, `bool`, `int`, `str` | as is |
| `float` | number; `nan` and `inf` become `null` |
| `dict` | object (keys become strings) |
| `list`, `tuple`, `set` | array |
| Vectorworks handle | `{"handle": "<UUID>"}` |
| anything else | its `repr()` |

## Errors

An uncaught exception ends the script. The result reports `error` (`Type: message`) and a `traceback` showing only the script's own frames. Changes made before the exception are not rolled back; use Undo in Vectorworks to remove them.

## The `vw` helpers

### Addressing

| Function | Returns |
|---|---|
| `vw.uuid(h)` | UUID string of a handle, or `None`. |
| `vw.obj(uuid)` | Handle for a UUID, or `None`. |
| `vw.named(name)` | Handle for a named object or resource, or `None`. |
| `vw.last()` | The last object this script created (`vs.LNewObj()`). |
| `vw.find(criteria, limit=None)` | List of handles matching a criteria string. Collected before returning, so it's safe to change them afterwards. |

### Description

| Function | Returns |
|---|---|
| `vw.info(h, extra=False)` | `{id, type, name, class, layer, bbox[, symbol][, plugin][, selected]}` |
| `vw.query(criteria, limit=200, extra=False)` | `{criteria, total, objects: [info…]}` |
| `vw.layers()` | `[{name, kind}]` with `kind` `design` or `sheet` |
| `vw.classes()` | List of class names. |
| `vw.overview(count_objects=True)` | What `document_overview` returns. |
| `vw.TYPE_NAMES` | Object type number → name, e.g. `3: 'rectangle'`. |

### Geometry

| Function | |
|---|---|
| `vw.circle(cx, cy, r)` | Draws a circle and returns it. (`vs.ArcByCenter` draws nothing on 2026+.) |
| `vw.arc(cx, cy, r, start, sweep)` | Draws an arc from `start` through `sweep` degrees and returns it. |

### Resources and libraries

| Function | |
|---|---|
| `vw.resources(type='symbol', path=None, filter=None)` | Resource names in the active document, or in the saved `.vwx` at `path`. |
| `vw.resource_type(t)` | Type alias or number → Vectorworks type number. |
| `vw.library_folders()` | `{user_libraries, user_favorites, user_defaults, app_libraries}` |
| `vw.library_files(max_files=500)` | `.vwx` files in the user Libraries and Favorites folders. |
| `vw.import_resources(path, names, type='symbol', on_conflict='skip')` | As [`library_import`](tools.md#library_import). |

### Resource types

| Alias | Number |
|---|---|
| `symbol`, `style` | 16 |
| `worksheet` | 18 |
| `material` | 19 |
| `record` | 47 |
| `hatch` | 66 |
| `folder` | 92 |
| `line_type` | 96 |
| `texture` | 97 |
| `text_style` | 109 |
| `image_fill` | 119 |
| `gradient` | 120 |
| `wall_style` | 127 |

Plurals and spaces are accepted (`textures`, `wall style`), as is any type number.
