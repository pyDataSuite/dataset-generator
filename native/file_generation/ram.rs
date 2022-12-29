use anyhow::Result;
use hdf5::{types::TypeDescriptor::*, types::*, File, Group};
use std::{
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::{ SystemExt};

use crate::{
    data_handler::{ SensorDataHandler},
    types::{SensorList, SystemPtr},
};

pub fn initialize_ram_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut ram_sensors = SensorList::new();
    let ram_group = file.create_group("RAM")?;

    ////////////////////
    // System time
    ram_sensors.push(Box::new(SensorDataHandler::new(
        &ram_group,
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

    // ram_sensors.push(Box::new(MultiSensorDataHandler::new(
    //     &ram_group,
    //     "ram_usage",
    //     Unsigned(IntSize::U8),
    //     Rc::clone(&sys),
    //     |_| [0, 1, 3, 4],
    // )?));

    ////////////////////
    // Generate RAM Sensor Handlers
    ram_sensors.push(Box::new(SensorDataHandler::new(
        &ram_group,
        "total_memory",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |system| system.total_memory(),
    )?));

    ram_sensors.push(Box::new(SensorDataHandler::new(
        &ram_group,
        "used_memory",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |system| system.used_memory(),
    )?));

    ram_sensors.push(Box::new(SensorDataHandler::new(
        &ram_group,
        "total_swap",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |system| system.total_swap(),
    )?));

    ram_sensors.push(Box::new(SensorDataHandler::new(
        &ram_group,
        "used_swap",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |system| system.used_swap(),
    )?));

    Ok((ram_group, ram_sensors))
}
