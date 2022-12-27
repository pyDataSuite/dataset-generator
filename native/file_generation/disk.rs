use anyhow::Result;
use hdf5::{types::TypeDescriptor::*, types::*, File, Group};
use std::{
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::SystemExt;

use crate::types::{SensorList, SystemPtr};

pub fn initialize_disk_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut disk_sensors = SensorList::new();
    let disk_group = file.create_group("DISK")?;
    // let num_disks = sys.borrow().disks().len();

    ////////////////////
    // System time
    disk_sensors.push(Box::new(SensorDataHandler::new(
        &disk_group,
        "system_time",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        },
    )?));

    ////////////////////
    // Grouped Disk Info
    // disk_sensors.push(Box::new(SensorDataHandler::new(
    //     &disk_group,
    //     "grouped_available_space",
    //     Unsigned(IntSize::U8),
    //     num_disks,
    //     Rc::clone(&sys),
    //     |data, system, index| -> Result<()> {
    //         let disk_availabilities: Vec<_> = system
    //             .disks()
    //             .iter()
    //             .map(|disk| disk.available_space())
    //             .collect();
    //         data.write_slice(&disk_availabilities, s![.., index])?;
    //         Ok(())
    //     },
    // )?));

    // disk_sensors.push(Box::new(SensorDataHandler::new(
    //     &disk_group,
    //     "grouped_total_space",
    //     Unsigned(IntSize::U8),
    //     num_disks,
    //     Rc::clone(&sys),
    //     |data, system, index| -> Result<()> {
    //         let disk_space: Vec<_> = system
    //             .disks()
    //             .iter()
    //             .map(|disk| disk.total_space())
    //             .collect();
    //         data.write_slice(&disk_space, s![.., index])?;
    //         Ok(())
    //     },
    // )?));

    // Per-disk Stats
    for (i, disk) in sys.borrow().disks().iter().enumerate() {
        let disk_name = format!("{:?}_available_space", disk.name()).replace("/", "_");

        disk_sensors.push(Box::new(SensorDataHandler::new(
            &disk_group,
            format!("{}_available_space", disk_name),
            Unsigned(IntSize::U8),
            0,
            Rc::clone(&sys),
            move |system| system.disks()[i].available_space(),
        )?));

        disk_sensors.push(Box::new(SensorDataHandler::new(
            &disk_group,
            format!("{}_total_space", disk_name),
            Unsigned(IntSize::U8),
            0,
            Rc::clone(&sys),
            move |system| system.disks()[i].total_space(),
        )?));

        disk_sensors.push(Box::new(SensorDataHandler::new(
            &disk_group,
            format!("{}_is_removable", disk_name),
            Boolean,
            0,
            Rc::clone(&sys),
            move |system| system.disks()[i].is_removable(),
        )?));
    }

    // // Add disk Metadata
    // disk_group
    //     .new_attr_builder()
    //     .with_data(sys.borrow().disks()[0].brand())
    //     .create("Brand")?;

    // disk_group
    //     .new_attr_builder()
    //     .with_data(sys.borrow().disks()[0].vendor_id())
    //     .create("VendorId")?;

    Ok((disk_group, disk_sensors))
}
