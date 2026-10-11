# Install and use a resource library

Ask Claude in plain language. These are the tools it uses, if you want to steer it.

## Install a library file

> Install the library at https://example.com/downloads/Truss-Library.vwx into a "Rigging" folder.

Claude calls `library_install`. The file goes into your user **Libraries** folder (or **Favorites**, if you ask), so it appears in the Resource Manager. Refresh the Resource Manager if it's already open.

- Local files work too: give the full path.
- Only `https://` downloads are accepted, and an existing file is never replaced unless you say so.
- Only install libraries from sources you trust: `.vwx` files can contain scripts.

## See what's in a library

> What truss symbols are in Truss-Library.vwx?

Claude calls `library_browse`, which reads the file without opening it. Ask for textures, hatches, materials, wall styles and so on by name.

## Bring resources into the drawing

> Import "Truss 3m" and "Truss 2m" from that library into this drawing.

Claude calls `library_import`. If the drawing already has a resource with the same name, Claude skips it unless you say otherwise:

- "replace the existing ones": the library version overwrites yours.
- "keep both": the import is renamed `Truss 3m-2`.

Identical resources are never duplicated: Vectorworks recognises an exact match and keeps the one you have.

## See all your libraries

> Which resource libraries do I have?

Claude calls `library_list`, which shows your Libraries and Favorites folders and the `.vwx` files in them.
