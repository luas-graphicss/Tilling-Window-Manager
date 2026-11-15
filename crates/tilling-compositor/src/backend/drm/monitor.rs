use std::ops::Range;

use smithay::backend::drm::DrmDevice;
use smithay::reexports::drm::control::{Device as _, connector, property};

const EDID_PROPERTY: &str = "EDID";
const HEADER: [u8; 8] = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];
const BASE_BLOCK_BYTES: usize = 128;
const MANUFACTURER: Range<usize> = 8..10;
const PRODUCT_CODE: Range<usize> = 10..12;
const SERIAL_NUMBER: Range<usize> = 12..16;
const DESCRIPTORS: Range<usize> = 54..126;
const DESCRIPTOR_BYTES: usize = 18;
const DISPLAY_DESCRIPTOR_PREFIX: [u8; 3] = [0; 3];
const DESCRIPTOR_TAG: usize = 3;
const DESCRIPTOR_TEXT: Range<usize> = 5..18;
const MONITOR_NAME_TAG: u8 = 0xfc;
const SERIAL_TEXT_TAG: u8 = 0xff;
const TEXT_TERMINATOR: u8 = 0x0a;
const LETTER_BITS: u16 = 0x1f;
const LETTER_SHIFTS: [u16; 3] = [10, 5, 0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Monitor {
    pub(super) make: String,
    pub(super) model: String,
    pub(super) serial: Option<String>,
}

impl Monitor {
    pub(super) fn read(drm: &DrmDevice, connector: connector::Handle) -> Option<Self> {
        Self::from_edid(&edid_of(drm, connector)?)
    }

    pub(super) fn from_edid(edid: &[u8]) -> Option<Self> {
        let base = edid.get(..BASE_BLOCK_BYTES)?;
        if base.get(..HEADER.len())? != HEADER {
            return None;
        }
        let make = manufacturer(u16::from_be_bytes(base.get(MANUFACTURER)?.try_into().ok()?))?;
        let product = u16::from_le_bytes(base.get(PRODUCT_CODE)?.try_into().ok()?);
        let serial_number = u32::from_le_bytes(base.get(SERIAL_NUMBER)?.try_into().ok()?);
        let mut name = None;
        let mut serial_text = None;
        for descriptor in base.get(DESCRIPTORS)?.as_chunks::<DESCRIPTOR_BYTES>().0 {
            match descriptor_text(descriptor) {
                Some((MONITOR_NAME_TAG, text)) => name = Some(text),
                Some((SERIAL_TEXT_TAG, text)) => serial_text = Some(text),
                Some(_) | None => {}
            }
        }
        let numbered = || (serial_number != 0).then(|| serial_number.to_string());
        Some(Self {
            make,
            model: name.unwrap_or_else(|| format!("{product:04X}")),
            serial: serial_text.or_else(numbered),
        })
    }

    pub(super) fn key(&self, connector: &str, has_twin: bool) -> String {
        match &self.serial {
            Some(serial) if !has_twin => format!("{}/{}/{serial}", self.make, self.model),
            Some(_) | None => format!("{}/{}@{connector}", self.make, self.model),
        }
    }
}

fn edid_of(drm: &DrmDevice, connector: connector::Handle) -> Option<Vec<u8>> {
    let properties = drm.get_properties(connector).ok()?;
    for (handle, raw) in properties.iter() {
        let Ok(info) = drm.get_property(*handle) else {
            continue;
        };
        if info.name().to_str() != Ok(EDID_PROPERTY) {
            continue;
        }
        let property::Value::Blob(blob) = info.value_type().convert_value(*raw) else {
            return None;
        };
        return drm.get_property_blob(blob).ok();
    }
    None
}

fn manufacturer(packed: u16) -> Option<String> {
    LETTER_SHIFTS
        .into_iter()
        .map(|shift| {
            let position = u8::try_from((packed >> shift) & LETTER_BITS).ok()?;
            (1..=26)
                .contains(&position)
                .then(|| char::from(b'A' + position - 1))
        })
        .collect()
}

fn descriptor_text(descriptor: &[u8]) -> Option<(u8, String)> {
    if descriptor.get(..DISPLAY_DESCRIPTOR_PREFIX.len())? != DISPLAY_DESCRIPTOR_PREFIX {
        return None;
    }
    let tag = *descriptor.get(DESCRIPTOR_TAG)?;
    let text: String = descriptor
        .get(DESCRIPTOR_TEXT)?
        .iter()
        .take_while(|byte| **byte != TEXT_TERMINATOR)
        .filter(|byte| byte.is_ascii_graphic() || **byte == b' ')
        .map(|byte| char::from(*byte))
        .collect();
    let text = text.trim();
    (!text.is_empty()).then(|| (tag, text.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEL: [u8; 2] = [0x10, 0xac];
    const FIRST_DESCRIPTOR: usize = 54;

    fn edid(product: u16, serial: u32, descriptors: &[(u8, &str)]) -> Vec<u8> {
        let mut block = vec![0_u8; BASE_BLOCK_BYTES];
        block[..HEADER.len()].copy_from_slice(&HEADER);
        block[MANUFACTURER].copy_from_slice(&DEL);
        block[PRODUCT_CODE].copy_from_slice(&product.to_le_bytes());
        block[SERIAL_NUMBER].copy_from_slice(&serial.to_le_bytes());
        for (index, (tag, text)) in descriptors.iter().enumerate() {
            let start = FIRST_DESCRIPTOR + index * DESCRIPTOR_BYTES;
            let padded: Vec<u8> = text
                .bytes()
                .chain([TEXT_TERMINATOR])
                .chain(std::iter::repeat(b' '))
                .take(DESCRIPTOR_TEXT.len())
                .collect();
            block[start + DESCRIPTOR_TAG] = *tag;
            block[start + DESCRIPTOR_TEXT.start..start + DESCRIPTOR_TEXT.end]
                .copy_from_slice(&padded);
        }
        block
    }

    #[test]
    fn name_and_serial_descriptors_identify_the_monitor() {
        let described = edid(
            0xa0d1,
            0,
            &[
                (MONITOR_NAME_TAG, "DELL U2720Q"),
                (SERIAL_TEXT_TAG, "7RGSN83"),
            ],
        );

        let monitor = Monitor::from_edid(&described).unwrap();

        assert_eq!(
            monitor,
            Monitor {
                make: "DEL".to_owned(),
                model: "DELL U2720Q".to_owned(),
                serial: Some("7RGSN83".to_owned())
            }
        );
        assert_eq!(monitor.key("DP-1", false), "DEL/DELL U2720Q/7RGSN83");
    }

    #[test]
    fn product_code_and_numeric_serial_stand_in_for_missing_descriptors() {
        let monitor = Monitor::from_edid(&edid(0xa0d1, 12_345, &[])).unwrap();

        assert_eq!(monitor.model, "A0D1");
        assert_eq!(monitor.serial.as_deref(), Some("12345"));
    }

    #[test]
    fn monitor_without_a_unique_serial_is_told_apart_by_its_connector() {
        let unnumbered = Monitor::from_edid(&edid(0xa0d1, 0, &[])).unwrap();
        let numbered = Monitor::from_edid(&edid(0xa0d1, 12_345, &[])).unwrap();

        assert_eq!(unnumbered.serial, None);
        assert_eq!(unnumbered.key("HDMI-A-1", false), "DEL/A0D1@HDMI-A-1");
        assert_eq!(numbered.key("DP-2", true), "DEL/A0D1@DP-2");
    }

    #[test]
    fn truncated_or_foreign_data_is_not_an_edid() {
        let mut wrong_header = edid(0xa0d1, 1, &[]);
        wrong_header[0] = 0x01;
        let mut unnamed_maker = edid(0xa0d1, 1, &[]);
        unnamed_maker[MANUFACTURER].copy_from_slice(&[0, 0]);

        assert_eq!(Monitor::from_edid(&[]), None);
        assert_eq!(Monitor::from_edid(&HEADER), None);
        assert_eq!(Monitor::from_edid(&wrong_header), None);
        assert_eq!(Monitor::from_edid(&unnamed_maker), None);
    }
}
