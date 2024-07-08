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
            let source = gst::ElementFactory::make("ximagesrc")
            .property("use-damage", false)
            .build()
            .unwrap();
            let capsfilter = gst::ElementFactory::make("capsfilter")
                .property("caps", gst::Caps::builder("video/x-raw")
                    .field("framerate", &gst::Fraction::new(30, 1))
                    .build()
                )
                .build()
                .unwrap();
            let videoconvert = gst::ElementFactory::make("videoconvert").build().unwrap();
            let enc = gst::ElementFactory::make("x264enc")
                .property_from_str("tune", "zerolatency")
                .build()
                .unwrap();
            let pay = gst::ElementFactory::make("rtph264pay").build().unwrap();
            let sink = gst::ElementFactory::make("multiudpsink").build().unwrap();
            sink.emit_by_name_with_values("add", &["192.168.171.69".into(), 9001.into()]);
            sink.emit_by_name_with_values("add", &["192.168.161.163".into(), 9001.into()]);

            let tee = gst::ElementFactory::make("tee").build().unwrap();

            let queue1 = gst::ElementFactory::make("queue").build().unwrap();
            let queue2 = gst::ElementFactory::make("queue").build().unwrap();

            let videoconvert2 = gst::ElementFactory::make("videoconvert").build().unwrap();
            let videosink = gst::ElementFactory::make("autovideosink").build().unwrap();

            let pipeline = gst::Pipeline::with_name("send-pipeline");
            pipeline.add_many(&[&source, &capsfilter, &tee, &queue1, &queue2, &videoconvert, &enc, &pay, &sink, &videoconvert2, &videosink]).unwrap();

            gst::Element::link_many(&[&source, &capsfilter, &tee, &queue1, &videoconvert, &enc, &pay, &sink]).unwrap();
            gst::Element::link_many(&[&tee, &queue2, &videoconvert2, &videosink]).unwrap();

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
