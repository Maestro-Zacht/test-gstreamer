use std::sync::Arc;
use std::thread;

use gst::prelude::*;
use gst::MessageView;
use gstreamer as gst;
use message_io::network::{NetEvent, Transport};
use message_io::node::{self};

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Send,
    Recv(ServerArgs),
}

#[derive(Args, Debug)]
struct ServerArgs {
    ip: String,
}

fn main() {
    let cli = Cli::parse();
    gst::init().unwrap();

    let pipeline = match cli.command {
        Commands::Send => {
            let pipeline_string = if cfg!(target_os = "windows") {
                "input-selector name=i ! tee name=t ! queue ! autovideosink t. ! queue ! videoconvert ! x264enc tune=zerolatency ! rtph264pay ! multiudpsink name=s d3d11screencapturesrc show-cursor=true ! video/x-raw,framerate=30/1 ! i.sink_0 videotestsrc pattern=white ! video/x-raw,framerate=30/1 ! i.sink_1"
            } else {
                "input-selector name=i ! tee name=t ! queue ! autovideosink t. ! queue ! videoconvert ! x264enc tune=zerolatency ! rtph264pay ! multiudpsink name=s ximagesrc use-damage=false ! video/x-raw,framerate=30/1 ! videoconvert ! i.sink_0 videotestsrc pattern=white ! i.sink_1"
            };

            let pipeline = gst::parse::launch(pipeline_string)
                .unwrap()
                .dynamic_cast::<gst::Pipeline>()
                .unwrap();

            let sink = pipeline.by_name("s").unwrap();
            let selector = pipeline.by_name("i").unwrap();

            thread::spawn(move || {
                let (handler, listener) = node::split::<()>();
                handler
                    .network()
                    .listen(Transport::Ws, "0.0.0.0:9000")
                    .unwrap();

                let sink = Arc::new(sink);

                let mut active_pad: u32 = 0;

                // let sink_clone = sink.clone();
                // thread::spawn(move || {
                //     thread::sleep(Duration::from_secs(20));
                //     handler.stop();
                //     sink_clone.emit_by_name_with_values("clear", &[]);
                //     println!("stopped server");
                // });

                listener.for_each(move |event| match event.network() {
                    NetEvent::Connected(_, _) => unreachable!(),
                    NetEvent::Accepted(endpoint, _) => {
                        let ip = endpoint.addr().ip().to_string();
                        sink.emit_by_name_with_values("add", &[(&ip).into(), 9001.into()]);
                        println!("{} connected", ip);
                        active_pad = (active_pad + 1) % 2;
                        selector.set_property(
                            "active-pad",
                            &selector
                                .static_pad(&format!("sink_{}", active_pad))
                                .unwrap(),
                        );
                    }
                    NetEvent::Message(_endpoint, _data) => println!("Message"),
                    NetEvent::Disconnected(endpoint) => {
                        let ip = endpoint.addr().ip().to_string();
                        sink.emit_by_name_with_values("remove", &[(&ip).into(), 9001.into()]);
                        println!("{} disconnected", ip);
                    }
                });

                println!("exit server listener");
            });

            pipeline
        }
        Commands::Recv(ServerArgs { ip }) => {
            thread::spawn(move || {
                let (handler, listener) = node::split::<()>();
                let (_server, _) = handler
                    .network()
                    .connect(Transport::Ws, format!("{}:9000", ip))
                    .unwrap();
                // thread::spawn(move || {
                //     thread::sleep(Duration::from_secs(1000));
                //     handler.stop();
                //     println!("stopped");
                // });
                listener.for_each(move |event| match event.network() {
                    NetEvent::Connected(_, _) => {
                        println!("Connected");
                    }
                    NetEvent::Accepted(_, _) => unreachable!(),
                    NetEvent::Message(_endpoint, _data) => println!("Message"),
                    NetEvent::Disconnected(_endpoint) => println!("disconnected"),
                });

                println!("exit?");

                //     let _client = ClientBuilder::new(&format!("ws://{}:9000", ip))
                //         .unwrap()
                //         .connect_insecure()
                //         .unwrap();
                //     thread::sleep(Duration::from_secs(1000));
            });

            let source = gst::ElementFactory::make("udpsrc")
                .property("port", 9001)
                .build()
                .unwrap();

            // let capsfilter = gst::ElementFactory::make("capsfilter")
            //     .property(
            //         "caps",
            //         ,
            //     )
            //     .build()
            //     .unwrap();

            let depay = gst::ElementFactory::make("rtph264depay").build().unwrap();
            let decode = gst::ElementFactory::make("decodebin").build().unwrap();
            let convert = gst::ElementFactory::make("videoconvert").build().unwrap();
            let sink = gst::ElementFactory::make("autovideosink").build().unwrap();

            let pipeline = gst::Pipeline::with_name("recv-pipeline");

            pipeline
                .add_many(&[&source, &depay, &decode, &convert, &sink])
                .unwrap();

            source
                .link_filtered(
                    &depay,
                    &gst::Caps::builder("application/x-rtp")
                        .field("media", "video")
                        .field("clock-rate", 90000)
                        .field("encoding-name", "H264")
                        .field("payload", 96)
                        .build(),
                )
                .unwrap();
            depay.link(&decode).unwrap();

            let convert_weak = convert.downgrade();
            decode.connect_pad_added(move |_, src_pad| {
                let sink_pad = match convert_weak.upgrade() {
                    None => return,
                    Some(s) => s.static_pad("sink").unwrap(),
                };
                src_pad.link(&sink_pad).unwrap();
            });

            convert.link(&sink).unwrap();

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
    println!("End of stream");
}
