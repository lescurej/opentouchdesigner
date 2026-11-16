mod cook;
mod speakers;
mod transport;

use super::{spec_animated, ChopSpec};

pub(crate) use cook::transport_for_input;
pub(crate) use transport::{AudioMovieState, MovieTransport};

pub const TYPE_NAME: &str = "audiomovieCHOP";

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Audio Movie",
        &[],
        "Plays a movie file's soundtrack, locked to a Movie File In TOP's playhead.",
        cook::params,
        cook::cook,
    )
}
