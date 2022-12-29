use anyhow::{Context, Result};
use hdf5::{types::TypeDescriptor::*, types::*, File, Group};
use std::{
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::{DiskExt, SystemExt};

use crate::{
    data_handler::{MultiSensorDataHandler, SensorDataHandler},
    types::{SensorList, SystemPtr},
};

pub fn initialize_disk_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut disk_sensors = SensorList::new();
    let disk_group = file
        .create_group("DISK")
        .with_context(|| "Trying to create disk group?")?;

    ////////////////////
    // System time
    disk_sensors.push(Box::new(SensorDataHandler::new(
        &disk_group,
        "system_time",
        Unsigned(IntSize::U8),
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
    disk_sensors.push(Box::new(MultiSensorDataHandler::new(
        &disk_group,
        "grouped_available_space",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |system| {
            // Create array of up to 20 disks and store the usage
            let mut ret_array = [0_u64; 20];
            for (i, disk) in system.disks().iter().enumerate() {
                if i > 20 {
                    continue;
                };
                ret_array[i] = disk.available_space();
            }

            ret_array
        },
    )?));

    disk_sensors.push(Box::new(MultiSensorDataHandler::new(
        &disk_group,
        "grouped_total_space",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |system| {
            // Create array of up to 20 disks and store the usage
            let mut ret_array = [0_u64; 20];
            for (i, disk) in system.disks().iter().enumerate() {
                if i > 20 {
                    continue;
                };
                ret_array[i] = disk.total_space();
            }

            ret_array
        },
    )?));

    // Per-disk Stats
    for (i, disk) in sys.borrow().disks().iter().enumerate() {
        let mut disk_name = format!("{:?}", disk.name())
            .replace('/', "_")
            .replace('"', "");

        while disk_group.member_names()?[0].contains(&disk_name) {
            disk_name += "_2"
        }
        println!("Disk: {}", disk_name);

        disk_sensors.push(Box::new(SensorDataHandler::new(
            &disk_group,
            format!("{}_available_space", disk_name),
            Unsigned(IntSize::U8),
            Rc::clone(&sys),
            move |system| system.disks()[i].available_space(),
        )?));

        disk_sensors.push(Box::new(SensorDataHandler::new(
            &disk_group,
            format!("{}_total_space", disk_name),
            Unsigned(IntSize::U8),
            Rc::clone(&sys),
            move |system| system.disks()[i].total_space(),
        )?));

        disk_sensors.push(Box::new(SensorDataHandler::new(
            &disk_group,
            format!("{}_is_removable", disk_name),
            Boolean,
            Rc::clone(&sys),
            move |system| system.disks()[i].is_removable(),
        )?));
    }

    Ok((disk_group, disk_sensors))
}
