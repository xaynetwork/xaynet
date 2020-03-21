set -x

if [ "$RELEASE_BUILD" == "true" ] ; then
    cargo build --release --features=telemetry,influx_metrics
    mv ./target/release/aggregator  /bin/aggregator
    mv ./target/release/coordinator /bin/coordinator
else
    cargo build --features=telemetry,influx_metrics
    mv ./target/debug/aggregator  /bin/aggregator
    mv ./target/debug/coordinator /bin/coordinator
fi
