//! Memory locations (markers and stored selections) and mix/edit groups.

use crate::{GroupId, MarkerId, TrackId};
use soundcraft_time::Samples;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum MarkerKind {
    /// A point marker on a ruler.
    #[default]
    Marker,
    /// A stored selection range.
    Selection,
    /// A location that only recalls view settings.
    None,
}

/// A memory location (Window › Memory Locations).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemoryLocation {
    pub id: MarkerId,
    /// User-visible number (1-based, unique).
    pub number: u32,
    pub name: String,
    pub kind: MarkerKind,
    pub start: Samples,
    pub end: Samples,
    #[serde(default)]
    pub comments: String,
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    /// Marker ruler lane 1..5.
    #[serde(default = "one")]
    pub ruler: u8,
    /// Track-lane marker on this track, if any.
    #[serde(default)]
    pub track: Option<TrackId>,
    /// The view settings this location recalls along with its place (General Properties).
    #[serde(default, skip_serializing_if = "LocationView::is_empty")]
    pub view: LocationView,
}

/// The view settings a memory location can store. Each is recalled only when stored, and only
/// for tracks and groups that still exist.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct LocationView {
    /// Zoom: samples per point and the left edge of the view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<StoredZoom>,
    /// Pre-roll and post-roll lengths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_post_roll: Option<(Samples, Samples)>,
    /// The tracks hidden when it was stored; recalling shows every other track (so tracks made
    /// later stay shown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_tracks: Option<Vec<TrackId>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_heights: Option<Vec<(TrackId, crate::TrackHeight)>>,
    /// Which groups were active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_enables: Option<Vec<(GroupId, bool)>>,
    /// A window configuration by number (1-based); the UI recalls it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_config: Option<u32>,
}

/// A stored zoom (see [`LocationView::zoom`]).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredZoom {
    pub samples_per_px: f64,
    pub scroll: Samples,
}

impl LocationView {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The names of the settings stored, as `markers.add`'s `store` takes them.
    pub fn stored(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for (name, on) in [
            ("zoom", self.zoom.is_some()),
            ("pre_post_roll", self.pre_post_roll.is_some()),
            ("track_visibility", self.hidden_tracks.is_some()),
            ("track_heights", self.track_heights.is_some()),
            ("group_enables", self.group_enables.is_some()),
            ("window_config", self.window_config.is_some()),
        ] {
            if on {
                out.push(name);
            }
        }
        out
    }
}

fn one() -> u8 {
    1
}

/// A track group (Track › Group…).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    /// Group letter id shown in the Groups list ("a", "b", …).
    pub letter: char,
    pub members: Vec<TrackId>,
    pub edit: bool,
    pub mix: bool,
    pub active: bool,
    pub color: [u8; 3],
    /// Shared attributes linked by the group (volume, mute, solo, rec, etc.).
    #[serde(default)]
    pub attributes: Vec<String>,
}
