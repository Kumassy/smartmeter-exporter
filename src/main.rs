use bytes::{BytesMut, BufMut, Bytes, Buf};
use log::{info, debug, error};
use std::io::{self, BufReader, BufRead};
use std::sync::{Arc, Mutex};
use std::{net::SocketAddr, io::Read, io::Write};
use std::error::Error;
use std::time::Duration;
use std::sync::mpsc::{channel, Receiver};

use env_logger::{
    Builder,
    Env,
};
use prometheus_exporter::prometheus::register_gauge;
use rppal::uart::{Parity, Uart, Queue};

mod parser;
use parser::{parser, PanDesc};
mod command;
use command::Command;

use crate::parser::Response;


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

    fn send_command(&mut self, cmd: Command) -> Result<(), Box<dyn Error>> {
        let cmd: Bytes = cmd.into();
        self.write_all(&cmd)?;
        Ok(())
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
        debug!("write: {:?}", buf);
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

// #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
// struct PanDesc {
//     channel: String,
//     channel_page: String, 
//     pan_id: String,
//     addr: String,
//     lqi: String,
//     pair_id: String,
// }

// fn parse_pan_desc(sensor: &mut Sensor) -> Result<PanDesc, Box<dyn Error>> {
//     let mut map = HashMap::new();
//     for _ in 0..6 {
//         let line = sensor.read_line()?.strip_prefix(b"  ").ok_or("parse error")?.to_vec();

//         let mut parts = line.split(|c| *c == b':');

//         let key = String::from_utf8(parts.next().ok_or("parse error")?.to_vec())?;
//         let value = String::from_utf8(parts.next().ok_or("parse error")?.to_vec())?;
//         map.insert(key, value);
//     }
//     info!("map: {:?}", map);

//     Ok(PanDesc {
//         channel: map.get("Channel").ok_or("parse error")?.to_string(),
//         channel_page: map.get("Channel Page").ok_or("parse error")?.to_string(),
//         pan_id: map.get("Pan ID").ok_or("parse error")?.to_string(),
//         addr: map.get("Addr").ok_or("parse error")?.to_string(),
//         lqi: map.get("LQI").ok_or("parse error")?.to_string(),
//         pair_id: map.get("PairID").ok_or("parse error")?.to_string(),
//     })
// }

fn active_scan(sensor: &mut MyUart, receiver: &mut Receiver<Response>) -> Result<PanDesc, Box<dyn Error>> {
    // active scan
    sensor.send_command(Command::ActiveScan { duration: 6 })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkScan { ..}) {
        return Err("SKSCAN failed".into());
    }

    let mut tmp = Err("unable to scan".into());
    loop {
        let r = receiver.recv()?;
        match r {
            Response::Event { num, sender, param } => {
                if num == 0x22 {
                    return tmp;
                }
            }
            Response::EPanDesc(pandesc) => {
                tmp = Ok(pandesc);
            },
            _ => {
            }
        }
    }
}

fn wait_for_connect(sensor: &mut MyUart, receiver: &mut Receiver<Response>) -> Result<(), Box<dyn Error>> {
    let total_wait_time = Duration::from_millis(0);
    loop {
        if total_wait_time > Duration::from_secs(30) {
            return Err("connect timeout".into());
        }

        let r = receiver.recv()?;
        match r {
            Response::Event { num: 0x24, .. } => {
                return Err("failed to connect to PANA".into());
            },
            Response::Event { num: 0x25, .. } => {
                return Ok(());
            }
            _ => {
            }
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

    let (sender, mut receiver) = channel();

    let mut uart = MyUart::new(uart);
    let mut uart_ = uart.clone();
    std::thread::spawn(move || {
        let mut buf = BytesMut::with_capacity(64);
        loop {
            let mut b = [0; 1024];

            match uart_.read(&mut b) {
                Ok(n) if n > 0 => {
                    debug!("read: {:?}", &b[..n]);
                    buf.put(&b[..n]);
                },
                Err(e) => {
                    error!("uart read error: {:?}", e);
                }
                _ => {}
            }

            debug!("current buf: {:?}", buf);
            match parser(&buf) {
                Ok((rest, line)) => {
                    debug!("parsed response: {:?}", line);
                    sender.send(line).unwrap();

                    buf = BytesMut::from(rest);
                },
                Err(nom::Err::Incomplete(n)) => {
                    // not enough data
                    debug!("parse incomplate: {:?}", n);
                },
                Err(e) => {
                    error!("parse error: {:?}", e);
                }
            }
        }
    });

    // reset
    uart.send_command(Command::SkReset)?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkReset) {
        error!("SKRESET failed");
    }

    // send id
    uart.send_command(Command::SkSetRbid { id: B_ID })?;
    let r = receiver.recv()?;

    if ! matches!(r, Response::SkSetRbid { ..}) {
        error!("SKSETRBID failed");
    }

    // send pw
    uart.send_command(Command::SkSetPwd { pwd: B_PW })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkSetPwd { ..} ) {
        error!("SKSETPWD failed");
    }

    let pan_desc = active_scan(&mut uart, &mut receiver)?;
    println!("pan_desc: {:?}", pan_desc);

    // set channel
    uart.send_command(Command::SkSreg { sreg: 0x02, val: pan_desc.channel as u32 })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkSreg { ..} ) {
        error!("SKSREG failed");
    }

    // set pan id
    uart.send_command(Command::SkSreg { sreg: 0x03, val: pan_desc.pan_id as u32 })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkSreg { ..} ) {
        error!("SKSREG failed");
    }

    // convert addr
    uart.send_command(Command::SkLl64 { addr64: &pan_desc.addr })?;
    let r = receiver.recv()?;
    let ipv6_addr = match r {
        Response::SkLl64 { ipaddr, .. } => ipaddr,
        _ => {
            error!("SKLL64 failed");
            return Err("SKLL64 failed".into());
        }
    };

    // connect to pana
    uart.send_command(Command::SkJoin { ipaddr: &ipv6_addr })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkJoin { ..} ) {
        error!("SKJOIN failed");
    }

    wait_for_connect(&mut uart, &mut receiver)?;
    info!("connected to PANA");

    loop {
        // send
        uart.send_command(Command::SendEnergyRequest { ipaddr: &ipv6_addr })?;

        loop {
            let r = receiver.recv()?;
            info!("{:?}", r);

            match r {
                Response::SkSendTo{ result: 0x00, .. } => {
                    break;
                },
                _ => {
                }
            }
        }
        std::thread::sleep(Duration::from_secs(15));
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