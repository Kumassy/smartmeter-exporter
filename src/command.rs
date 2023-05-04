use bytes::{Bytes, BytesMut, BufMut};

use crate::echonet_lite::{EchonetLite, EHd, EHD1_ECHONET_LITE, EHD2_FORMAT1, EData, EDataFormat1, EOJ_MANAGEMENT_CONTROLLER, EOJ_HOUSING_LOW_VOLTAGE_SMART_METER, Esv, EDataProperty, EpcLowVoltageSmartMeter};

pub type Addr64 = str;
pub type IpAddr = str;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command<'a> {
    SkReset,
    SkSetRbid{
        id: &'a str,
    },
    SkSetPwd {
        pwd: &'a str,
    },
    ActiveScan {
        duration: u8,
    },
    SkSreg {
        sreg: u8,
        val: u32,
    },
    SkLl64 {
        addr64: &'a Addr64,
    },
    SkJoin {
        ipaddr: &'a IpAddr,
    },
    SendEnergyRequest {
        ipaddr: &'a IpAddr,
    }
}

impl Into<Bytes> for Command<'_> {
    fn into(self) -> Bytes {
        match self {
            Command::SkReset => {
                Bytes::from_static(b"SKRESET\r\n")
            },
            Command::SkSetRbid { id }=> {
                let mut cmd = BytesMut::new();
                cmd.put(&b"SKSETRBID "[..]);
                cmd.put(id.as_bytes());
                cmd.put(&b"\r\n"[..]);
                cmd.into()
            },
            Command::SkSetPwd { pwd } => {
                let mut cmd = BytesMut::new();
                cmd.put(&b"SKSETPWD "[..]);
                cmd.put(format!("{:X}", pwd.len()).as_bytes());
                cmd.put(&b" "[..]);
                cmd.put(pwd.as_bytes());
                cmd.put(&b"\r\n"[..]);
                cmd.into()
            },
            Command::ActiveScan { duration } => {
                let mut cmd = BytesMut::new();
                cmd.put(&b"SKSCAN 2 FFFFFFFF "[..]);
                cmd.put(format!("{:X}", duration).as_bytes());
                cmd.put(&b"\r\n"[..]);
                cmd.into()
            },
            Command::SkSreg { sreg, val } => {
                let mut cmd = BytesMut::new();
                cmd.put(&b"SKSREG S"[..]);
                cmd.put(format!("{:X}", sreg).as_bytes());
                cmd.put(&b" "[..]);
                cmd.put(format!("{:X}", val).as_bytes());
                cmd.put(&b"\r\n"[..]);
                cmd.into()
            },
            Command::SkLl64 { addr64 } => {
                let mut cmd = BytesMut::new();
                cmd.put(&b"SKLL64 "[..]);
                cmd.put(addr64.as_bytes());
                cmd.put(&b"\r\n"[..]);
                cmd.into()
            },
            Command::SkJoin { ipaddr } => {
                let mut cmd = BytesMut::new();
                cmd.put(&b"SKJOIN "[..]);
                cmd.put(ipaddr.as_bytes());
                cmd.put(&b"\r\n"[..]);
                cmd.into()
            },
            Command::SendEnergyRequest { ipaddr } => {
                // get current power consumption
                let get_now_p = EchonetLite {
                    ehd: EHd {
                        ehd1: EHD1_ECHONET_LITE,
                        ehd2: EHD2_FORMAT1,
                        tid: 0x0001,
                    },
                    edata: EData::EDataFormat1(EDataFormat1 {
                        seoj: EOJ_MANAGEMENT_CONTROLLER,
                        deoj: EOJ_HOUSING_LOW_VOLTAGE_SMART_METER,
                        esv: Esv::PROP_READ,
                        opc: 0x01,
                        props: vec![EDataProperty {
                            epc: EpcLowVoltageSmartMeter::INSTANTANEOUS_ENERGY,
                            pdc: 0x00,
                            edt: Bytes::new(),
                        }],
                    })
                };
                let get_now_p: Bytes = get_now_p.into();

                let mut cmd = BytesMut::from(format!("SKSENDTO 1 {} 0E1A 1 {:>04X} ", ipaddr, get_now_p.len()).as_bytes());
                cmd.put(get_now_p);
                cmd.put(&b"\r\n"[..]);
                cmd.into()
            },
        } 
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_sk_reset() {
        let cmd = Command::SkReset;
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKRESET\r\n"));
    }

    #[test]
    fn test_sk_set_rbid() {
        let cmd = Command::SkSetRbid { id: "12345678" };
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKSETRBID 12345678\r\n"));
    }

    #[test]
    fn test_sk_set_pwd() {
        let cmd = Command::SkSetPwd { pwd: "123XXXXXXXXX" };
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKSETPWD C 123XXXXXXXXX\r\n"));
    }

    #[test]
    fn test_active_scan() {
        let cmd = Command::ActiveScan { duration: 6 };
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKSCAN 2 FFFFFFFF 6\r\n"));
    }

    #[test]
    fn test_sk_sreg() {
        let cmd = Command::SkSreg { sreg: 0x02, val: 0x21 };
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKSREG S2 21\r\n"));
    }

    #[test]
    fn test_sk_ll64() {
        let cmd = Command::SkLl64 { addr64: "0123456789ABCDEF" };
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKLL64 0123456789ABCDEF\r\n"));
    }

    #[test]
    fn test_sk_join() {
        let cmd = Command::SkJoin { ipaddr: "FE80:0000:0000:0000:0123:4567:89ab:cdef" };
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKJOIN FE80:0000:0000:0000:0123:4567:89ab:cdef\r\n"));
    }

    #[test]
    fn test_send_energy_request() {
        let cmd = Command::SendEnergyRequest { ipaddr: "FE80:0000:0000:0000:0123:4567:89ab:cdef" };
        assert_eq!(std::convert::Into::<Bytes>::into(cmd), Bytes::from_static(b"SKSENDTO 1 FE80:0000:0000:0000:0123:4567:89ab:cdef 0E1A 1 000E \x10\x81\x00\x01\x05\xFF\x01\x02\x88\x01\x62\x01\xE7\x00\r\n"));
    }
}