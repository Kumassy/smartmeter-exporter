use bytes::{BytesMut, BufMut};
use log::info;
use std::io::{self, BufReader, BufRead};
use std::sync::{Arc, Mutex};
use std::{net::SocketAddr, io::Read, io::Write};
use std::error::Error;
use std::time::Duration;

use env_logger::{
    Builder,
    Env,
};
use prometheus_exporter::prometheus::register_gauge;
use rppal::uart::{Parity, Uart, Queue};


#[derive(Debug, Clone)]
struct MyUart {
    inner: Arc<Mutex<Uart>>
}

impl MyUart {
    fn new(uart: Uart) -> Self {
        Self {
            inner: Arc::new(Mutex::new(uart))
        }
    }
}

impl Read for MyUart {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.lock().expect("failed to acuire lock").read(buf).map_err(|e| 
            io::Error::new(io::ErrorKind::Other, e)
        )
    }
}

impl Write for MyUart {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().expect("failed to acuire lock").write(buf).map_err(|e| 
            io::Error::new(io::ErrorKind::Other, e)
        )
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().expect("failed to acuire lock").flush(Queue::Both).map_err(|e| 
            io::Error::new(io::ErrorKind::Other, e)
        )
    }
}

struct Sensor {
    writer: MyUart,
    reader: BufReader<MyUart>
}

impl Sensor {
    fn new(uart: Uart) -> Self {
        let myuart = MyUart::new(uart);
        let reader = BufReader::new(myuart.clone()); 
        Self {
            writer: myuart,
            reader
        }
    }

    fn read_line(&mut self) -> Result<String, Box<dyn Error>> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(line.trim_end_matches("\r\n").to_string())
    }

    fn write_all(&mut self, mut buf: BytesMut) -> Result<(), Box<dyn Error>> {
        buf.put(&b"\r\n"[..]);
        self.writer.write_all(&buf)?;
        Ok(())
    }
}

fn expect_or_err(got: &str, expected: &str) -> Result<(), Box<dyn Error>> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {}, got {}", expected, got).into())
    }
}

const B_ID: &str = std::env!("B_ID");
const B_PW: &str = std::env!("B_PW");

fn main() -> Result<(), Box<dyn Error>> {
    // pretty_env_logger::init();

    // Setup logger with default level info so we can see the messages from
    // prometheus_exporter.
    Builder::from_env(Env::default().default_filter_or("debug")).init();

    // Parse address used to bind exporter to.
    let addr_raw = "0.0.0.0:9186";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    // Start exporter and update metrics every five seconds.
    let exporter = prometheus_exporter::start(addr).expect("can not start exporter");
    let duration = std::time::Duration::from_millis(1000);

    // Create metric
    let my_metrics = register_gauge!("my_metrics", "my metrics")
        .expect("can not create gauge my_metrics");



    // Connect to the primary UART and configure it for 115.2 kbit/s, no
    // parity bit, 8 data bits and 1 stop bit.
    let mut uart = Uart::with_path("/dev/ttyAMA0", 115200, Parity::None, 8, 1)?;

    // Configure read() to block until at least 1 byte is received or timeout elapsed
    uart.set_read_mode(0, Duration::from_millis(2000))?;
    uart.set_write_mode(true)?;

    let mut sensor = Sensor::new(uart);


    // sensor.write_all(BytesMut::from("SKVER"))?;
    // let line = sensor.read_line()?;
    // println!("line: {:?}", line);
    // let line = sensor.read_line()?;
    // println!("line: {:?}", line);

    // reset
    sensor.write_all(BytesMut::from("SKRESET"))?;
    expect_or_err(&sensor.read_line()?, "SKRESET")?;
    expect_or_err(&sensor.read_line()?, "OK")?;

    // send id
    let command = format!("SKSETRBID {}", B_ID);
    sensor.write_all(BytesMut::from(command.as_bytes()))?;
    expect_or_err(&sensor.read_line()?, &command)?;
    expect_or_err(&sensor.read_line()?, "OK")?;

    // send pw
    let command = format!("SKSETPWD C {}", B_PW);
    sensor.write_all(BytesMut::from(command.as_bytes()))?;
    expect_or_err(&sensor.read_line()?, &command)?;
    expect_or_err(&sensor.read_line()?, "OK")?;

    // active scan
    sensor.write_all(BytesMut::from("SKSCAN 2 FFFFFFFF 6"))?;
    expect_or_err(&sensor.read_line()?, "SKSCAN 2 FFFFFFFF 6")?;
    expect_or_err(&sensor.read_line()?, "OK")?;

    std::thread::sleep(Duration::from_millis(10000));
    let mut i = 0;
    while i < 100 {
        let line = sensor.read_line()?;
        println!("scan result {:?}", line);

        i += 1;
    }


    loop {
        // Will block until duration is elapsed.
        let _guard = exporter.wait_duration(duration);

        info!("Updating metrics");

        let new_value = my_metrics.get() + 1.0;
        info!("New random value: {}", new_value);

        my_metrics.set(new_value);
    }

    Ok(())
}