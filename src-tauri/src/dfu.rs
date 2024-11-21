use crate::{
    error::{self, Error},
    DFUSE_DEFAULT_ADDRESS, USB_BRIDGE_PRODUCT_DFU_ID, USB_BRIDGE_VENDOR_ID,
};
use dfu_nusb::DfuNusb;
use fs_extra::file::{copy_with_progress, CopyOptions, TransitProcess};
use log::{debug, error};
use std::{path::PathBuf, time::Duration};
use sysinfo::{DiskExt, RefreshKind, System, SystemExt};

pub fn install_rpi<F>(binary: PathBuf, progress_handler: F) -> Result<u64, error::Error>
where
    F: FnMut(TransitProcess),
{
    // sleep to allow disk to mount
    std::thread::sleep(Duration::from_secs(3));

    // get disk info from system
    let mut sys = System::new_with_specifics(RefreshKind::new().with_disks_list());

    // retrieve our disk info
    sys.refresh_disks_list();
    sys.refresh_disks();

    // brittle... but works
    let disks = sys.disks();
    debug!("available disks: {:?}", disks);

    let rpi_disk = disks
        .iter()
        .find(|&disk| disk.is_removable() && disk.name().eq_ignore_ascii_case("RPI-RP2"));

    match rpi_disk {
        Some(disk) => {
            let options = CopyOptions::new().buffer_size(512);
            let destination = disk
                .mount_point()
                .join(PathBuf::from(binary.file_name().unwrap()));

            // Copy binary file path to device
            match copy_with_progress(binary, destination, &options, progress_handler) {
                Ok(bytes_written) => Ok(bytes_written),
                Err(err) => err!(Error::IO(format!("upload failed with reason: {:?}", err))),
            }
        }
        None => err!(Error::Install("UF2 disk not available".to_string())),
    }
}

pub fn install_bridge<F>(binary: PathBuf, progress_handler: F) -> Result<(), error::Error>
where
    F: FnMut(usize) + 'static,
{
    // open the binary file
    let file = std::fs::File::open(binary)
        .map_err(|e| Error::IO(format!("could not open firmware file: {}", e)))?;

    let file_size = u32::try_from(file.metadata().unwrap().len())
        .map_err(|e| Error::IO(format!("firmware file is too large: {}", e)))?;

    let device = try_open(USB_BRIDGE_VENDOR_ID, USB_BRIDGE_PRODUCT_DFU_ID, 0, 0)
        .map_err(|e| Error::Usb(format!("unable to connect with device: {}", e)))?;

    // setup device with progress and default address
    let mut device = device.into_sync_dfu();
    let device = device
        .with_progress(progress_handler)
        .override_address(DFUSE_DEFAULT_ADDRESS);

    match device.download(file, file_size) {
        Ok(_) => (),
        Err(dfu_nusb::Error::Nusb(..)) => {
            error!("unable to download firmware to device");
            return Err(Error::Usb(
                "unable to download firmware to device".to_string(),
            ));
        }
        e => {
            return e
                .map_err(|err| Error::Usb(format!("could not write firmware to device: {}", err)))
        }
    }

    // detach and reset the usb device
    device
        .detach()
        .map_err(|e| Error::Usb(format!("unable to detach device: {}", e)))?;
    device
        .usb_reset()
        .map_err(|e| Error::Usb(format!("unable to reset device: {}", e)))?;
    Ok(())
}

fn try_open(vid: u16, pid: u16, int: u8, alt: u8) -> Result<DfuNusb, dfu_nusb::Error> {
    let info = nusb::list_devices()
        .unwrap()
        .find(|dev| dev.vendor_id() == vid && dev.product_id() == pid)
        .ok_or(dfu_nusb::Error::DeviceNotFound)?;
    let device = info.open()?;
    let interface = device.claim_interface(int)?;

    DfuNusb::open(device, interface, alt)
}
