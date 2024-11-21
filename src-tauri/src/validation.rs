// COMPATIBILITY

use std::path::Path;

use log::debug;

use crate::device::{ConnectedDevice, ConnectedDeviceType};

fn after<'value>(value: &'value str, a: &str) -> &'value str {
    // Find the string and return the part after.
    if let Some(pos_a) = value.rfind(a) {
        let adjusted_pos_a = pos_a + a.len();
        if adjusted_pos_a < value.len() {
            return &value[adjusted_pos_a..];
        }
    }
    ""
}

fn _is_compatible(device: &ConnectedDevice, file_name: &str) -> bool {
    let device_str = match &device.device_type {
        ConnectedDeviceType::Bridge6 => "bridge6",
        ConnectedDeviceType::Bridge4 => "bridge4",
        _ => "",
    };

    // get the version string from the file name - should be 7 characters long
    let version = after(file_name, format!("{device_str}_v").as_str())
        .chars()
        .take(7)
        .collect::<String>();

    // collect our device details and determine if the last character of the version matches our hardware revision
    match &device.device_details {
        Some(details) => match version.chars().last() {
            Some(last) => {
                let result = last.to_digit(10) == details.get_hardware_revision();
                debug!(
                    "{} is compatible: {result} - version: {version}",
                    &file_name
                );
                result
            }
            None => false,
        },
        None => false,
    }
}

fn _is_not_diag(device: &ConnectedDevice, file_name: &str) -> bool {
    let device_str = match &device.device_type {
        ConnectedDeviceType::Click => "click",
        ConnectedDeviceType::ULoop => "uloop",
        _ => "",
    };
    file_name.starts_with(format!("{device_str}_v").as_str())
}

pub fn is_name_compatible(device: &ConnectedDevice, file_name: &str, allow_diag: bool) -> bool {
    match &device.device_type {
        // assume format: bridgeX_v1.2.1.1.bin || device_v1.0.0.0.uf2
        // the last number in the version is the compatible revision
        ConnectedDeviceType::Bridge6 | ConnectedDeviceType::Bridge4 => {
            _is_compatible(device, file_name)
        }
        ConnectedDeviceType::Click | ConnectedDeviceType::ULoop => {
            if !allow_diag {
                _is_not_diag(device, file_name)
            } else {
                true
            }
        }
        ConnectedDeviceType::Unknown => false,
        _ => true, // assume it's true by default if we have a device type
    }
}

pub fn is_file_compatible(device: &ConnectedDevice, binary: &Path, allow_diag: bool) -> bool {
    let file_name = binary.file_name().unwrap().to_string_lossy();
    is_name_compatible(device, &file_name, allow_diag)
}

#[cfg(test)]
mod tests {
    use crate::{
        device::{ConnectedDevice, DeviceDetails},
        validation::is_name_compatible,
    };

    #[test]
    fn is_compatible() {
        let mut mock_devices: Vec<ConnectedDevice> = vec![];
        let mut mock_assets: Vec<String> = vec![];

        for i in 1..4 {
            mock_devices.push(ConnectedDevice {
                id: String::from("test"),
                releases: None,
                vendor_id: 0,
                product_id: 0,
                description: Some(String::from("Bridge 4")),
                serial_number: Some(String::from("test")),
                device_type: crate::device::ConnectedDeviceType::Bridge4,
                device_details: Some(DeviceDetails {
                    uid: String::from(""),
                    device_model: String::from(""),
                    firmware_version: String::from(""),
                    hardware_version: format!("v1.0.{i}"),
                    device_name: String::from(""),
                    profile_id: String::from(""),
                }),
            })
        }

        for i in 1..4 {
            mock_assets.push(format!("bridge4_v1.0.1.{i}.bin"))
        }

        println!("assets: {:?}", mock_assets);
        println!("devices: {:?}", mock_devices);

        // Asset: bridge4_v1.0.1.1.bin - HW: v1.0.1 - true
        assert_eq!(
            is_name_compatible(&mock_devices[0], &mock_assets[0], false),
            true
        );

        // Asset: bridge4_v1.0.1.1.bin - HW: v1.0.2 - false
        assert_eq!(
            is_name_compatible(&mock_devices[1], &mock_assets[0], false),
            false
        );

        // Asset: bridge4_v1.0.1.2.bin - HW: v1.0.2 - true
        assert_eq!(
            is_name_compatible(&mock_devices[1], &mock_assets[1], false),
            true
        );

        // Asset: bridge4_v1.0.1.2.bin - HW: v1.0.3 - false
        assert_eq!(
            is_name_compatible(&mock_devices[2], &mock_assets[1], false),
            false
        );

        // Asset: bridge4_v1.0.1.3.bin - HW: v1.0.3 - true
        assert_eq!(
            is_name_compatible(&mock_devices[2], &mock_assets[2], false),
            true
        );

        // Asset: bridge4_v1.0.1.2.bin - HW: v1.0.3 - false
        assert_eq!(
            is_name_compatible(&mock_devices[2], &mock_assets[1], false),
            false
        );

        // All Bridge 6 should fail - even with compatiable versions
        assert_eq!(
            is_name_compatible(&mock_devices[0], "bridge6", false),
            false
        );

        assert_eq!(
            is_name_compatible(&mock_devices[1], "bridge6", false),
            false
        );

        // assert_eq!(result, 4);
    }
}
