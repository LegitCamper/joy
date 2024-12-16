use crate::{common::*, input::UseSPIColors};
// use cgmath::{vec2, Vector2, Vector3};
use core::{convert::TryFrom, fmt, num::ParseIntError, str::FromStr};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SPIRange(u32, u8);

impl SPIRange {
    pub unsafe fn new(offset: u32, size: u8) -> SPIRange {
        assert!(size <= 0x1D);
        SPIRange(offset, size)
    }

    pub fn offset(&self) -> u32 {
        self.0
    }

    pub fn size(&self) -> u8 {
        self.1
    }
}

const RANGE_FACTORY_CALIBRATION_SENSORS: SPIRange = SPIRange(0x6020, 0x18);
const RANGE_FACTORY_CALIBRATION_STICKS: SPIRange = SPIRange(0x603D, 0x12);
const RANGE_USER_CALIBRATION_STICKS: SPIRange = SPIRange(0x8010, 0x16);
const RANGE_USER_CALIBRATION_SENSORS: SPIRange = SPIRange(0x8026, 0x1A);

pub(crate) const RANGE_CONTROLLER_COLOR_USE_SPI: SPIRange = SPIRange(0x601B, 1);
pub(crate) const RANGE_CONTROLLER_COLOR: SPIRange = SPIRange(0x6050, 12);

pub trait SPI: TryFrom<SPIReadResult, Error = WrongRangeError> {
    fn range() -> SPIRange;
}

#[derive(Debug, Clone, Copy)]
pub struct WrongRangeError {
    expected: SPIRange,
    got: SPIRange,
}

impl fmt::Display for WrongRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "wrong SPI range: expected {:?}, got {:?}",
            self.expected, self.got
        )
    }
}

impl core::error::Error for WrongRangeError {}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct SPIReadRequest {
    offset: U32LE,
    size: u8,
}

impl SPIReadRequest {
    pub fn new(range: SPIRange) -> SPIReadRequest {
        assert!(range.1 <= 0x1d);
        SPIReadRequest {
            offset: range.0.into(),
            size: range.1,
        }
    }

    pub fn range(&self) -> SPIRange {
        SPIRange(self.offset.into(), self.size)
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct SPIWriteRequest {
    address: U32LE,
    size: u8,
    data: SPIData,
}

impl SPIWriteRequest {
    pub unsafe fn new(range: SPIRange, data: &[u8]) -> SPIWriteRequest {
        assert_eq!(range.1 as usize, data.len());
        let mut raw = [0; 0x1D];
        raw[..range.1 as usize].copy_from_slice(data);
        SPIWriteRequest {
            address: range.0.into(),
            size: range.1,
            data: SPIData { raw },
        }
    }

    pub fn range(&self) -> SPIRange {
        SPIRange(self.address.into(), self.size)
    }
}

impl From<ControllerColor> for SPIWriteRequest {
    fn from(color: ControllerColor) -> SPIWriteRequest {
        let range = ControllerColor::range();
        assert!(range.1 <= 0x1d);
        SPIWriteRequest {
            address: range.0.into(),
            size: range.1,
            data: SPIData { color },
        }
    }
}

impl SPI for UseSPIColors {
    fn range() -> SPIRange {
        RANGE_CONTROLLER_COLOR_USE_SPI
    }
}

impl From<UseSPIColors> for SPIWriteRequest {
    fn from(use_spi_colors: UseSPIColors) -> SPIWriteRequest {
        let range = UseSPIColors::range();
        assert!(range.1 <= 0x1d);
        SPIWriteRequest {
            address: range.0.into(),
            size: range.1,
            data: SPIData {
                use_spi_colors: use_spi_colors.into(),
            },
        }
    }
}

impl TryFrom<SPIReadResult> for UseSPIColors {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.use_spi_colors.try_into().unwrap() })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}

impl fmt::Debug for SPIWriteRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = f.debug_struct("SPIWriteRequest");
        dbg_spi_data(&mut out, self.address, self.size, &self.data);
        out.finish()
    }
}

fn dbg_spi_data(out: &mut fmt::DebugStruct, address: U32LE, size: u8, data: &SPIData) {
    unsafe {
        let raw = &&data.raw[..size as usize];
        match (u32::from(address), size) {
            (0x6000, 16) => out.field("serial", raw),
            (0x603d, 25) => out.field("stick_factory", &data.sticks_factory_calib),
            (0x6050, 13) => out.field("color", &data.color),
            (0x6080, 24) => out
                .field("horizontal_offset", &&raw[..6])
                .field("stick_parameter1", &&raw[6..]),
            (0x6098, 18) => out.field("stick_parameter2", raw),
            (0x8010, 24) => out.field("stick_user", &data.sticks_user_calib),
            (0x8028, 24) => out.field("imu_user", &data.imu_factory_calib),
            _ => out
                .field("address", &address)
                .field("size", &size)
                .field("raw", raw),
        };
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct SPIReadResult {
    pub(crate) address: U32LE,
    pub(crate) size: u8,
    pub(crate) data: SPIData,
}

impl SPIReadResult {
    pub fn range(&self) -> SPIRange {
        SPIRange(self.address.into(), self.size)
    }

    pub fn raw(&self) -> [u8; 0x1D] {
        unsafe { self.data.raw }
    }
}

impl fmt::Debug for SPIReadResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = f.debug_struct("SPIReadResult");
        dbg_spi_data(&mut out, self.address, self.size, &self.data);
        out.finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct SPIWriteResult {
    status: u8,
}

impl SPIWriteResult {
    /// success is 0
    pub fn new(status: u8) -> Self {
        Self { status }
    }
    pub fn success(&self) -> bool {
        self.status == 0
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub(crate) union SPIData {
    pub(crate) sticks_factory_calib: SticksCalibration,
    pub(crate) sticks_user_calib: UserSticksCalibration,
    pub(crate) imu_factory_calib: SensorCalibration,
    pub(crate) imu_user_calib: UserSensorCalibration,
    pub(crate) color: ControllerColor,
    pub(crate) use_spi_colors: RawId<UseSPIColors>,
    pub(crate) raw: [u8; 0x1D],
}

fn cord_packing(x: u16, y: u16) -> [u8; 3] {
    let mut stick_cal = [0u8; 3];

    stick_cal[0] = (x & 0x00FF) as u8;
    stick_cal[1] = ((x >> 8) & 0x000F) as u8 | ((y & 0x000F) << 4) as u8;
    stick_cal[2] = (y >> 4) as u8;

    stick_cal
}

#[repr(packed)]
#[derive(Copy, Clone, Default, Debug)]
pub struct SticksCalibration {
    pub left: LeftStickCalibration,
    pub right: RightStickCalibration,
}

impl SPI for SticksCalibration {
    fn range() -> SPIRange {
        RANGE_FACTORY_CALIBRATION_STICKS
    }
}

impl Into<SPIReadResult> for SticksCalibration {
    fn into(self) -> SPIReadResult {
        SPIReadResult {
            address: RANGE_FACTORY_CALIBRATION_STICKS.offset().into(),
            size: RANGE_FACTORY_CALIBRATION_STICKS.size(),
            data: SPIData {
                sticks_factory_calib: self,
            },
        }
    }
}

impl TryFrom<SPIReadResult> for SticksCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.sticks_factory_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct UserSticksCalibration {
    pub left: LeftUserStickCalibration,
    pub right: RightUserStickCalibration,
}

impl SPI for UserSticksCalibration {
    fn range() -> SPIRange {
        RANGE_USER_CALIBRATION_STICKS
    }
}

impl Into<SPIReadResult> for UserSticksCalibration {
    fn into(self) -> SPIReadResult {
        SPIReadResult {
            address: RANGE_USER_CALIBRATION_STICKS.offset().into(),
            size: RANGE_USER_CALIBRATION_STICKS.size(),
            data: SPIData {
                sticks_user_calib: self,
            },
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct LeftStickCalibration {
    max: [u8; 3],
    center: [u8; 3],
    min: [u8; 3],
}

impl Default for LeftStickCalibration {
    fn default() -> Self {
        LeftStickCalibration {
            max: cord_packing(0x510, 0x479),
            center: cord_packing(0x79F, 0x8A0),
            min: cord_packing(0x4F7, 0x424),
        }
    }
}

impl LeftStickCalibration {
    fn conv_x(&self, raw: [u8; 3]) -> u16 {
        (((raw[1] as u16) << 8) & 0xF00) | raw[0] as u16
    }

    fn conv_y(&self, raw: [u8; 3]) -> u16 {
        ((raw[2] as u16) << 4) | (raw[1] >> 4) as u16
    }

    pub fn max(&self) -> (u16, u16) {
        let center = self.center();
        (
            (center.0 + self.conv_x(self.max)).min(0xFFF),
            (center.1 + self.conv_y(self.max)).min(0xFFF),
        )
    }

    pub fn center(&self) -> (u16, u16) {
        (self.conv_x(self.center), self.conv_y(self.center))
    }

    pub fn min(&self) -> (u16, u16) {
        let center = self.center();
        (
            center.0.saturating_sub(self.conv_x(self.min)),
            center.1.saturating_sub(self.conv_y(self.min)),
        )
    }

    // pub fn value_from_raw(&self, x: u16, y: u16) -> Vector2<f64> {
    //     let min = self.min();
    //     let center = self.center();
    //     let max = self.max();
    //     let rel_x = x.max(min.0).min(max.0) as f64 - center.0 as f64;
    //     let rel_y = y.max(min.1).min(max.1) as f64 - center.1 as f64;

    //     vec2(
    //         if rel_x >= 0. {
    //             rel_x / (max.0 as f64 - center.0 as f64)
    //         } else {
    //             rel_x / (center.0 as f64 - min.0 as f64)
    //         },
    //         if rel_y >= 0. {
    //             rel_y / (max.1 as f64 - center.1 as f64)
    //         } else {
    //             rel_y / (center.1 as f64 - min.1 as f64)
    //         },
    //     )
    // }
}

impl fmt::Debug for LeftStickCalibration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StickCalibration")
            .field("min", &self.min())
            .field("center", &self.center())
            .field("max", &self.max())
            .finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct RightStickCalibration {
    // x, y
    center: [u8; 3],
    min: [u8; 3],
    max: [u8; 3],
}

impl Default for RightStickCalibration {
    fn default() -> Self {
        RightStickCalibration {
            center: cord_packing(0x79F, 0x8A0),
            min: cord_packing(0x4F7, 0x424),
            max: cord_packing(0x510, 0x479),
        }
    }
}

impl RightStickCalibration {
    fn conv_x(&self, raw: [u8; 3]) -> u16 {
        (((raw[1] as u16) << 8) & 0xF00) | raw[0] as u16
    }

    fn conv_y(&self, raw: [u8; 3]) -> u16 {
        ((raw[2] as u16) << 4) | (raw[1] >> 4) as u16
    }

    pub fn max(&self) -> (u16, u16) {
        let center = self.center();
        (
            (center.0 + self.conv_x(self.max)).min(0xFFF),
            (center.1 + self.conv_y(self.max)).min(0xFFF),
        )
    }

    pub fn center(&self) -> (u16, u16) {
        (self.conv_x(self.center), self.conv_y(self.center))
    }

    pub fn min(&self) -> (u16, u16) {
        let center = self.center();
        (
            center.0.saturating_sub(self.conv_x(self.min)),
            center.1.saturating_sub(self.conv_y(self.min)),
        )
    }

    // pub fn value_from_raw(&self, x: u16, y: u16) -> Vector2<f64> {
    //     let min = self.min();
    //     let center = self.center();
    //     let max = self.max();
    //     let rel_x = x.max(min.0).min(max.0) as f64 - center.0 as f64;
    //     let rel_y = y.max(min.1).min(max.1) as f64 - center.1 as f64;

    //     vec2(
    //         if rel_x >= 0. {
    //             rel_x / (max.0 as f64 - center.0 as f64)
    //         } else {
    //             rel_x / (center.0 as f64 - min.0 as f64)
    //         },
    //         if rel_y >= 0. {
    //             rel_y / (max.1 as f64 - center.1 as f64)
    //         } else {
    //             rel_y / (center.1 as f64 - min.1 as f64)
    //         },
    //     )
    // }
}

impl TryFrom<SPIReadResult> for UserSticksCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.sticks_user_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}

impl fmt::Debug for RightStickCalibration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StickCalibration")
            .field("min", &self.min())
            .field("center", &self.center())
            .field("max", &self.max())
            .finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct LeftUserStickCalibration {
    magic: [u8; 2],
    calib: LeftStickCalibration,
}

impl Default for LeftUserStickCalibration {
    fn default() -> Self {
        Self {
            magic: USER_CALIB_MAGIC,
            calib: LeftStickCalibration::default(),
        }
    }
}

impl LeftUserStickCalibration {
    pub fn set_magic(&mut self, magic: bool) {
        match magic {
            true => self.magic = USER_CALIB_MAGIC,
            false => self.magic = USER_NO_CALIB_MAGIC,
        }
    }

    pub fn set_calib(&mut self, calib: LeftStickCalibration) {
        self.calib = calib;
    }

    pub fn calib(&self) -> Option<LeftStickCalibration> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib)
        } else {
            None
        }
    }

    pub fn max(&self) -> Option<(u16, u16)> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.max())
        } else {
            None
        }
    }

    pub fn center(&self) -> Option<(u16, u16)> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.center())
        } else {
            None
        }
    }

    pub fn min(&self) -> Option<(u16, u16)> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib.min())
        } else {
            None
        }
    }
}

impl fmt::Debug for LeftUserStickCalibration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.magic == USER_CALIB_MAGIC {
            f.write_fmt(format_args!("{:?}", self.calib))
        } else {
            f.write_str("NoUserStickCalibration")
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct RightUserStickCalibration {
    magic: [u8; 2],
    calib: RightStickCalibration,
}

impl Default for RightUserStickCalibration {
    fn default() -> Self {
        Self {
            magic: USER_CALIB_MAGIC,
            calib: RightStickCalibration::default(),
        }
    }
}

impl RightUserStickCalibration {
    pub fn set_calib(&mut self, calib: RightStickCalibration) {
        self.magic = USER_CALIB_MAGIC;
        self.calib = calib;
    }

    pub fn calib(&self) -> Option<RightStickCalibration> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib)
        } else {
            None
        }
    }

    // pub fn max(&self) -> Option<(u16, u16)> {
    //     if self.magic == USER_CALIB_MAGIC {
    //         Some(self.calib.max())
    //     } else {
    //         None
    //     }
    // }

    // pub fn center(&self) -> Option<(u16, u16)> {
    //     if self.magic == USER_CALIB_MAGIC {
    //         Some(self.calib.center())
    //     } else {
    //         None
    //     }
    // }

    // pub fn min(&self) -> Option<(u16, u16)> {
    //     if self.magic == USER_CALIB_MAGIC {
    //         Some(self.calib.min())
    //     } else {
    //         None
    //     }
    // }
}

impl fmt::Debug for RightUserStickCalibration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.magic == USER_CALIB_MAGIC {
            f.write_fmt(format_args!("{:?}", self.calib))
        } else {
            f.write_str("NoUserStickCalibration")
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct SensorCalibration {
    acc_orig: [I16LE; 3],
    acc_sens: [I16LE; 3],
    gyro_orig: [I16LE; 3],
    gyro_sens: [I16LE; 3],
}

impl Default for SensorCalibration {
    fn default() -> Self {
        SensorCalibration {
            acc_orig: [
                (-2 * 1024).into(), // FFB0
                (-1 * 1024).into(), // FEB9
                (0 * 1024).into(),  // 00E0
            ],
            acc_sens: [4000.into(); 3], // Default sensitivity: ±8G.
            gyro_orig: [
                14 * 128,  // 000E
                253 * 128, // FFDF
                224 * 128, // FFD0
            ]
            .map(|x| I16LE::from(x)),
            gyro_sens: [5173.into(); 3], // Default sensitivity: ±2000dps.
        }
    }
}

impl SensorCalibration {
    pub fn reset() -> SensorCalibration {
        let zero = [I16LE([0; 2]); 3];
        SensorCalibration {
            acc_orig: zero,
            acc_sens: zero,
            gyro_orig: zero,
            gyro_sens: zero,
        }
    }

    // pub fn acc_offset(&self) -> Vector3<f64> {
    //     vector_from_raw(self.acc_orig)
    // }

    // pub fn set_acc_offset(&mut self, offset: Vector3<f64>) {
    //     self.acc_orig = raw_from_vector(offset);
    // }

    // pub fn acc_factor(&self) -> Vector3<f64> {
    //     vector_from_raw(self.acc_sens)
    // }

    // pub fn set_acc_factor(&mut self, factor: Vector3<f64>) {
    //     self.acc_sens = raw_from_vector(factor);
    // }

    // pub fn gyro_offset(&self) -> Vector3<f64> {
    //     vector_from_raw(self.gyro_orig)
    // }

    // pub fn set_gyro_offset(&mut self, offset: Vector3<f64>) {
    //     self.gyro_orig = raw_from_vector(offset);
    // }

    // pub fn gyro_factor(&self) -> Vector3<f64> {
    //     vector_from_raw(self.gyro_sens)
    // }

    // pub fn set_gyro_factor(&mut self, factor: Vector3<f64>) {
    //     self.gyro_sens = raw_from_vector(factor);
    // }
}

impl SPI for SensorCalibration {
    fn range() -> SPIRange {
        RANGE_FACTORY_CALIBRATION_SENSORS
    }
}

impl Into<SPIReadResult> for SensorCalibration {
    fn into(self) -> SPIReadResult {
        SPIReadResult {
            address: RANGE_FACTORY_CALIBRATION_SENSORS.offset().into(),
            size: RANGE_FACTORY_CALIBRATION_SENSORS.size(),
            data: SPIData {
                imu_factory_calib: self,
            },
        }
    }
}

impl TryFrom<SPIReadResult> for SensorCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.imu_factory_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}

const USER_CALIB_MAGIC: [u8; 2] = [0xB2, 0xA1];
const USER_NO_CALIB_MAGIC: [u8; 2] = [0xFF; 2];

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct UserSensorCalibration {
    magic: [u8; 2],
    calib: SensorCalibration,
}

impl Default for UserSensorCalibration {
    fn default() -> Self {
        UserSensorCalibration {
            magic: USER_CALIB_MAGIC,
            calib: SensorCalibration::default(),
        }
    }
}

impl UserSensorCalibration {
    pub fn reset() -> UserSensorCalibration {
        UserSensorCalibration {
            magic: USER_NO_CALIB_MAGIC,
            calib: SensorCalibration::reset(),
        }
    }
}

impl SPI for UserSensorCalibration {
    fn range() -> SPIRange {
        RANGE_USER_CALIBRATION_SENSORS
    }
}

impl From<SensorCalibration> for UserSensorCalibration {
    fn from(calib: SensorCalibration) -> Self {
        UserSensorCalibration {
            magic: USER_CALIB_MAGIC,
            calib,
        }
    }
}

impl From<UserSensorCalibration> for SPIWriteRequest {
    fn from(calib: UserSensorCalibration) -> Self {
        let range = UserSensorCalibration::range();
        SPIWriteRequest {
            address: range.0.into(),
            size: range.1,
            data: SPIData {
                imu_user_calib: calib,
            },
        }
    }
}

impl Into<SPIReadResult> for UserSensorCalibration {
    fn into(self) -> SPIReadResult {
        SPIReadResult {
            address: RANGE_USER_CALIBRATION_SENSORS.offset().into(),
            size: RANGE_USER_CALIBRATION_SENSORS.size(),
            data: SPIData {
                imu_user_calib: self,
            },
        }
    }
}

impl TryFrom<SPIReadResult> for UserSensorCalibration {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.imu_user_calib })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}

impl UserSensorCalibration {
    pub fn calib(&self) -> Option<SensorCalibration> {
        if self.magic == USER_CALIB_MAGIC {
            Some(self.calib)
        } else {
            None
        }
    }
    // pub fn acc_offset(&self) -> Option<Vector3<f64>> {
    //     if self.magic == USER_CALIB_MAGIC {
    //         Some(self.calib.acc_offset())
    //     } else {
    //         None
    //     }
    // }

    // pub fn acc_factor(&self) -> Option<Vector3<f64>> {
    //     if self.magic == USER_CALIB_MAGIC {
    //         Some(self.calib.acc_factor())
    //     } else {
    //         None
    //     }
    // }

    // pub fn gyro_offset(&self) -> Option<Vector3<f64>> {
    //     if self.magic == USER_CALIB_MAGIC {
    //         Some(self.calib.gyro_offset())
    //     } else {
    //         None
    //     }
    // }

    // pub fn gyro_factor(&self) -> Option<Vector3<f64>> {
    //     if self.magic == USER_CALIB_MAGIC {
    //         Some(self.calib.gyro_factor())
    //     } else {
    //         None
    //     }
    // }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Color(u8, u8, u8);

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl FromStr for Color {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: do better
        assert!(s.len() == 6);
        Ok(Color(
            u8::from_str_radix(s.get(0..2).unwrap(), 16)?,
            u8::from_str_radix(s.get(2..4).unwrap(), 16)?,
            u8::from_str_radix(s.get(4..6).unwrap(), 16)?,
        ))
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct ControllerColor {
    pub body: Color,
    pub buttons: Color,
    pub left_grip: Color,
    pub right_grip: Color,
}

impl SPI for ControllerColor {
    fn range() -> SPIRange {
        RANGE_CONTROLLER_COLOR
    }
}

impl Into<SPIReadResult> for ControllerColor {
    fn into(self) -> SPIReadResult {
        SPIReadResult {
            address: RANGE_CONTROLLER_COLOR.offset().into(),
            size: RANGE_CONTROLLER_COLOR.size(),
            data: SPIData { color: self },
        }
    }
}

impl TryFrom<SPIReadResult> for ControllerColor {
    type Error = WrongRangeError;

    fn try_from(value: SPIReadResult) -> Result<Self, Self::Error> {
        if value.range() == Self::range() {
            Ok(unsafe { value.data.color })
        } else {
            Err(WrongRangeError {
                expected: Self::range(),
                got: value.range(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::println;

    #[test]
    fn left_calibration() {
        let calib = LeftStickCalibration::default();

        // check if calibration values are plausible
        assert!(calib.min().0 < calib.center().0);
        assert!(calib.center().0 < calib.max().0);
        assert!(calib.min().1 < calib.center().1);
        assert!(calib.center().1 < calib.max().1);
    }

    #[test]
    fn right_calibration() {
        let calib = RightStickCalibration::default();

        // check if calibration values are plausible
        assert!(calib.min().0 < calib.center().0);
        assert!(calib.center().0 < calib.max().0);
        assert!(calib.min().1 < calib.center().1);
        assert!(calib.center().1 < calib.max().1);
    }
}
