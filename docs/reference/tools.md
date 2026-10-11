# MCP tools

vwmcp exposes 12 tools. Annotations tell clients which tools only read.

| Tool | Reads only | Needs Vectorworks |
|---|---|---|
| [`status`](#status) | yes | no (reports if it's missing) |
| [`run_python`](#run_python) | no | yes |
| [`job_result`](#job_result) | yes | no |
| [`vs_lookup`](#vs_lookup) | yes | no |
| [`vs_docs`](#vs_docs) | yes | no |
| [`document_overview`](#document_overview) | yes | yes |
| [`query_objects`](#query_objects) | yes | yes |
| [`screenshot`](#screenshot) | yes | yes |
| [`library_list`](#library_list) | yes | yes |
| [`library_browse`](#library_browse) | yes | yes |
| [`library_import`](#library_import) | no | yes |
| [`library_install`](#library_install) | no | yes (asks it for the folder) |

Tools that run inside Vectorworks report `ok`, `result`, `stdout` and `elapsed_ms`. On failure they report `error` and `traceback` instead, and are marked as errors.

## `status`

Checks the whole chain: whether Vectorworks is running, minimised or blocked by a dialog, when the pump last ran, and a live round trip.

No parameters.

## `run_python`

Runs Python on Vectorworks' main thread, as one undo step. See [Scripting inside Vectorworks](scripting.md).

| Parameter | Type | Default | |
|---|---|---|---|
| `code` | string | required | The script. |
| `params` | object | `{}` | Available to the script as `params`. |
| `timeout_s` | integer | 120 | How long to wait once the script has started (1–3600). |

If the script outlives `timeout_s`, the error includes a job id for [`job_result`](#job_result). If Vectorworks never picks the job up (within 15 s), the job is withdrawn and nothing runs.

## `job_result`

| Parameter | Type | |
|---|---|---|
| `id` | string | Job id from a `run_python` timeout message. |

Each result can be collected once.

## `vs_lookup`

Exact signatures from Vectorworks' published `vs.py` stub. Doesn't need Vectorworks.

| Parameter | Type | Default | |
|---|---|---|---|
| `query` | string | required | A function name (`Oval`, `vs.GetBBox`) or words (`wall height`). |
| `limit` | integer | 15 | Maximum results (1–100). |

An exact name returns the full signature, argument notes and related functions. Words return a ranked list.

## `vs_docs`

| Parameter | Type | |
|---|---|---|
| `name` | string | Exact function name. |

The full reference page (description, parameters, remarks, examples), up to 16,000 characters.

## `document_overview`

| Parameter | Type | Default | |
|---|---|---|---|
| `count_objects` | boolean | true | Count objects per layer. |

Returns file path and name, active layer and class, layers (`design`/`sheet`) with counts, class count and selection size.

## `query_objects`

| Parameter | Type | Default | |
|---|---|---|---|
| `criteria` | string | required | A Vectorworks criteria string, e.g. `(T=RECT)`, `(C='Walls') & (L='Ground')`, `(SEL=TRUE)`, `(ALL)`. |
| `limit` | integer | 200 | Objects described (1–5000); `total` is always the full count. |
| `include_selection` | boolean | false | Add `selected` to each object. |

Each object: `id` (UUID), `type`, `name`, `class`, `layer`, `bbox` `[x1, y1, x2, y2]`, plus `symbol` for symbol instances and `plugin` for plug-in objects.

## `screenshot`

| Parameter | Type | Default | |
|---|---|---|---|
| `max_width` | integer | 1600 | Maximum width in pixels (256–4096). |

A PNG of the Vectorworks main window, captured even when it's behind other windows. Fails while it's minimised.

## `library_list`

No parameters. Returns the user Libraries, Favorites and Defaults folders, the application Libraries folder, and the `.vwx` files in the user Libraries and Favorites folders.

## `library_browse`

| Parameter | Type | Default | |
|---|---|---|---|
| `file` | string | required | Full path to a saved `.vwx`. |
| `type` | string | `symbol` | See [resource types](scripting.md#resource-types). |
| `filter` | string | none | Case-insensitive substring. |
| `limit` | integer | 500 | Names returned (1–10,000); `total` is always the full count. |

## `library_import`

| Parameter | Type | Default | |
|---|---|---|---|
| `file` | string | required | Full path to a saved `.vwx`. |
| `names` | string[] | required | Exact resource names. |
| `type` | string | `symbol` | See [resource types](scripting.md#resource-types). |
| `on_conflict` | string | `skip` | `skip`, `replace`, or `rename` (keep both; import gets `-2`, `-3`…). |

Per-name `status`: `imported`, `kept_existing`, `replaced_or_identical`, `not_found` or `failed`. With `rename` there's also `imported_as`.

## `library_install`

| Parameter | Type | Default | |
|---|---|---|---|
| `source` | string | required | Local path or `https://` URL of a `.vwx`. |
| `destination` | string | `libraries` | `libraries` or `favorites`. |
| `subfolder` | string | none | Relative path inside the destination, e.g. `Stage/Trusses`. |
| `filename` | string | from source | Must end in `.vwx`. |
| `overwrite` | boolean | false | Replace an existing file. |

Refuses `http://`, downloads over 4 GB, web pages served instead of files, and subfolders containing `..` or absolute paths.
