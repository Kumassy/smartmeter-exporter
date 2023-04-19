use bytes::{BytesMut, BufMut, Bytes, Buf};
use log::{info, debug};
use std::collections::HashMap;
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

    fn read_line(&mut self) -> Result<BytesMut, Box<dyn Error>> {
        let mut line = Vec::new();
        self.reader.read_until(b'\n',&mut line)?;
        let line = Bytes::from(line);
        debug!("read {:?}", line);

        let line = line.strip_suffix(b"\r\n").unwrap_or(&line);
        let line = BytesMut::from(line);
        Ok(line)
    }

    fn write_all(&mut self, mut buf: impl Into<BytesMut>) -> Result<(), Box<dyn Error>> {
        let mut buf = buf.into();
        buf.put(&b"\r\n"[..]);
        debug!("write {:?}", &buf);
        self.writer.write_all(&buf)?;
        Ok(())
    }

}

fn expect_or_err(got: impl Into<Bytes>, expected: impl Into<Bytes>) -> Result<(), Box<dyn Error>> {
    let got = got.into();
    let expected = expected.into();
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {}, got {}", String::from_utf8_lossy(&expected), String::from_utf8_lossy(&got)).into())
    }
}

fn assert_start_with_or_error(got: impl Into<Bytes>, start_with: impl Into<Bytes>) -> Result<(), Box<dyn Error>> {
    let got = got.into();
    let start_with = start_with.into();
    if got.starts_with(&start_with) {
        Ok(())
    } else {
        Err(format!("expected start with {}, got {}", String::from_utf8_lossy(&start_with), String::from_utf8_lossy(&got)).into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct PanDesc {
    channel: String,
    channel_page: String, 
    pan_id: String,
    addr: String,
    lqi: String,
    pair_id: String,
}

fn parse_pan_desc(sensor: &mut Sensor) -> Result<PanDesc, Box<dyn Error>> {
    let mut map = HashMap::new();
    for _ in 0..6 {
        let line = sensor.read_line()?.strip_prefix(b"  ").ok_or("parse error")?.to_vec();

        let mut parts = line.split(|c| *c == b':');

        let key = String::from_utf8(parts.next().ok_or("parse error")?.to_vec())?;
        let value = String::from_utf8(parts.next().ok_or("parse error")?.to_vec())?;
        map.insert(key, value);
    }
    info!("map: {:?}", map);

    Ok(PanDesc {
        channel: map.get("Channel").ok_or("parse error")?.to_string(),
        channel_page: map.get("Channel Page").ok_or("parse error")?.to_string(),
        pan_id: map.get("Pan ID").ok_or("parse error")?.to_string(),
        addr: map.get("Addr").ok_or("parse error")?.to_string(),
        lqi: map.get("LQI").ok_or("parse error")?.to_string(),
        pair_id: map.get("PairID").ok_or("parse error")?.to_string(),
    })
}

fn active_scan(sensor: &mut Sensor) -> Result<PanDesc, Box<dyn Error>> {
    // active scan
    sensor.write_all(BytesMut::from("SKSCAN 2 FFFFFFFF 6"))?;
    expect_or_err(sensor.read_line()?, "SKSCAN 2 FFFFFFFF 6")?;
    expect_or_err(sensor.read_line()?, "OK")?;


    // wait and parse scan result
    let total_wait_time = Duration::from_millis(0);
    let mut tmp = Err("unable to scan".into());
    let pan_desc = loop {
        if total_wait_time > Duration::from_secs(30) {
            return Err("scan timeout".into());
        }

        let line = sensor.read_line()?;
        if line.starts_with(b"EVENT 20") {
            expect_or_err(sensor.read_line()?, "EPANDESC")?;

            tmp = parse_pan_desc(sensor)
        } else if line.starts_with(b"EVENT 22") {
            break tmp;
        }
    };
    
    pan_desc
}

fn wait_for_connect(sensor: &mut Sensor) -> Result<(), Box<dyn Error>> {
    let total_wait_time = Duration::from_millis(0);
    loop {
        if total_wait_time > Duration::from_secs(30) {
            return Err("connect timeout".into());
        }

        let line = sensor.read_line()?;
        if line.starts_with(b"EVENT 24") {
            return Err("failed to connect to PANA".into());
        } else if line.starts_with(b"EVENT 25") {
            return Ok(());
        }
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
    sensor.write_all("SKRESET")?;
    expect_or_err(sensor.read_line()?, "SKRESET")?;
    expect_or_err(sensor.read_line()?, "OK")?;

    // send id
    let command = format!("SKSETRBID {}", B_ID);
    sensor.write_all(command.as_bytes())?;
    expect_or_err(sensor.read_line()?, command)?;
    expect_or_err(sensor.read_line()?, "OK")?;

    // send pw
    let command = format!("SKSETPWD C {}", B_PW);
    sensor.write_all(command.as_bytes())?;
    expect_or_err(sensor.read_line()?, command)?;
    expect_or_err(sensor.read_line()?, "OK")?;


    let pan_desc = active_scan(&mut sensor)?;
    println!("pan_desc: {:?}", pan_desc);

    // set channel
    let command = format!("SKSREG S2 {}", pan_desc.channel);
    sensor.write_all(command.as_bytes())?;
    expect_or_err(sensor.read_line()?, command)?;
    expect_or_err(sensor.read_line()?, "OK")?; 

    // set pan id
    let command = format!("SKSREG S3 {}", pan_desc.pan_id);
    sensor.write_all(command.as_bytes())?;
    expect_or_err(sensor.read_line()?, command)?;
    expect_or_err(sensor.read_line()?, "OK")?;

    // convert addr
    let command = format!("SKLL64 {}", pan_desc.addr);
    sensor.write_all(command.as_bytes())?;
    expect_or_err(sensor.read_line()?, command)?;
    let ipv6_addr = String::from_utf8(sensor.read_line()?.to_vec())?;

    // connect to pana
    let command = format!("SKJOIN {}", ipv6_addr);
    sensor.write_all(command.as_bytes())?;
    expect_or_err(sensor.read_line()?, command)?;
    expect_or_err(sensor.read_line()?, "OK")?;

    wait_for_connect(&mut sensor)?;
    info!("connected to PANA");

    let get_now_p = Bytes::from(&b"\x10\x81\x00\x01\x05\xFF\x01\x02\x88\x01\x62\x01\xE7\x00"[..]);
    let command = format!("SKSENDTO 1 {} 0E1A 1 {:>04x} ", ipv6_addr, get_now_p.len());
    let mut cmd = BytesMut::from(command.as_bytes());
    cmd.put(get_now_p);
    sensor.write_all(cmd)?;
    expect_or_err(sensor.read_line()?, command)?;
    expect_or_err(sensor.read_line()?, "OK")?;

    loop {
        let line = sensor.read_line()?;
        println!("line: {:?}", line);
    }

    // loop {
    //     // Will block until duration is elapsed.
    //     let _guard = exporter.wait_duration(duration);

    //     info!("Updating metrics");

    //     let new_value = my_metrics.get() + 1.0;
    //     info!("New random value: {}", new_value);

    //     my_metrics.set(new_value);
    // }

    // Ok(())
}