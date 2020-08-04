#[cfg(feature = "metrics")]
use influxdb::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello World!");

    #[cfg(feature = "metrics")]
    {
        let _client = Client::new("http://127.0.0.1:8086", "test");
        // and so on, rest of the influx code...
        // can't go further w/o an influxdb server actually here!

        println!("Influx says Hi!");
    }

    Ok(())
}
