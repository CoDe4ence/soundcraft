# Memory locations

> **Last reviewed:** 2026-11-12 · **Last updated:** 2026-11-12 · **Change:** major (created from a code read) · **Target:** Avid Pro Tools Ultimate 2026.4.1

What SoundCraft's memory locations (markers and stored selections) do today, read from the code,
and what they lack compared with Pro Tools. The work to close the gaps is ranked as
[G-23 in gaps.md](gaps.md#g-23-memory-locations-depth).

**Summary:** markers and selections work for everyday navigation: you can add, name, recall, step
through, delete and copy them, put them on five ruler lanes, and they are saved with the session
and exported to MIDI files. The Memory Locations window is a basic list. A Pro Tools memory location
also stores view settings (zoom, pre/post-roll, shown tracks, track heights, group enables, window
configuration), and SoundCraft stores none of them. Several marker options and display settings
can be switched on but have no effect.

## What a memory location holds

`MemoryLocation` in `crates/model/src/markers.rs`:

| Field | Meaning |
|---|---|
| `number` | User-visible number, unique; a new one takes the highest number plus one (`Session::add_marker`) |
| `name`, `comments` | Free text |
| `kind` | `Marker` (a point), `Selection` (a stored range) or `None` (documented as "only recalls view settings") |
| `start`, `end` | Samples; a marker has `end == start` |
| `color` | Optional RGB; the ruler draws it, otherwise yellow |
| `ruler` | Marker ruler lane 1–5 |
| `track` | Optional track for a track-lane marker; nothing sets it yet |

Memory locations are part of the session document, so they are saved, reopened and undone like any
other edit.

## What works

**Commands** (`crates/engine/src/cmd/event.rs`; every one is also available to the CLI, the control
channel and MCP):

| Command | What it does |
|---|---|
| `markers.add` (Enter) | New marker at the playhead while playing, else at the selection start. `kind: "selection"` stores the edit selection. Takes `name`, `at` or `start`/`end`, and `ruler` 1–5 |
| `markers.edit` | Rename, comments, move (`at`, keeps a selection's length), colour (`[r,g,b]` or `null`); select it by `number`, `id` or `name` |
| `markers.delete` | Delete one, by `number`, `id` or `name` |
| `markers.recall` | A marker puts the insertion point there; a selection restores the edit selection; both locate the transport |
| `markers.next`, `markers.previous` | Step to the next or previous marker start from the selection |

**Editing and options:**
- Copy takes the point markers inside the copied range; Edit ▸ Paste Special ▸ Merge Markers pastes them at the insertion point, skipping exact duplicates (`edit.paste_merge_markers`).
- Event ▸ Time Operations ▸ Insert Time moves later markers; Cut Time removes the ones inside the range and moves later ones back (`crates/engine/src/cmd/event.rs`).
- Edit ▸ Selection ▸ Move/Extend Edit Selection To Marker Lane Above/Below choose ruler lanes 1–5 and report the markers in the selection (`edit.marker_lane_*`).
- Grid lines and snapping can include marker boundaries (`grid_clips_markers`).
- Event ▸ Extract Chords from Selection writes each bar's chord as a marker on Markers 2 (`event.extract_chords`).

**In the Edit window** (`crates/ui-egui/src/edit_window/rulers.rs`):
- Five marker rulers (View ▸ Rulers ▸ Markers … Markers 5, and All Marker Rulers). Markers draw as flags in their colour; a selection also shades its range.
- Click a marker to recall it, Alt-click to delete it, double-click it to rename it, and double-click empty ruler space to add a marker on that lane.
- View ▸ Marker Displays ▸ Ruler Lines draws a line down the tracks at each marker (`crates/ui-egui/src/edit_window/overlay.rs`).

**The Memory Locations window** (Window ▸ Memory Locations, Cmd+5; `crates/ui-egui/src/panels/memory.rs`), a dockable panel:
- "+ Marker" and "+ Selection" buttons.
- One row per location: number (click to recall), editable name, location and length in the main counter's format, type, and a delete button.

**Files:**
- Standard MIDI File export writes each marker as a Marker meta event (`crates/engine/src/io/midi.rs`).
- File ▸ Export ▸ Session Info as Text ends with a markers listing: number, location, name (`crates/engine/src/inspect.rs`).

## What is missing or does nothing

Ranked by how much a Pro Tools user would miss it.

1. **No stored view settings.** A Pro Tools memory location can also store zoom, pre/post-roll, shown/hidden tracks, track heights, group enables and the window configuration, and recall them. SoundCraft stores none of these. A `None` location is meant to recall only view settings, so recalling one just moves the playhead.
2. **The window is a list, not an editor.** It has no comments, colour, ruler lane or kind controls, and no dialog for a new location's name and settings. It can't sort, filter or search, it has no "delete all", and it has no per-location view-setting columns. Comments and colour can be set only through `markers.edit`.
3. **Markers Follow Edit does nothing.** `options.markers_follow_edit` is saved and shown checked, but no edit reads it: shuffle edits, deleting clips and other edits leave markers where they were. Only Insert Time and Cut Time move markers, and they always do.
4. **No track-lane markers.** `MemoryLocation::track` exists, but no command or UI sets it. View ▸ Marker Displays ▸ Track Lane, Track Lane Color, Folder Lines and Ruler Lane Color are saved toggles that nothing draws. Options ▸ Marker Target Follows First Selected Track is the same.
5. **Markers can't be dragged** on the ruler. They move only through `markers.edit`.
6. **No keypad recall.** The `markers.recall` spec lists "Period + n + Period", but no key handler enters that sequence. `crates/ui-egui/src/shortcuts.rs` notes that egui reports keypad digits as main-keyboard digits, so the numeric keypad can't be told apart.
7. **Imports drop markers.** The MIDI reader parses Marker meta events (`crates/midi/src/smf.rs`), but MIDI import does not add them to the session. File ▸ Import ▸ Session Data imports tracks only, not memory locations.
8. **Small inconsistencies:**
   - A new marker's default name, "Marker n", counts the existing markers plus one. Its number is the highest number plus one. After a delete the two differ.
   - Copy and Merge Markers carry only point markers, not stored selections.
   - The session text listing leaves out each location's kind, length and comments.

**Rough estimate to close the gaps:** about 20–35 agent-hours. Item 1 and the window editor are most
of it.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-12 | major | Created from a code read: what memory locations hold and do, and eight ranked gaps |
