// write test module

use std::fmt;

use bytes::Bytes;
use nom::{IResult, bytes::streaming::{tag, take_while1, take, take_while_m_n }, branch::alt, character::{streaming::{space1, hex_digit1, crlf}, is_alphanumeric, is_hex_digit}, sequence::{tuple, delimited, preceded}, combinator::{map_res, map, opt, all_consuming, recognize}, ToUsize, number::streaming::{be_u8, be_u16}, multi::count };

use crate::echonet_lite::{EchonetLite, EData, EDataFormat1, Eoj, EDataProperty, EHd};

pub type Addr64 = String;
pub type IpAddr = String;
#[derive(PartialEq)]
pub enum Response {
    Ok,

    // echo backs
    SkReset,
    SkSetRbid{
        id: String,
    },
    SkSetPwd {
        len: u8,
        pwd: String,
    },
    SkScan {
        mode: u8,
        channel_mask: u32,
        duration: u8,
    },
    SkSreg {
        sreg: u8,
        val: u32,
    },
    SkLl64  {
        addr64: Addr64,
        ipaddr: IpAddr,
    },
    SkJoin {
        ipaddr: IpAddr,
    },
    SkSendTo {
        handle: u8,
        ipaddr: IpAddr,
        port: u16,
        sec: u8,
        datalen: u16,
        result: u8, // param of Event 0x21
    },

    // events
    Event {
        num: u8,
        sender: IpAddr,
        param: Option<u8>,
    },
    EPanDesc(PanDesc),
    ERxUdp {
        sender: IpAddr,
        dest: IpAddr,
        rport: u16,
        lport: u16,
        senderlla: Addr64,
        secured: u8,
        datalen: u16,
        data: EchonetLite,
    }
}

#[derive(PartialEq, Default, Clone)]
pub struct PanDesc {
    pub channel: u8,
    pub channel_page: u8,
    pub pan_id: u16,
    pub addr: String, // Addr64
    pub lqi: u8,
    pub pair_id: String, // char[8]
}


impl fmt::Debug for PanDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PanDesc")
         .field("channel", &format_args!("{:#x}", self.channel))
         .field("channel_page", &format_args!("{:#x}", self.channel_page))
         .field("pan_id", &format_args!("{:#x}", self.pan_id))
         .field("addr", &self.addr)
         .field("lqi", &format_args!("{:#x}", self.lqi))
         .field("pair_id", &self.pair_id)
         .finish()
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Ok {
            } => {
                f.debug_struct("Ok")
                 .finish()
            },
            Response::SkReset {
            } => {
                f.debug_struct("SkReset")
                 .finish()
            },
            Response::SkSetRbid {
                id,
            } => {
                f.debug_struct("SkSetRbid")
                 .field("id", &id)
                 .finish()
            },
            Response::SkSetPwd {
                len,
                pwd,
            } => {
                f.debug_struct("SkSetPwd")
                 .field("len", &len)
                 .field("pwd", &pwd)
                 .finish()
            },
            Response::SkScan {
                mode,
                channel_mask,
                duration,
            } => {
                f.debug_struct("SkScan")
                 .field("mode", &format_args!("{:#x}", mode))
                 .field("channel_mask", &format_args!("{:#x}", channel_mask))
                 .field("duration", &duration)
                 .finish()
            },
            Response::SkSreg {
                sreg,
                val,
            } => {
                f.debug_struct("SkSreg")
                 .field("sreg", &format_args!("{:#x}", sreg))
                 .field("val", &format_args!("{:#x}", val))
                 .finish()
            },
            Response::SkLl64 {
                addr64,
                ipaddr,
            } => {
                f.debug_struct("SkLl64")
                 .field("addr64", &addr64)
                 .field("ipaddr", &ipaddr)
                 .finish()
            },
            Response::SkJoin {
                ipaddr,
            } => {
                f.debug_struct("SkJoin")
                 .field("ipaddr", &ipaddr)
                 .finish()
            },
            Response::SkSendTo {
                handle,
                ipaddr,
                port,
                sec,
                datalen,
                result,
            } => {
                f.debug_struct("SkSendTo")
                 .field("handle", &format_args!("{:#x}", handle))
                 .field("ipaddr", &ipaddr)
                 .field("port", &port)
                 .field("sec", &format_args!("{:#x}", sec))
                 .field("datalen", &datalen)
                 .field("result", &format_args!("{:#x}", result))
                 .finish()
            },
            Response::Event {
                num,
                sender,
                param,
            } => {
                f.debug_struct("Event")
                 .field("num", &format_args!("{:#x}", num))
                 .field("sender", &sender)
                 .field("param", &param)
                 .finish()
            },
            Response::EPanDesc(pan_desc) => {
                f.debug_struct("EPanDesc")
                 .field("pan_desc", &pan_desc)
                 .finish()
            },
            Response::ERxUdp {
                sender,
                dest,
                rport,
                lport,
                senderlla,
                secured,
                datalen,
                data,
            } => {
                f.debug_struct("ERxUdp")
                 .field("sender", &sender)
                 .field("dest", &dest)
                 .field("rport", &rport)
                 .field("lport", &lport)
                 .field("senderlla", &senderlla)
                 .field("secured", &format_args!("{:#x}", secured))
                 .field("datalen", &datalen)
                 .field("data", &data)
                 .finish()
            },
        }
    }
}

fn parse_ok(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, _) = tuple((tag("OK"), crlf))(input)?;
    Ok((input, Response::Ok))
}

fn parse_skreset(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, _) = tuple((tag("SKRESET"), crlf))(input)?;
    let (input, _) = parse_ok(input)?;

    Ok((input, Response::SkReset))
}

fn parse_sksetrbid(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, id, _)) = tuple((
        tag("SKSETRBID"),
        space1,
        take_while1(is_alphanumeric),
        crlf,
    ))(input)?;
    let (input, _) = parse_ok(input)?;

    let id = std::str::from_utf8(id).map_err(|e|
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::AlphaNumeric))
    )?;

    Ok((input, Response::SkSetRbid {
        id: id.to_string()
    }))
}

fn from_hex_u8(input: &[u8]) -> Result<u8, std::num::ParseIntError> {
    let str = String::from_utf8_lossy(input);
    u8::from_str_radix(&str, 16)
}

fn from_hex_u16(input: &[u8]) -> Result<u16, std::num::ParseIntError> {
    let str = String::from_utf8_lossy(input);
    u16::from_str_radix(&str, 16)
}

fn from_hex_u32(input: &[u8]) -> Result<u32, std::num::ParseIntError> {
    let str = String::from_utf8_lossy(input);
    u32::from_str_radix(&str, 16)
}
fn from_hex_u64(input: &[u8]) -> Result<u64, std::num::ParseIntError> {
    let str = String::from_utf8_lossy(input);
    u64::from_str_radix(&str, 16)
}

fn parse_sksetpwd(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, len, _, pwd, _)) = tuple((
        tag("SKSETPWD"),
        space1,
        map_res(hex_digit1, from_hex_u8),
        space1,
        take_while1(is_alphanumeric),
        crlf,
    ))(input)?;
    let (input, _) = parse_ok(input)?;

    if pwd.len() != len.to_usize() {
        return Err(nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Verify)));
    }

    let pwd = std::str::from_utf8(pwd).map_err(|e|
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::AlphaNumeric))
    )?;

    Ok((input, Response::SkSetPwd {
        len,
        pwd: pwd.to_string(),
    }))
}

fn parse_skscan(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, mode, _, channel_mask, _, duration, _)) = tuple((
        tag("SKSCAN"),
        space1,
        map_res(hex_digit1, from_hex_u8),
        space1,
        map_res(hex_digit1, from_hex_u32),
        space1,
        map_res(hex_digit1, from_hex_u8),
        crlf,
    ))(input)?;
    let (input, _) = parse_ok(input)?;

    Ok((input, Response::SkScan {
        mode,
        channel_mask,
        duration,
    }))
}

fn parse_ipv6_addr(input: &[u8]) -> IResult<&[u8], IpAddr> {
    let (input, addr) = recognize(
        tuple((
        take_while_m_n(4, 4, is_hex_digit),
        tag(":"),
        take_while_m_n(4, 4, is_hex_digit),
        tag(":"),
        take_while_m_n(4, 4, is_hex_digit),
        tag(":"),
        take_while_m_n(4, 4, is_hex_digit),
        tag(":"),
        take_while_m_n(4, 4, is_hex_digit),
        tag(":"),
        take_while_m_n(4, 4, is_hex_digit),
        tag(":"),
        take_while_m_n(4, 4, is_hex_digit),
        tag(":"),
        take_while_m_n(4, 4, is_hex_digit),
    ))
    )(input)?;

    let addr = std::str::from_utf8(addr).map_err(|e|
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::AlphaNumeric))
    )?;

    Ok((input, addr.to_string()))
}

fn parse_event(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, num, _, sender, _, param, _)) = tuple((
        tag("EVENT"),
        space1,
        map_res(hex_digit1, from_hex_u8),
        space1,
        parse_ipv6_addr,
        opt(space1),
        opt(map_res(hex_digit1, from_hex_u8)),
        crlf,
    ))(input)?;

    Ok((input, Response::Event {
        num,
        sender,
        param,
    }))
}
fn parse_epandesc(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, channel, channel_page, pan_id, addr, lqi, pair_id)) = tuple((
        tuple((tag("EPANDESC"), crlf)),
        delimited(tag("  Channel:"), map_res(hex_digit1, from_hex_u8), crlf),
        delimited(tag("  Channel Page:"), map_res(hex_digit1, from_hex_u8), crlf),
        delimited(tag("  Pan ID:"), map_res(hex_digit1, from_hex_u16), crlf),
        delimited(tag("  Addr:"), take_while1(is_alphanumeric), crlf),
        delimited(tag("  LQI:"), map_res(hex_digit1, from_hex_u8), crlf),
        delimited(tag("  PairID:"), take_while1(is_alphanumeric), crlf),
    ))(input)?;

    let addr = std::str::from_utf8(addr).map_err(|e|
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::AlphaNumeric))
    )?;
    let pair_id = std::str::from_utf8(pair_id).map_err(|e|
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::AlphaNumeric))
    )?;

    Ok((input, Response::EPanDesc(PanDesc {
        channel,
        channel_page,
        pan_id,
        addr: addr.to_string(),
        lqi,
        pair_id: pair_id.to_string(),
    })))
}

fn parse_sksreg(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, sreg, _, val, _)) = tuple((
        tag("SKSREG"),
        space1,
        preceded(tag("S"), map_res(hex_digit1, from_hex_u8)),
        space1,
        map_res(hex_digit1, from_hex_u32),
        crlf,
    ))(input)?;
    let (input, _) = parse_ok(input)?;

    Ok((input, Response::SkSreg {
        sreg,
        val,
    }))
}

fn parse_skll64(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, addr, _)) = tuple((
        tag("SKLL64"),
        space1,
        take_while1(is_alphanumeric),
        crlf,
    ))(input)?;
    let (input, (ipaddr, _)) = tuple((
        parse_ipv6_addr,
        crlf,
    ))(input)?;


    let addr = std::str::from_utf8(addr).map_err(|_e|
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::AlphaNumeric))
    )?;

    Ok((input, Response::SkLl64 {
        addr64: addr.to_string(),
        ipaddr,
    }))
}

fn parse_skjoin(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, ipaddr, _)) = tuple((
        tag("SKJOIN"),
        space1,
        parse_ipv6_addr,
        crlf,
    ))(input)?;
    let (input, _) = parse_ok(input)?;

    Ok((input, Response::SkJoin {
        ipaddr,
    }))
}

fn parse_erxudp(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, sender, _, dest, _, rport, _, lport, _, senderlla, _, secured, _, datalen, _)) = tuple((
        tag("ERXUDP"),
        space1,
        parse_ipv6_addr,
        space1,
        parse_ipv6_addr,
        space1,
        map_res(hex_digit1, from_hex_u16),
        space1,
        map_res(hex_digit1, from_hex_u16),
        space1,
        take_while1(is_alphanumeric),
        space1,
        map_res(hex_digit1, from_hex_u8),
        space1,
        map_res(hex_digit1, from_hex_u16),
        space1,
    ))(input)?;

    let addr = std::str::from_utf8(senderlla).map_err(|_e|
        nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::AlphaNumeric))
    )?;

    let (input, data) = take(datalen)(input)?;
    let (data, ehd) = parse_ehd(data)?;

    let edata = if ehd.ehd1 == 0x10 && ehd.ehd2 == 0x81 {
        let (_, edata) = all_consuming(parse_edata)(data)?;

        edata
    } else {
        let bytes = Bytes::copy_from_slice(data);
        EData::InvalidEData(bytes)
    };
    let (input, _) = crlf(input)?;

    Ok((input, Response::ERxUdp {
        sender,
        dest,
        rport,
        lport,
        senderlla: addr.to_string(),
        secured,
        datalen,
        data: EchonetLite {
            ehd,
            edata
        }
    }))
}

fn parse_ehd(input: &[u8]) -> IResult<&[u8], EHd> {
    let (input, (ehd1, ehd2, tid)) = tuple((
        be_u8,
        be_u8,
        be_u16,
    ))(input)?;

    let ehd = EHd {
        ehd1,
        ehd2,
        tid,
    };
    Ok((input, ehd))
}

fn parse_edata_property(input: &[u8]) -> IResult<&[u8], EDataProperty> {
    let (input, epc) = be_u8(input)?;
    let (input, pdc) = be_u8(input)?;
    let (input, edt) = take(pdc)(input)?;

    let edt = Bytes::copy_from_slice(edt);

    let p = EDataProperty {
        epc,
        pdc,
        edt,
    };
    Ok((input, p))
}


fn parse_eoj(input: &[u8]) -> IResult<&[u8], Eoj> {
    let (input, (class_group_code, class_code, instance_code)) = tuple((
        be_u8,
        be_u8,
        be_u8,
    ))(input)?;

    Ok((input, Eoj {
        class_group_code,
        class_code,
        instance_code,
    }))
}

fn parse_edata(input: &[u8]) -> IResult<&[u8], EData> {
    let (input, (seoj, deoj, esv, opc)) = tuple((
        parse_eoj,
        parse_eoj,
        be_u8,
        be_u8,
    ))(input)?;

    let (input, props) = count(parse_edata_property, opc as usize)(input)?;

    Ok((input, EData::EDataFormat1(EDataFormat1 {
        seoj,
        deoj,
        esv,
        opc,
        props,
    })))
}

fn parse_sksendto(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, (_, _, handle, _, ipaddr, _, port, _, sec, _, datalen, _, _)) = tuple((
        tag("SKSENDTO"),
        space1,
        map_res(hex_digit1, from_hex_u8),
        space1,
        parse_ipv6_addr,
        space1,
        map_res(hex_digit1, from_hex_u16),
        space1,
        map_res(hex_digit1, from_hex_u8),
        space1,
        map_res(hex_digit1, from_hex_u16),
        space1,
        crlf,
    ))(input)?;

    let (input, event) = parse_event(input)?;
    let (input, _) = parse_ok(input)?;
    let (input, _) = crlf(input)?;


    if let Response::Event { num: 0x21, param: Some(result), .. } = event {
        Ok((input, Response::SkSendTo {
            handle,
            ipaddr,
            port,
            sec,
            datalen,
            result,
        }))
    } else {
        Err(nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Verify)))
    }
}

pub fn parser(input: &[u8]) -> IResult<&[u8], Response> {
    alt((
        parse_skreset,
        parse_sksetrbid,
        parse_sksetpwd,
        parse_skscan,
        parse_event,
        parse_epandesc,
        parse_sksreg,
        parse_skll64,
        parse_skjoin,
        parse_erxudp,
        parse_sksendto,
    ))(input)
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::{io::{Cursor, BufReader, BufRead}};

    #[test]
    fn test_parse_cursor() -> Result<(), Box<dyn std::error::Error>> {
        let c = Cursor::new(b"AAA\r\nBBB\r\n");
        let mut reader = BufReader::new(c);

        let mut line = Vec::new();
        reader.read_until(b'\n', &mut line)?;
        assert_eq!(line, b"AAA\r\n");

        let mut line = Vec::new();
        reader.read_until(b'\n', &mut line)?;
        assert_eq!(line, b"BBB\r\n");

        Ok(())
    }
    #[test]
    fn test_parse_ok() {
        let (rest, response) = parse_ok(&b"OK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::Ok);
    }

    #[test]
    fn test_parse_skreset() {
        let (rest, response) = parser(&b"SKRESET\r\nOK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkReset);
    }

    #[test]
    fn test_parse_sksetrbid() {
        let (rest, response) = parser(&b"SKSETRBID 11111122222222333333334444444AAA\r\nOK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkSetRbid {
            id: "11111122222222333333334444444AAA".to_string()
        });
    }

    #[test]
    fn test_from_hex_u8() {
        assert_eq!(from_hex_u8(b"3"), Ok(0x3));
        assert_eq!(from_hex_u8(b"e"), Ok(0xe));
        assert_eq!(from_hex_u8(b"2F"), Ok(0x2F));
    }

    #[test]
    fn test_from_hex_invalid_utf8() {
        assert!(matches!(from_hex_u8(b"\xf1"), Err(std::num::ParseIntError { .. }))); // invalid utf8
        assert!(matches!(from_hex_u8(b"\x11"), Err(std::num::ParseIntError { .. }))); // valid utf8 but not a number
        assert!(matches!(from_hex_u8(b"FFFFFFFF"), Err(std::num::ParseIntError { .. }))); // overflow
    }

    #[test]
    fn test_from_hex_u16() {
        assert_eq!(from_hex_u16(b"1234"), Ok(0x1234));
        assert_eq!(from_hex_u16(b"FE80"), Ok(0xfe80));
    }

    #[test]
    fn test_from_hex_u32() {
        assert_eq!(from_hex_u32(b"12345678"), Ok(0x12345678));
        assert_eq!(from_hex_u32(b"FFFFFFFF"), Ok(0xffffffff));
    }

    #[test]
    fn test_from_hex_u64() {
        assert_eq!(from_hex_u64(b"C0F94500404B6F57"), Ok(0xc0f94500404b6f57));
    }

    #[test]
    fn test_parse_sksetpwd() {
        let (rest, response) = parser(&b"SKSETPWD C 123XXXXXXXXX\r\nOK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkSetPwd {
            len: 0x0c,
            pwd: "123XXXXXXXXX".to_string(),
        });
    }

    #[test]
    fn test_parse_sksetpwd_wrong_length_field() {
        let res = parser(&b"SKSETPWD F 123XXXXXXXXX\r\nOK\r\n"[..]);
        assert_eq!(res, Err(nom::Err::Failure(nom::error::Error::new(&b""[..], nom::error::ErrorKind::Verify))));
    }

    #[test]
    fn test_parse_skscan() {
        let (rest, response) = parser(&b"SKSCAN 2 FFFFFFFF 6\r\nOK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkScan {
            mode: 2,
            channel_mask: 0xffffffff,
            duration: 6,
        });
    }

    #[test]
    fn test_parse_ipv6_addr() {
        let (rest, addr) = parse_ipv6_addr(&b"FE80:0000:0000:0000:0123:4567:89ab:cdef\r\n"[..]).unwrap();
        assert_eq!(rest, &b"\r\n"[..]); // need delimiter because streaming parser read as much u16 hex text as possible
        assert_eq!(addr, "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string());
    }

    #[test]
    fn test_parse_event() {
        let (rest, response) = parser(&b"EVENT 20 FE80:0000:0000:0000:0123:4567:89ab:cdef\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::Event {
            num: 0x20,
            sender: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
            param: None,
        });

        let (rest, response) = parser(&b"EVENT 22 FE80:0000:0000:0000:0123:4567:89ab:cdef\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::Event {
            num: 0x22,
            sender: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
            param: None,
        });

        let (rest, response) = parser(&b"EVENT 21 FE80:0000:0000:0000:0123:4567:89ab:cdef 02\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::Event {
            num: 0x21,
            sender: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
            param: Some(0x02),
        });
    }

    #[test]
    fn test_parse_epandesc() {
        let (rest, epandesc) = parser(&b"EPANDESC\r\n  Channel:21\r\n  Channel Page:09\r\n  Pan ID:8888\r\n  Addr:001D129012345678\r\n  LQI:E1\r\n  PairID:00AXXXXX\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(epandesc, Response::EPanDesc(PanDesc {
            channel: 0x21,
            channel_page: 0x09,
            pan_id: 0x8888,
            addr: "001D129012345678".to_string(),
            lqi: 0xe1,
            pair_id: "00AXXXXX".to_string(),
        }));
    }

    #[test]
    fn test_parse_sksreg() {
        let (rest, response) = parser(&b"SKSREG S2 1A\r\nOK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkSreg {
            sreg: 2,
            val: 0x1a,
        });

        let (rest, response) = parser(&b"SKSREG S3 EF66\r\nOK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkSreg {
            sreg: 3,
            val: 0xef66,
        });
    }

    #[test]
    fn test_parse_skll64() {
        let (rest, response) = parser(&b"SKLL64 0123456789ABCDEF\r\nFE80:0000:0000:0000:0123:4567:89ab:cdef\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkLl64 {
            addr64: "0123456789ABCDEF".to_string(),
            ipaddr: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
        });
    }

    #[test]
    fn test_parse_skjion() {
        let (rest, response) = parser(&b"SKJOIN FE80:0000:0000:0000:0123:4567:89ab:cdef\r\nOK\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkJoin {
            ipaddr: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
        });
    }

    #[test]
    fn test_parse_ehd() {
        let (rest, ehd) = parse_ehd(&b"\x10\x81\x00\x01"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(ehd, EHd {
            ehd1: 0x10,
            ehd2: 0x81,
            tid: 0x0001,
        });
    }

    #[test]
    fn test_parse_eoj() {
        let (rest, eoj) = parse_eoj(&b"\x05\xFF\x01"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(eoj, Eoj {
            class_group_code: 0x05,
            class_code: 0xff,
            instance_code: 0x01,
        });
    }

    #[test]
    fn test_parse_edata() {
        let (rest, edata) = parse_edata(&b"\x05\xff\x01\x02\x88\x01b\x01\xe7\x00"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(edata, EData::EDataFormat1(EDataFormat1{
            seoj: Eoj {
                class_group_code: 0x05,
                class_code: 0xff,
                instance_code: 0x01,
            },
            deoj: Eoj {
                class_group_code: 0x02,
                class_code: 0x88,
                instance_code: 0x01,
            },
            esv: 0x62,
            opc: 0x01,
            props: vec![EDataProperty {
                epc: 0xe7,
                pdc: 0x00,
                edt: Bytes::from_static(b""),
            }],
        }));
    }

    #[test]
    fn test_parse_erxudp() {
        let (rest, response) = parser(&b"ERXUDP FE80:0000:0000:0000:0123:4567:89ab:cdef FE80:0000:0000:0000:3210:7654:ba98:fedc 0E1A 0E1A 001D129012345678 1 0012 \x10\x81\0\x01\x02\x88\x01\x05\xff\x01r\x01\xe7\x04\0\0\x01\xa8\r\n"[..]).unwrap();

        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::ERxUdp {
            sender: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
            dest: "FE80:0000:0000:0000:3210:7654:ba98:fedc".to_string(),
            rport: 0xe1a,
            lport: 0xe1a,
            senderlla: "001D129012345678".to_string(),
            secured: 0x01,
            datalen: 0x012,
            data: EchonetLite {
                ehd: EHd {
                    ehd1: 0x10,
                    ehd2: 0x81,
                    tid: 0x0001,
                },
                edata: EData::EDataFormat1(EDataFormat1 {
                    seoj: Eoj {
                        class_group_code: 0x02,
                        class_code: 0x88,
                        instance_code: 0x01,
                    },
                    deoj: Eoj {
                        class_group_code: 0x05,
                        class_code: 0xff,
                        instance_code: 0x01,
                    },
                    esv: 0x72,
                    opc: 0x01,
                    props: vec![EDataProperty {
                        epc: 0xe7,
                        pdc: 0x04,
                        edt: Bytes::from_static(b"\0\0\x01\xa8"),
                    }],
                })
            }
        });

    }

    #[test]
    fn test_parse_erxudp_invalid_frame() {
        let (rest, response) = parser(&b"ERXUDP FE80:0000:0000:0000:0123:4567:89ab:cdef FE80:0000:0000:0000:3210:7654:ba98:fedc 02CC 02CC 001D129012345678 0 0028 \0\0\0(\xc0\0\0\x02\x06\x04S\x07\x8d\xd5a\xbf\0\x06\0\0\0\x04\0\0\0\0\0\x05\0\x03\0\0\0\x04\0\0\0\0\0\x0c\r\n"[..]).unwrap();

        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::ERxUdp {
            sender: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
            dest: "FE80:0000:0000:0000:3210:7654:ba98:fedc".to_string(),
            rport: 0x2cc,
            lport: 0x2cc,
            senderlla: "001D129012345678".to_string(),
            secured: 0x00,
            datalen: 0x028,
            data: EchonetLite {
                ehd: EHd {
                    ehd1: 0x00,
                    ehd2: 0x00,
                    tid: 0x0028,
                },
                edata: EData::InvalidEData(Bytes::from_static(b"\xc0\0\0\x02\x06\x04S\x07\x8d\xd5a\xbf\0\x06\0\0\0\x04\0\0\0\0\0\x05\0\x03\0\0\0\x04\0\0\0\0\0\x0c"))
            }
        });


        let (rest, response) = parser(&b"ERXUDP FE80:0000:0000:0000:0123:4567:89ab:cdef FE80:0000:0000:0000:3210:7654:ba98:fedc 02CC 02CC 001D129012345678 0 0058 \0\0\0X\xa0\0\0\x02\x06\x04S\x07\x8d\xd5a\xc2\0\x07\0\0\0\x04\0\0\0\0\0\0\0\x02\0\0\0\x04\0\0\x03\xb5\0\x04\0\x04\0\0\0\x04\0\0\0\0\x07\x01\0\x08\0\0\0\x04\0\0\0\x01Q\x80\0\x01\0\0\0\x10\0\0\x13v\x01$1\x1c\x90\xd3T\xb6p 83\xee\xe7\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::ERxUdp {
            sender: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
            dest: "FE80:0000:0000:0000:3210:7654:ba98:fedc".to_string(),
            rport: 0x2cc,
            lport: 0x2cc,
            senderlla: "001D129012345678".to_string(),
            secured: 0x00,
            datalen: 0x058,
            data: EchonetLite {
                ehd: EHd {
                    ehd1: 0x00,
                    ehd2: 0x00,
                    tid: 0x0058,
                },
                edata: EData::InvalidEData(Bytes::from_static(b"\xa0\0\0\x02\x06\x04S\x07\x8d\xd5a\xc2\0\x07\0\0\0\x04\0\0\0\0\0\0\0\x02\0\0\0\x04\0\0\x03\xb5\0\x04\0\x04\0\0\0\x04\0\0\0\0\x07\x01\0\x08\0\0\0\x04\0\0\0\x01Q\x80\0\x01\0\0\0\x10\0\0\x13v\x01$1\x1c\x90\xd3T\xb6p 83\xee\xe7"))
            }
        });
    }

    #[test]
    fn test_parse_sksendto() {
        let (rest, response) = parser(&b"SKSENDTO 1 FE80:0000:0000:0000:0123:4567:89ab:cdef 0E1A 1 000e \r\nEVENT 21 FE80:0000:0000:0000:0123:4567:89ab:cdef 00\r\nOK\r\n\r\n"[..]).unwrap();
        assert_eq!(rest, &b""[..]);
        assert_eq!(response, Response::SkSendTo {
            handle: 0x1,
            ipaddr: "FE80:0000:0000:0000:0123:4567:89ab:cdef".to_string(),
            port: 0xe1a,
            sec: 0x1,
            datalen: 0x0e,
            result: 0x00,
        });
    }
}