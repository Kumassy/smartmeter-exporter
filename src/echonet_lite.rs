use std::fmt;
use bytes::{Bytes, BytesMut, BufMut};


#[derive(Debug, PartialEq)]
pub struct EchonetLite {
    pub ehd: EHd,
    pub edata: EData,
}

#[derive(PartialEq, Default, Clone, Copy)]
pub struct EHd {
    pub ehd1: u8,
    pub ehd2: u8,
    pub tid: u16,
}

pub const EHD1_ECHONET_LITE: u8 = 0x10;
pub const EHD2_FORMAT1: u8 = 0x81;

#[derive(PartialEq, Default, Clone)]
pub struct EDataProperty {
    pub epc: u8,
    pub pdc: u8,
    pub edt: Bytes,
}

#[derive(PartialEq, Eq, Default, Clone, Copy)]
pub struct Eoj {
    pub class_group_code: u8,
    pub class_code: u8,
    pub instance_code: u8,
}

pub const EOJ_HOUSING_LOW_VOLTAGE_SMART_METER: Eoj = Eoj {
    class_group_code: 0x02,
    class_code: 0x88,
    instance_code: 0x01,
};
pub const EOJ_MANAGEMENT_CONTROLLER: Eoj = Eoj {
    class_group_code: 0x05,
    class_code: 0xFF,
    instance_code: 0x01,
};

#[derive(Debug, PartialEq, Clone, Copy)]
#[non_exhaustive]
pub struct EpcLowVoltageSmartMeter;
impl EpcLowVoltageSmartMeter {
    pub const STATUS: u8 = 0x80;
    pub const EFFECTIVE_DIGITS_OF_CUMULATIVE_ENERGY: u8 = 0xD7;
    pub const CUMULATIVE_ENERGY_NORMAL_DIRECTION: u8 = 0xE0;
    pub const CUMULATIVE_ENERGY_REVERSE_DIRECTION: u8 = 0xE3;
    pub const CUMULATIVE_ENERGY_UNIT: u8 = 0xE1;
    pub const INSTANTANEOUS_ENERGY: u8 = 0xE7;
    pub const INSTANTANEOUS_CURRENT: u8 = 0xE8;
    pub const CUMULATIVE_ENERGY_FIXED_TIME_NORMAL_DIRECTION: u8 = 0xEA;
    pub const CUMULATIVE_ENERGY_FIXED_TIME_REVERSE_DIRECTION: u8 = 0xEB;
}

#[derive(Debug, PartialEq, Clone)]
pub enum EData {
    EDataFormat1(EDataFormat1),
    InvalidEData(Bytes),
}

#[derive(PartialEq, Default, Clone)]
pub struct EDataFormat1 {
    pub seoj: Eoj, 
    pub deoj: Eoj, 
    pub esv: u8,
    pub opc: u8,
    pub props: Vec<EDataProperty>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[non_exhaustive]
pub struct Esv;
impl Esv {
    pub const PROP_WRITE_NO_RES: u8 = 0x60;
    pub const PROP_WRITE: u8 = 0x61;
    pub const PROP_READ: u8 = 0x62;
    pub const PROP_NOTIFY: u8 = 0x62;
    pub const PROP_WRITE_READ: u8 = 0x6E;
}

impl fmt::Debug for EHd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EHd")
         .field("ehd1", &format_args!("{:#x}", self.ehd1))
         .field("ehd2", &format_args!("{:#x}", self.ehd2))
         .field("tid", &format_args!("{:#x}", self.tid))
         .finish()
    }
}

impl fmt::Debug for Eoj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Eoj")
         .field("class_group_code", &format_args!("{:#x}", self.class_group_code))
         .field("class_code", &format_args!("{:#x}", self.class_code))
         .field("instance_code", &format_args!("{:#x}", self.instance_code))
         .finish()
    }
}

impl fmt::Debug for EDataProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EDataProperty")
         .field("epc", &format_args!("{:#x}", self.epc))
         .field("pdc", &format_args!("{:#x}", self.pdc))
         .field("edt", &self.edt)
         .finish()
    }
}

impl fmt::Debug for EDataFormat1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EDataType1")
         .field("seoj", &self.seoj)
         .field("deoj", &self.deoj)
         .field("esv", &format_args!("{:#x}", self.esv))
         .field("opc", &format_args!("{:#x}", self.opc))
         .field("props", &self.props)
         .finish()
    }
}


impl Into<Bytes> for EHd {
    fn into(self) -> Bytes {
        let mut bytes = BytesMut::new();
        bytes.put_u8(self.ehd1);
        bytes.put_u8(self.ehd2);
        bytes.put_u16(self.tid);
        bytes.freeze()
    }
}

impl Into<Bytes> for EDataProperty {
    fn into(self) -> Bytes {
        let mut bytes = BytesMut::new();
        bytes.put_u8(self.epc);
        bytes.put_u8(self.pdc);
        bytes.put(self.edt);
        bytes.freeze()
    }
}

impl Into<Bytes> for Eoj {
    fn into(self) -> Bytes {
        let mut bytes = BytesMut::new();
        bytes.put_u8(self.class_group_code);
        bytes.put_u8(self.class_code);
        bytes.put_u8(self.instance_code);
        bytes.freeze()
    }
}

impl Into<Bytes> for EDataFormat1 {
    fn into(self) -> Bytes {
        let mut bytes = BytesMut::new();

        bytes.put::<Bytes>(self.seoj.into());
        bytes.put::<Bytes>(self.deoj.into());
        bytes.put_u8(self.esv);
        bytes.put_u8(self.opc);
        for prop in self.props {
            bytes.put::<Bytes>(prop.into());
        }
        bytes.freeze()
    }
}

impl Into<Bytes> for EData {
    fn into(self) -> Bytes {
        match self {
            EData::EDataFormat1(data) => data.into(),
            EData::InvalidEData(data) => data,
        }
    }
}

impl Into<Bytes> for EchonetLite {
    fn into(self) -> Bytes {
        let mut bytes = BytesMut::new();
        bytes.put::<Bytes>(self.ehd.into());
        bytes.put::<Bytes>(self.edata.into());
        bytes.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echonet_lite_as_bytes() {
        let data = EchonetLite {
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
        
        let bytes: Bytes = data.into();

        assert_eq!(bytes, Bytes::from_static(b"\x10\x81\x00\x01\x05\xFF\x01\x02\x88\x01\x62\x01\xE7\x00"));
    }

}