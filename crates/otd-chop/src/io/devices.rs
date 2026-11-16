//! I/O submodule: devices.

use super::*;

/// cpal 0.18 exposes the human-readable name through a description struct.
pub(crate) fn device_name(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_string())
}
