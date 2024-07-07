use gst::prelude::*;
use gst::MessageView;
use gstreamer as gst;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Send,
    Recv,
}

fn main() {
    let cli = Cli::parse();
    gst::init().unwrap();

    let pipeline = match cli.command {
        Commands::Send => {
            let source = gst::ElementFactory::make("videotestsrc").build().unwrap();
            let enc = gst::ElementFactory::make("x264enc").build().unwrap();
            let pay = gst::ElementFactory::make("rtph264pay").build().unwrap();
            let sink = gst::ElementFactory::make("udpsink")
                .property("host", "172.18.208.40")
                .property("port", 9001)
                .build()
                .unwrap();

            let pipeline = gst::Pipeline::with_name("send-pipeline");
            pipeline.add_many(&[&source, &enc, &pay, &sink]).unwrap();

            gst::Element::link_many(&[&source, &enc, &pay, &sink]).unwrap();

            pipeline
        }
        Commands::Recv => {
            let source = gst::ElementFactory::make("udpsrc")
                .property("port", 9001)
                .property(
                    "caps",
                    &gst::Caps::builder("test")
                        .field(
                            "application/x-rtp",
                            &gst::Structure::builder("application/x-rtp")
                                .field("media", &"video")
                                .field("clock-rate", &90000)
                                .field("encoding-name", &"H264")
                                .field("payload", &96)
                                .build(),
                        )
                        .build(),
                )
                .build()
                .unwrap();
            let depay = gst::ElementFactory::make("rtph264depay").build().unwrap();
            let dec = gst::ElementFactory::make("decodebin").build().unwrap();
            let sink = gst::ElementFactory::make("autovideosink").build().unwrap();

            let pipeline = gst::Pipeline::with_name("recv-pipeline");
            pipeline.add_many(&[&source, &depay, &dec, &sink]).unwrap();

            gst::Element::link_many(&[&source, &depay, &dec, &sink]).unwrap();

            pipeline
        }
    };

    pipeline.set_state(gst::State::Playing).unwrap();

    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
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
