# Vectorworks 2026+ API pitfalls

Calls that fail silently, hang, crash or have changed on Vectorworks 2026 and later. Claude sees this list in the `run_python` description; it's here for people writing scripts.

Most of these were found by [vwx-mcp](https://github.com/vicquick/vwx-mcp) running every command against a live Vectorworks 2026.

## Silent failures

| Symptom | Cause | Instead |
|---|---|---|
| Circle not drawn, `object_id` null | `vs.ArcByCenter` is broken | `vw.circle`, or `vs.Oval(left, top, right, bottom)` |
| Arc has the wrong extent | `vs.Arc`'s last argument is the sweep, not the end angle | `vs.Arc(l, t, r, b, start, sweep)` or `vw.arc` |
| A name "exists" that doesn't | `vs.GetObject` returns a type-0 dummy for unknown names | `vw.named`, or check `vs.GetTypeN(h) != 0` |
| `SetFillFore` / `SetPenFore` do nothing | Three numbers instead of one tuple | pass `(r, g, b)` |
| `LNewObj()` returns the wrong object | Some operations replace the object (e.g. send-to-surface) | name it with `vs.SetName` and look it up |
| Text position metres off | `GetBBox` of rotated text is a screen box | `vs.HCenter(h)`, `vs.GetTextOrientation(h)` |
| Drop shadow 1000× too big after copying | `SetDropShadowData` stores offset and blur ×1000 | divide by 1000, then read back |
| Criteria match nothing | Quoting | single-quote names: `"((R IN ['Part Info']))"` |
| An empty group "contains" the whole layer | `FInGroup` of an empty group points into the parent list | compare each child's parent UUID with the container's |

## Hangs and crashes

| Don't | Because | Instead |
|---|---|---|
| `while h: h = vs.NextObj(h)` | can loop forever | `vw.find` / `vs.ForEachObject` |
| Create, delete or restack inside a `ForEachObject` callback | invalidates the iteration | collect first (`vw.find` does), then change |
| `vs.SetPrefReal(85)` + `vs.SetPref(68)` before `vs.Wall` | crashes Vectorworks | `SetWallWidth` → `Wall` → `SetWallThickness` / `SetWallHeights` |
| Pass `lambda *a: …` as a callback | Vectorworks counts named parameters and raises a "Script Errors" dialog | declare exactly the parameters expected: `def f(h)` for `ForEachObject`; `def f(name): return 0` (skip) or `1` (replace) for `ImportResToCurFileN` |
| Call export/import commands (`export_pdf`, `ImportDWG`…) unattended | they open modal dialogs | do these by hand |

## Renamed or changed

| Old | Now |
|---|---|
| `CreateExtrude(h, ht)` | `HExtrude(h, zBottom, zTop)` |
| `LinDimN(…)` | `LinearDim(start, end, offset, dimType, arrow, textFlag, textOffset)` |
| `AngularDim(center, p1, p2, …)` | `AngularDim(startPt, endPt, arcCenter, textOffset, arrow, textFlag, posAngle)` |
| `SetClassVisibility(name, n)` | `ShowClass(name)` / `HideClass(name)` |
| `IFC_SetPSetAttribute(…)` | `IFC_SetPsetProp(h, pset, prop, value)`, value as a string |
| `ZoomToSel()` | `DoMenuTextByName('Fit To Objects', 0)` |
| `SetWSCellValue(ws, r, c, v)` | `SetWSCellFormula(ws, r, c, r, c, v)` |
| `SaveDocument()` | `DoMenuTextByName('Save', 0)` |
| `SymbolCreate(…)` | `BeginSym(name)` … `EndSym()` |
| `HMirror(h, p1, p2)` | `MirrorN(h, dup, p1, p2, preserveMatrix)` |
| `SetLName(h, name)` | `SetName(h, name)` |
| `GetVWVersion()` | `GetVersion()` → `(major, minor, maint, build)` |
| `GetNumberOfComponents(h)` → int | → `(ok, count)` |
| `GetClassName` / `GetClName` | `ClassNum()` + `ClassList(i)` (1-based) |
| `InternalIndex` APIs | UUIDs: `GetObjectUuid`, `GetObjectByUuid` |
| pre-2019 marker/pen/component calls | the `…N` variants (`SetLSN`, `InsertNewComponentN`) |

Note: Vectorworks 2027 reports itself as major version 32.
