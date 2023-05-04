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

pub const EOJ_LOW_VOLTAGE_SMART_METER: Eoj = Eoj {
    class_group_code: 0x02,
    class_code: 0x88,
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

#[derive(Debug, PartialEq, Default, Clone)]
struct EDataType2 {
    pub data: Bytes,
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
