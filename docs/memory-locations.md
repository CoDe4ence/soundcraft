# Memory locations

> **Last reviewed:** 2026-11-12 · **Last updated:** 2026-11-12 · **Change:** major (the eight gaps closed; previously major: created from a code read) · **Target:** Avid Pro Tools Ultimate 2026.4.1

What SoundCraft's memory locations (markers, stored selections and view-only locations) do, read
from the code. The eight gaps the first version of this page listed are closed; what remains is
listed at the end, and in [gaps.md](gaps.md#g-23-memory-locations-depth).

## What a memory location holds

`MemoryLocation` in `crates/model/src/markers.rs`:

| Field | Meaning |
|---|---|
| `number` | User-visible number, unique; a new one takes the highest number plus one, and its default name ("Marker n") uses the same number |
| `name`, `comments` | Free text |
| `kind` | `Marker` (a point), `Selection` (a stored range) or `None` (view settings only, no place) |
| `start`, `end` | Samples; a marker has `end == start` |
| `color` | Optional RGB; drawn on rulers and lanes, otherwise yellow |
| `ruler` | Marker ruler lane 1–5 |
| `track` | A track-lane marker's track; `None` for a ruler marker |
| `view` | The view settings it recalls (`LocationView`): zoom and scroll, pre/post-roll, hidden tracks, track heights, group enables, a window configuration number. Each is optional |

Memory locations are part of the session document: saved, reopened and undone like any other edit.

## Commands

All in the command registry, so menus, shortcuts, the CLI, the control channel and MCP share them.

| Command | What it does |
|---|---|
| `markers.add` (Enter) | New location at the playhead while playing, else at the selection start; `kind: "selection"` stores the edit selection. Takes `name`, `at` or `start`/`end`, `ruler` 1–5 or `track`, `comments`, `color`, and `store: [...]` (the view settings to capture now) with `window_config: n` |
| `markers.edit` | Rename, comments, move (`at`), colour, `ruler`, `track` (null: back to the ruler), `store` / `forget` view settings, `window_config` |
| `markers.delete` | Delete one, by `number`, `id` or `name` |
| `markers.delete_all` | Delete all of them, or one ruler's (`ruler`) or one track's (`track`) |
| `markers.recall` (Period, number, Period) | A marker or selection goes to its place; the stored view settings are recalled too, only for tracks and groups that still exist, with stored numbers clamped. A `none` location recalls only its view settings. Returns the window configuration for the UI to recall |
| `markers.next`, `markers.previous` | Step to the next or previous marker |

The view settings `store` and `forget` name: `zoom`, `pre_post_roll`, `track_visibility`,
`track_heights`, `group_enables`, `window_config`. Track visibility stores the hidden tracks, so
tracks made later stay shown.

## Editing

- **Markers Follow Edit** (Options): a track's lane markers follow that track's Shuffle edits, stopping at a locked clip as the clips do. Ruler markers follow Shuffle edits made on every track that holds clips: cut, clear, paste, duplicate, repeat, insert silence, duplicate-extend, move to playlist and placing a file. Markers in time that closes up are removed. With the option off, Shuffle edits leave markers alone.
- **Insert Time and Cut Time** always move markers; Cut Time removes the ones in the cut.
- **Copy** carries markers and stored selections that start in the range; **Paste Special ▸ Merge Markers** pastes them at the insertion point, skipping exact duplicates.
- **Move/Extend Edit Selection To Marker Lane Above/Below** choose ruler lanes 1–5.
- **Grid** lines and snapping can include marker boundaries.
- **Event ▸ Extract Chords from Selection** writes each bar's chord as a marker on Markers 2.
- **Deleting a track** removes its lane markers; undo brings them back.

## In the Edit window

- **Five marker rulers** (View ▸ Rulers ▸ Markers … Markers 5). Ruler markers draw as flags in their colour; a selection also shades its range. Click recalls, Alt-click deletes, double-click renames (or adds one on empty ruler), and **dragging** moves a marker, snapped in Grid mode, in one undo step.
- **Track lanes** (View ▸ Marker Displays ▸ Track Lane): a strip along the top of each track shows its lane markers, with the same click, double-click and drag actions. With **Options ▸ Marker Target Follows First Selected Track**, a new marker goes onto the first selected track instead of the ruler.
- **Ruler Lines** draws a line down the tracks at each ruler marker, and down its own track for a lane marker. **Folder Lines** adds a folder's tracks' lane markers to the folder's lane and lines. **Ruler Lane Color** and **Track Lane Color** shade each lane from one marker to the next in its colour.

## The Memory Locations window

Window ▸ Memory Locations (Cmd+5), a dockable panel (`crates/ui-egui/src/panels/memory.rs`):

- **New…** opens a dialog: name, type, ruler or track lane, colour, comments, and which view settings to store (with a window configuration number). **Edit** opens it for an existing location: settings left ticked keep what they stored, new ticks capture now, unticked ones are forgotten. Each OK is one undo step.
- **+ Marker** and **+ Selection** add one at once. **Find** filters by name or comments. **Delete ▸ Delete All Memory Locations**.
- Rows: number (click to recall), Edit, delete, colour menu, editable name, location and length in the main counter's format, type, where it lives (ruler n or a track), the stored settings (Z P V H G W, named on hover) and editable comments.

## Keyboard recall

Period, the location's number (up to five digits, shown in the status line), Period. Escape
cancels; any other key cancels and then does its usual job; an unknown number says so. In
Commands Keyboard Focus, Period nudges: egui reports the numeric keypad's Period as the main
keyboard's, so there the entry starts only when nothing is selected to nudge.

## Files

- **MIDI import** turns the file's Marker meta events into markers, placed like the notes (`file.import_midi`, `markers?: true`; at most 10,000; the same file twice adds them once). **MIDI export** writes each marker as a Marker meta event.
- **Import Session Data** (`file.import_session_data`, `markers?: false`) brings the donor's memory locations with new numbers, skipping ones already here. Lane markers follow their tracks when imported; stored track settings are mapped to the imported tracks and group enables are dropped.
- **Session Info as Text** lists each location's number, location, type, length, name and comments.

## What remains

- **Keyboard recall in Commands Keyboard Focus with a selection.** The numeric keypad can't be told apart from the main keyboard (an egui limit), so Period nudges there. Recall from the window, the rulers or the lanes instead.
- **The window is one fixed set of columns.** Pro Tools lets you choose which columns show and sort by them; the panel scrolls sideways when it is narrow.
- **A location's place can be typed only through `markers.edit` (`at`)**; the window and dialog show it but move it only by dragging.
- **Fidelity is unmeasured**: these behaviours follow the features' names and observed behaviour, not a side-by-side comparison with Pro Tools.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-12 | major | The eight gaps closed: stored view settings, editor window, Markers Follow Edit, track-lane markers and display options, dragging, keyboard recall, imports, naming/copy/listing. What remains listed |
| 2026-11-12 | major | Created from a code read: what memory locations hold and do, and eight ranked gaps |
