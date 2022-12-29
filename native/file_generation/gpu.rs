use anyhow::Result;
use hdf5::{types::TypeDescriptor::*, types::*, File, Group};
use std::{
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    data_handler::SensorDataHandler,
    types::{SensorList, SystemPtr},
};

pub fn initialize_gpu_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut gpu_sensors = SensorList::new();
    let gpu_group = file.create_group("GPU")?;

    ////////////////////
    // System time
    gpu_sensors.push(Box::new(SensorDataHandler::new(
        &gpu_group,
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
    // Generate gpu Sensor Handlers

    Ok((gpu_group, gpu_sensors))
}
