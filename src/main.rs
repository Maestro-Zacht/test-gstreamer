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
            let sink = gst::ElementFactory::make("multiudpsink").build().unwrap();
            sink.emit_by_name_with_values("add", &["172.20.208.40".into(), 9001.into()]);
            sink.emit_by_name_with_values("add", &["172.20.215.16".into(), 9001.into()]);

            let pipeline = gst::Pipeline::with_name("send-pipeline");
            pipeline.add_many(&[&source, &enc, &pay, &sink]).unwrap();

            gst::Element::link_many(&[&source, &enc, &pay, &sink]).unwrap();

            pipeline
        }
        Commands::Recv => {
            gst::parse::launch(
                "udpsrc port=9001 caps=\"application/x-rtp, media=(string)video, clock-rate=(int)90000, encoding-name=(string)H264, payload=(int)96\" ! rtph264depay ! decodebin ! videoconvert ! autovideosink"
            )
            .unwrap()
            .dynamic_cast::<gst::Pipeline>()
            .unwrap()
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
    println!("End of stream");
}
