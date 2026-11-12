# Selection

> **Last reviewed:** 2026-11-12 · **Last updated:** 2026-11-12 · **Change:** major (created from a code read) · **Target:** Avid Pro Tools Ultimate 2026.4.1

What SoundCraft's edit and timeline selections do today, read from the code, and what they lack
compared with Pro Tools. The work is ranked as [G-24 in gaps.md](gaps.md#g-24-selection).

**Summary:** every Edit ▸ Selection menu item exists, and selecting with the mouse, Tab to
transient and Restore Last Selection work for everyday editing. But the timeline selection is
invisible and can't be made on its own, so Link Timeline and Edit Selection has almost no effect;
Link Track and Edit Selection has none. Several selection commands have small bugs, the counter's
Start/End/Length can't be typed, and a few mouse and keyboard ways of selecting are missing.

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

- **Commands** (`crates/engine/src/cmd/edit.rs`, `edit_more.rs`, `tab.rs`): `edit.select` (tracks, range, clips, Clip List files; active edit groups add their members), Select All, Deselect All, Restore Last Selection, Change Timeline to Match Edit and back, Move Edit Left/Right, Halve/Double Edit, Duplicate and Extend Edit, Extend Edit Up/Down (and to group members), Remove Edit from Top/Bottom, the marker-lane selection commands, Play Edit, Play Timeline, Tab and Shift+Tab to clip boundaries or transients.
- **Mouse** (`crates/ui-egui/src/edit_window/input.rs`): click places the insertion point, drag selects a range across the tracks dragged over, Shift-click extends, double-click on a clip selects it, the Grabber selects clips. The Edit window shades the selection on its tracks and marks it on the main ruler.
- **Grid** mode snaps the selection.

## What is missing or wrong

Ranked by how much a Pro Tools user would miss it.

1. **The timeline selection is invisible and can't be made on its own.** Nothing draws it, dragging in a timebase ruler only moves the playhead, and playback (Space) ignores it; only Play Timeline and TCE to Timeline read it. So Options ▸ Link Timeline and Edit Selection, when off, changes almost nothing.
2. **With the link on, the two selections drift apart.** Only `edit.select` and Tab copy the edit selection to the timeline selection; Move Edit, Halve/Double, Select All, Restore Last Selection and recalling a memory location don't, so Play Timeline then plays the old range (checked: after Move Edit Right the edit selection was 2000–3000 and the timeline selection still 1000–2000).
3. **Link Track and Edit Selection does nothing.** The option is saved and shown, but one track list serves as both the selected tracks and the edit selection's tracks, so they are always linked.
4. **The counter's Start, End and Length can't be typed.** They are drawn as text only.
5. **Mouse gaps:** Shift-click on another track adds only that track, not the tracks between; double-click in empty space doesn't select the gap between clips; triple-click doesn't select the whole track; dragging in a timebase ruler doesn't select.
6. **Keyboard gaps:** Shift+Home and Shift+End don't extend the selection to the session start or end; they do nothing.
7. **Bugs in selection commands:**
   - Remove Edit from Top/Bottom take the first/last *selected* track, which is the order the tracks were selected in, not the top/bottom one (`edit.select` with `exact` keeps the given order). Checked: with Snare then Kick selected, Remove Edit from Top removed Snare, not Kick.
   - Move Edit Left/Right on an insertion point moves it by one sample.
   - Halve/Double Edit don't keep the selection within the session's limit (four Doubles from 2³⁹ samples reached 8× the limit).

**Rough estimate:** about 15–25 agent-hours; item 3 (a separate track selection that every
track command reads) is the largest.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-11-12 | major | Created from a code read: what selection holds and does, and seven ranked gaps, four bugs confirmed |
