use arrform::{arrform, ArrForm};
use cgmath::Vector3;
use core::{any::type_name, fmt, marker::PhantomData};
use num::{FromPrimitive, ToPrimitive};

pub const NINTENDO_VENDOR_ID: u16 = 1406;

pub const JOYCON_L_BT: u16 = 0x2006;
pub const JOYCON_R_BT: u16 = 0x2007;
pub const PRO_CONTROLLER: u16 = 0x2009;
pub const JOYCON_CHARGING_GRIP: u16 = 0x200e;

pub const HID_IDS: &[u16] = &[
    JOYCON_L_BT,
    JOYCON_R_BT,
    PRO_CONTROLLER,
    JOYCON_CHARGING_GRIP,
];

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum InputReportId {
    Normal = 0x3F,
    StandardAndSubcmd = 0x21,
    MCUFwUpdate = 0x23,
    StandardFull = 0x30,
    StandardFullMCU = 0x31,
    // 0x32 not used
    // 0x33 not used
}

// All unused values are a Nop
#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum SubcommandId {
    GetOnlyControllerState = 0x00,
    BluetoothManualPairing = 0x01,
    RequestDeviceInfo = 0x02,
    SetInputReportMode = 0x03,
    GetTriggerButtonsElapsedTime = 0x04,
    SetShipmentMode = 0x08,
    SPIRead = 0x10,
    SPIWrite = 0x11,
    SetMCUConf = 0x21,
    SetMCUState = 0x22,
    SetUnknownData = 0x24,
    SetPlayerLights = 0x30,
    SetHomeLight = 0x38,
    SetIMUMode = 0x40,
    SetIMUSens = 0x41,
    EnableVibration = 0x48,

    // arg [4,0,0,2], ret [0,8,0,0,0,0,0,44]
    // arg [4,4,5,2], ret [0,8,0,0,0,0,200]
    // arg [4,4,50,2], ret [0,8,0,0,0,0,5,0,0,14]
    // arg [4,4,10,2], ret [0,20,0,0,0,0,244,22,0,0,230,5,0,0,243,11,0,0,234,12, 0, 0]
    // get ringcon calibration: arg [4,4,26,2]
    //                          ret [0,20,0,0,0,0] + [135, 8, 28, 0, 48, 247, 243, 0, 44, 12, 224]
    // write ringcon calibration: arg [20,4,26,1,16] + [135, 8, 28, 0, 48, 247, 243, 0, 44, 12, 224]
    //                            ret [0, 4]
    // get number steps offline ringcon: arg [4,4,49,2], ret [0,8,0,0,0,0,nb_steps, 0,0, 127|143]
    // reset number steps offline ringcon: arg [8,4,49,1,4], ret [0,4]
    // Possibly accessory interaction like ringcon
    MaybeAccessory = 0x58,
    // Always [] arg, [0, 32] return
    Unknown0x59 = 0x59,
    // Always [4, 1, 1, 2] arg, [] return
    Unknown0x5a = 0x5a,
    // Always [] arg, [] return
    Unknown0x5b = 0x5b,
    // Always variable arg, [] return
    Unknown0x5c = 0x5c,
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct U16LE([u8; 2]);

impl From<u16> for U16LE {
    fn from(u: u16) -> Self {
        U16LE(u.to_le_bytes())
    }
}

impl From<U16LE> for u16 {
    fn from(u: U16LE) -> u16 {
        u16::from_le_bytes(u.0)
    }
}

impl fmt::Debug for U16LE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("0x{:x}", u16::from(*self)))
    }
}

impl fmt::Display for U16LE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        u16::from(*self).fmt(f)
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct I16LE(pub [u8; 2]);

impl From<i16> for I16LE {
    fn from(u: i16) -> I16LE {
        I16LE(u.to_le_bytes())
    }
}

impl From<I16LE> for i16 {
    fn from(u: I16LE) -> i16 {
        i16::from_le_bytes(u.0)
    }
}

impl fmt::Debug for I16LE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("0x{:x}", i16::from(*self)))
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct U32LE([u8; 4]);

impl From<u32> for U32LE {
    fn from(u: u32) -> Self {
        U32LE(u.to_le_bytes())
    }
}

impl From<U32LE> for u32 {
    fn from(u: U32LE) -> u32 {
        u32::from_le_bytes(u.0)
    }
}

impl fmt::Debug for U32LE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("0x{:x}", u32::from(*self)))
    }
}

#[cfg(test)]
pub(crate) fn offset_of<A, B>(a: &A, b: &B) -> usize {
    b as *const _ as usize - a as *const _ as usize
}

pub fn vector_from_raw(raw: [I16LE; 3]) -> Vector3<f64> {
    Vector3::new(
        i16::from(raw[0]) as f64,
        i16::from(raw[1]) as f64,
        i16::from(raw[2]) as f64,
    )
}

pub fn raw_from_vector(v: Vector3<f64>) -> [I16LE; 3] {
    [
        (v.x as i16).into(),
        (v.y as i16).into(),
        (v.z as i16).into(),
    ]
}

#[repr(transparent)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct RawId<Id>(u8, PhantomData<Id>);

impl<Id> RawId<Id> {
    pub fn new(id: u8) -> Self {
        RawId(id, PhantomData)
    }
}

impl<Id: FromPrimitive> RawId<Id> {
    pub fn try_into(self) -> Option<Id> {
        Id::from_u8(self.0)
    }
}

impl<Id: ToPrimitive> From<Id> for RawId<Id> {
    fn from(id: Id) -> Self {
        RawId(id.to_u8().expect("always one byte"), PhantomData)
    }
}

impl<Id: fmt::Debug + FromPrimitive + Copy> fmt::Debug for RawId<Id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(Some(id)) = self.try_into() {
            write!(f, "{:?}", id)
        } else {
            f.debug_tuple(arrform!(14, ":RawId<{}>", type_name::<Id>()).as_str())
                .field(&arrform!(8, "0x{:x}", self.0).as_str())
                .finish()
        }
    }
}

impl<Id: fmt::Display + FromPrimitive + Copy> fmt::Display for RawId<Id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(Some(id)) = self.try_into() {
            write!(f, "{}", id)
        } else {
            f.debug_tuple("RawId")
                .field(&arrform!(8, "0x{:x}", self.0).as_str())
                .finish()
        }
    }
}

impl<Id: FromPrimitive + PartialEq + Copy> PartialEq<Id> for RawId<Id> {
    fn eq(&self, other: &Id) -> bool {
        Id::from_u8(self.0).expect("Could be be represented as a u8") == *other
    }
}

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
pub enum Bool {
    False = 0,
    True = 1,
}

impl From<bool> for Bool {
    fn from(b: bool) -> Self {
        match b {
            false => Bool::False,
            true => Bool::True,
        }
    }
}
