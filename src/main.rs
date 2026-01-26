use ethel::state::State;
use janus::{context::EmptyRoutine, window::DisplayParameters};

fn main() {
    tracing_subscriber::FmtSubscriber::builder().init();

    let ctx = janus::context::StatefulContext::<EmptyRoutine, State>::new(
        EmptyRoutine,
        DisplayParameters::windowed("ethel", 1920, 1080),
    );
    janus::run(ctx);
}
