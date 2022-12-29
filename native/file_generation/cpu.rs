use anyhow::Result;
use hdf5::{types::TypeDescriptor::*, types::*, File, Group};
use std::{
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::{CpuExt, SystemExt};

use crate::{
    data_handler::{MultiSensorDataHandler, SensorDataHandler},
    types::{SensorList, SystemPtr},
};

pub fn initialize_cpu_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut cpu_sensors = SensorList::new();
    let cpu_group = file.create_group("CPU")?;
    // let num_cpus = sys.borrow().cpus().len();

    ////////////////////
    // System time
    cpu_sensors.push(Box::new(SensorDataHandler::new(
        &cpu_group,
        "system_time",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |_| -> u64 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        },
    )?));

    ////////////////////
    // Grouped CPU Usage
    cpu_sensors.push(Box::new(MultiSensorDataHandler::new(
        &cpu_group,
        "grouped_cpu_usage",
        Float(FloatSize::U4),
        Rc::clone(&sys),
        |system| {
            // Create array of up to 20 cpus and store the usage
            let mut ret_array = [0.0_f32; 20];
            for (i, cpu) in system.cpus().iter().enumerate() {
                if i >= 20 {
                    continue;
                };
                ret_array[i] = cpu.cpu_usage();
            }

            ret_array
        },
    )?));

    // Grouped CPU Frequency
    cpu_sensors.push(Box::new(MultiSensorDataHandler::new(
        &cpu_group,
        "grouped_cpu_frequency",
        Unsigned(IntSize::U8),
        Rc::clone(&sys),
        |system| {
            // Create array of up to 20 cpus and store the usage
            let mut ret_array = [0u64; 20];
            for (i, cpu) in system.cpus().iter().enumerate() {
                if i >= 20 {
                    continue;
                };
                ret_array[i] = cpu.frequency();
            }

            ret_array
        },
    )?));

    // Per-CPU Stats
    for (i, cpu) in sys.borrow().cpus().iter().enumerate() {
        cpu_sensors.push(Box::new(SensorDataHandler::new(
            &cpu_group,
            format!("{}_usage", cpu.name()),
            Float(FloatSize::U4),
            Rc::clone(&sys),
            move |system| system.cpus()[i].cpu_usage(),
        )?));

        cpu_sensors.push(Box::new(SensorDataHandler::new(
            &cpu_group,
            format!("{}_frequency", cpu.name()),
            Unsigned(IntSize::U8),
            Rc::clone(&sys),
            move |system| system.cpus()[i].frequency(),
        )?));
    }

    // Add CPU Metadata
    cpu_group
        .new_attr_builder()
        .with_data(sys.borrow().cpus()[0].brand())
        .create("Brand")?;

    cpu_group
        .new_attr_builder()
        .with_data(sys.borrow().cpus()[0].vendor_id())
        .create("VendorId")?;

    Ok((cpu_group, cpu_sensors))
}
