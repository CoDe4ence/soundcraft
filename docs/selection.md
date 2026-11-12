# Selection

> **Last reviewed:** 2026-11-12 · **Last updated:** 2026-11-12 · **Change:** major (gaps 1, 2, 4–7 fixed; previously major: created from a code read) · **Target:** Avid Pro Tools Ultimate 2026.4.1

What SoundCraft's edit and timeline selections do today, read from the code, and what they lack
compared with Pro Tools. The work is ranked as [G-24 in gaps.md](gaps.md#g-24-selection).

**Summary:** every Edit ▸ Selection menu item exists and works as named. The timeline selection
is made by dragging in a timebase ruler, drawn, played by Space, and kept with the edit selection
while linked. Start, End and Length are typed in the counter; Shift+Home/End extend; double-click
selects a gap and triple-click a whole track. What remains is Link Track and Edit Selection, which
still has no effect (see the end).

## What the selection holds

`EditState` in `crates/model/src/session.rs`:

| Field | Meaning |
|---|---|
| `selection` | The edit selection: a range, or a point (the insertion point) |
| `selected_tracks` | The tracks in the edit selection; also what the track headers show as selected |
| `selected_clips` | Clips selected as objects (Grabber) |
| `selected_sources` | Files selected in the Clip List |
| `timeline_selection` | The timeline selection, kept separate when unlinked |
| `link_timeline_edit`, `link_track_edit` | Options ▸ Link Timeline and Edit Selection / Link Track and Edit Selection |

The engine also keeps the previous selection for Restore Last Selection (`Engine::last_selection`),
recorded after every command that changes the selection.

## What works

- **Commands** (`crates/engine/src/cmd/edit.rs`, `edit_more.rs`, `tab.rs`, `timeline_selection.rs`, `selection_extend.rs`): `edit.select` (tracks in track order, range, clips, Clip List files; active edit groups add their members), Select All, Deselect All, Restore Last Selection, Change Timeline to Match Edit and back, Move Edit Left/Right (by the selection's length; an insertion point stays), Halve/Double Edit (within the session's limit), Duplicate and Extend Edit, Extend Edit Up/Down (and to group members), Remove Edit from Top/Bottom (the Edit window's top and bottom track), the marker-lane commands, Play Edit, Play Timeline, Tab and Shift+Tab to clip boundaries or transients, `edit.select_timeline`, and `edit.extend_to_start` / `edit.extend_to_end` (Shift+Home, Shift+End).
- **The timeline selection:** dragging in a timebase ruler makes it (a click still locates). With Link Timeline and Edit Selection on it is the edit selection, across every shown track, and every command that moves the edit selection keeps them together. Off, the timebase rulers shade it and the main ruler's markers show it. Space plays it; Play Edit plays the edit selection.
- **Mouse** (`crates/ui-egui/src/edit_window/input.rs`): click places the insertion point, drag selects a range across the tracks dragged over, Shift-click extends the range or adds the clicked track (selected tracks needn't be adjacent, as in Pro Tools), double-click selects a clip or the gap between clips, triple-click selects the whole track, the Grabber selects clips.
- **The counter:** click Start, End or Length to type it in the main counter's format (`soundcraft_time::parse_length` reads a length the way the counter writes it). Enter applies, Escape cancels, a bad time changes nothing.
- **Grid** mode snaps the selection.

## What remains

- **Link Track and Edit Selection has no effect.** One track list serves as both the selected tracks (the highlighted names, what track commands act on) and the edit selection's tracks, so they are always linked. Making the option work means a separate track selection that the headers draw and every track command reads, which changes what existing sessions do (they store the option off). See G-24.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-12 | major | Fixed: timeline selection made, drawn and played; linked selections kept together; typed Start/End/Length; Shift+Home/End; gap and whole-track clicks; Remove Edit, Move Edit and Double Edit bugs. Link Track and Edit Selection remains |
| 2026-11-12 | major | Created from a code read: what selection holds and does, and seven ranked gaps, four bugs confirmed |
