# Your first session

In this tutorial you'll install vwmcp, connect it to Vectorworks, and then ask Claude to draw something, describe the drawing and check its own work. It takes about ten minutes.

You'll need:

- Windows, with Vectorworks 2026 or later, opened at least once.
- Claude Desktop (or Claude Code).
- `vwmcp.exe` from the [latest release](https://github.com/MattA-Official/vwmcp/releases/latest).

Use a new, empty Vectorworks document for this tutorial, not one you care about.

## 1. Install vwmcp

Close Claude Desktop, then double-click `vwmcp.exe`.

A window opens and lists what it did. Look for these lines:

```
vwmcp 0.3.0 installed to C:\Users\you\AppData\Local\vwmcp
  Claude Desktop: registered "vectorworks" (restart Claude Desktop to load it).

Vectorworks:
  2027: ready to install (one click, below)
```

Press Enter to close the window.

## 2. Connect Vectorworks

Open Vectorworks and choose **Help › Install Partner Products…**. Find **vwmcp** in the list and click **Install**.

Vectorworks restarts. Afterwards, open **Tools › Utilities**: there's now a **vwmcp** command with the shortcut **Ctrl+Shift+B**. You never need to click it; vwmcp uses it behind the scenes.

Create a new blank document.

## 3. Check the connection

Open Claude Desktop and send:

> Check the Vectorworks connection.

Claude calls the `status` tool. You should see a successful round trip that names your blank document and the Vectorworks version. If it doesn't succeed, see [Fix common problems](../how-to/troubleshoot.md) before going on.

## 4. Draw something

Send:

> Draw a 4 m × 3 m rectangle with its bottom-left corner at the origin, and a circle of radius 0.5 m at its centre. Name the rectangle "Stage".

Watch Vectorworks: both shapes appear. You'll notice Claude looked up a function or two with `vs_lookup` before writing its script. That's how it gets the arguments right first time.

Now press **Ctrl+Z** in Vectorworks. Both shapes disappear together: each script Claude runs is a single undo step. Press **Ctrl+Y** to bring them back.

## 5. Ask about the drawing

Send:

> What's in this document?

Claude calls `document_overview` and `query_objects`, and reports the layers, the two objects, their classes and their bounding boxes. Every object has a UUID, which is how Claude refers to it in later steps.

## 6. Let Claude check its own work

Send:

> Take a screenshot and tell me whether the circle is centred.

Claude captures the Vectorworks window, even if it's behind other windows, and checks the picture against the numbers.

## 7. Work while Claude works

Click into another application, say a browser, and send:

> Add four 0.2 m square columns at the corners of Stage.

Keep typing in the browser. The columns appear in Vectorworks without it coming to the front or interrupting your typing.

## What you've learnt

You've installed vwmcp, connected Vectorworks once, and seen the loop Claude uses: look up the API, run a script, inspect, check visually. Each script is a single undo step, and everything works while Vectorworks is in the background.

Next:

- [Install and use a resource library](../how-to/resource-libraries.md)
- [How vwmcp talks to Vectorworks](../explanation/architecture.md)
- [MCP tools reference](../reference/tools.md)
