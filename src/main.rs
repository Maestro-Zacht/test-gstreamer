use gst::prelude::*;
use gstreamer as gst;

fn main() {
    gst::init().unwrap();

    let source = gst::ElementFactory::make("videotestsrc").build().unwrap();
    // let enc = gst::ElementFactory::make("x264enc").build().unwrap();
    let sink = gst::ElementFactory::make("autovideosink").build().unwrap();

    let pipeline = gst::Pipeline::with_name("test-pipeline");
    pipeline.add_many(&[&source, &sink]).unwrap();

    gst::Element::link_many(&[&source, &sink]).unwrap();

    pipeline.set_state(gst::State::Playing).unwrap();

    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(_) => break,
            MessageView::Error(err) => {
                eprintln!(
                    "Error from {:?}: {} ({:?})",
                    msg.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            _ => (),
        }
    }
}
