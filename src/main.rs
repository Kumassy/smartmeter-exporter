use bytes::{BytesMut, BufMut, Bytes, Buf};
use log::{info, debug, error, warn};
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
use parser::{parser, PanDesc, IpAddr};
mod command;
use command::Command;

use crate::parser::{Response, EchonetLite, EData, EDataType1, EOJ_LOW_VOLTAGE_SMART_METER, EDataProperty, EpcLowVoltageSmartMeter};


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

fn active_scan(sensor: &mut MyUart, receiver: &mut Receiver<Response>) -> Result<PanDesc, Box<dyn Error>> {
    sensor.send_command(Command::ActiveScan { duration: 6 })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkScan { ..}) {
        return Err("SKSCAN failed".into());
    }

    let mut tmp = Err("unable to find sensor within duration".into());
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


fn initialize() -> Result<(MyUart, Receiver<Response>, IpAddr), Box<dyn Error>>  {
    let mut uart = Uart::with_path("/dev/ttyAMA0", 115200, Parity::None, 8, 1)?;

    // Configure read() to block until at least 1 byte is received or timeout elapsed
    uart.set_read_mode(0, Duration::from_millis(2000))?;
    uart.set_write_mode(true)?;

    let (sender, mut receiver) = channel();

    let mut uart = MyUart::new(uart);
    let mut uart_ = uart.clone();
    std::thread::spawn(move || {
        let mut buf = BytesMut::with_capacity(1024);
        loop {
            let mut b = [0; 1024];

            match uart_.read(&mut b) {
                Ok(n) if n > 0 => {
                    debug!("read: {:?}", &b[..n]);
                    buf.put(&b[..n]);
                },
                Err(e) => {
                    error!("uart read error: {:?}", e);
                    break;
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

                    // finish reading from device
                    break;
                }
            }
        }
        // explicitly drop sender, so receiver.recv() will return Err
        drop(sender);
    });

    // reset
    uart.send_command(Command::SkReset)?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkReset) {
        return Err("SKRESET failed".into());
    }

    // send id
    uart.send_command(Command::SkSetRbid { id: B_ID })?;
    let r = receiver.recv()?;

    if ! matches!(r, Response::SkSetRbid { ..}) {
        return Err("SKSETRBID failed".into());
    }

    // send pw
    uart.send_command(Command::SkSetPwd { pwd: B_PW })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkSetPwd { ..} ) {
        return Err("SKSETPWD failed".into());
    }

    let pan_desc = active_scan(&mut uart, &mut receiver)?;
    debug!("pan_desc: {:?}", pan_desc);

    // set channel
    uart.send_command(Command::SkSreg { sreg: 0x02, val: pan_desc.channel as u32 })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkSreg { ..} ) {
        return Err("SKSREG failed".into());
    }

    // set pan id
    uart.send_command(Command::SkSreg { sreg: 0x03, val: pan_desc.pan_id as u32 })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkSreg { ..} ) {
        return Err("SKSREG failed".into());
    }

    // convert addr
    uart.send_command(Command::SkLl64 { addr64: &pan_desc.addr })?;
    let r = receiver.recv()?;
    let ipv6_addr = match r {
        Response::SkLl64 { ipaddr, .. } => ipaddr,
        _ => {
            return Err("SKLL64 failed".into());
        }
    };

    // connect to pana
    uart.send_command(Command::SkJoin { ipaddr: &ipv6_addr })?;
    let r = receiver.recv()?;
    if ! matches!(r, Response::SkJoin { ..} ) {
        return Err("SKJOIN failed".into());
    }

    wait_for_connect(&mut uart, &mut receiver)?;
    info!("connected to PANA");
    Ok((uart, receiver, ipv6_addr))
}

fn main() -> Result<(), Box<dyn Error>> {
    Builder::from_env(Env::default().default_filter_or("debug")).init();

    let addr_raw = "0.0.0.0:9186";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    let exporter = prometheus_exporter::start(addr).expect("can not start exporter");
    let duration = std::time::Duration::from_millis(10000);

    let counter_error_initialize = register_gauge!("counter_error_initialize", "# of error when try to initialize sensor with PANA")
        .expect("can not create gauge counter_error_initialize");
    let counter_error_sksendto = register_gauge!("counter_error_sksendto", "# of error when sending data to sensor")
        .expect("can not create gauge counter_error_sksendto");
    let counter_success_initialize = register_gauge!("counter_success_initialize", "# of times client finished initialization")
        .expect("can not create gauge counter_success_initialize");
    let counter_request_energy = register_gauge!("counter_request_energy", "# of times client send energy request")
        .expect("can not create gauge counter_request_energy");
    let instantaneous_energy = register_gauge!("instantaneous_energy", "Current Power Consumption in Watt")
        .expect("can not create gauge instantaneous_energy");

    loop {
        let (mut uart, mut receiver, ipv6_addr) = match initialize() {
            Ok(ipv6_addr) => ipv6_addr,
            Err(e) => {
                error!("unable to initialize smartmeter: {:?}", e);
                std::thread::sleep(Duration::from_secs(30));
                counter_error_initialize.inc();
                continue;
            }
        };
        counter_success_initialize.inc();

        // main loop
        'main: loop {
            let _guard = exporter.wait_duration(duration);
            if let Err(e) = uart.send_command(Command::SendEnergyRequest { ipaddr: &ipv6_addr }) {
                error!("failed to send command: {:?}", e);
                counter_error_sksendto.inc();
                break 'main;
            }
            counter_request_energy.inc();
    

            // wait response for energy request
            'wait_response: loop {
                let r = match receiver.recv() {
                    Ok(r) => r,
                    Err(e) => {
                        error!("reader thread closed when they encouter error: {:?}", e);
                        break 'main;
                    }
                };
                info!("{:?}", r);
    
                match r {
                    Response::SkSendTo{ result: 0x00, .. } => {
                        debug!("send energy request success");
                    },
                    Response::SkSendTo{ result: _, .. } => {
                        warn!("failed to send energy request: {:?}", r);
                        counter_error_sksendto.inc();
                        break 'wait_response;
                    },
                    Response::ERxUdp {
                        data: EchonetLite {
                            edata: EData::EDataType1(EDataType1 {
                                seoj: EOJ_LOW_VOLTAGE_SMART_METER,
                                props,
                                ..
                        }), .. }, ..
                    } => {
                        for prop in props {
                            match prop {
                                EDataProperty {
                                    epc: EpcLowVoltageSmartMeter::INSTANTANEOUS_CURRENT,
                                    pdc: 0x04,
                                    mut edt,
                                    ..
                                } => {
                                    let power = edt.get_u32();
                                    instantaneous_energy.set(power as f64);
                                },
                                _ => {
                                    // ignore
                                }
                            }
                        }
                        break 'wait_response;
                    },
                    _ => {
                        // ignore
                    }
                }
            }
        }
    }
}