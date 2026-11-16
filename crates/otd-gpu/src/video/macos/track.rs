use cidre::{arc, av};

pub(crate) fn load_video_track(asset: &av::UrlAsset) -> Option<arc::R<av::asset::Track>> {
    load_track(asset, av::MediaType::video())
}

pub(crate) fn load_track(
    asset: &av::UrlAsset,
    media: &av::MediaType,
) -> Option<arc::R<av::asset::Track>> {
    let (tx, rx) = std::sync::mpsc::channel();
    asset.load_tracks_with_media_type_block(media, move |tracks, err| {
        let track = if err.is_none() {
            tracks.and_then(|t| t.get(0).ok().map(|tr| tr.retained()))
        } else {
            None
        };
        let _ = tx.send(track);
    });
    rx.recv().ok().flatten()
}