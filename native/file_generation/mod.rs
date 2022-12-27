use std::{fs::remove_file, path::Path, time::SystemTime};
use sysinfo::SystemExt;

use anyhow::Result;
use hdf5::File;

use crate::types::{SensorList, SystemPtr};

use comp::*;
use cpu::*;
use disk::*;
use gpu::*;
use ram::*;

mod comp;
mod cpu;
mod disk;
mod gpu;
mod ram;

pub fn initialize_data_file(sys: SystemPtr, sys_time: SystemTime) -> Result<(File, SensorList)> {
    // let sys = sys.borrow_mut();
    // Remove data file if it already exists
    let file_path = Path::new("dataset.hdf5");
    if file_path.exists() {
        remove_file(file_path)?;
    }

    // Generate new data file
    let file = File::create(file_path)?;

    // Add system information as attributes to the root of the file
    if let Some(name) = sys.borrow().name() {
        file.new_attr_builder()
            .with_data(&name)
            .create("SystemName")?;
    }
    if let Some(kernel_version) = sys.borrow().kernel_version() {
        file.new_attr_builder()
            .with_data(&kernel_version)
            .create("KernelVersion")?;
    }
    if let Some(os_version) = sys.borrow().os_version() {
        file.new_attr_builder()
            .with_data(&os_version)
            .create("OsVersion")?;
    }
    if let Some(host_name) = sys.borrow().host_name() {
        file.new_attr_builder()
            .with_data(&host_name)
            .create("HostName")?;
    }

    // Get the list of SensorDataHandlers
    // let mut sensor_handlers: Vec<Box<dyn Updatable>> = vec![];
    let mut sensor_handlers = SensorList::new();

    // Get information about specific systems
    let (cpu, mut cpu_handlers) = initialize_cpu_data(&file, Rc::clone(&sys), sys_time)?;
    let (_ram, mut ram_handlers) = initialize_ram_data(&file, Rc::clone(&sys), sys_time)?;
    let (disk, mut disk_handlers) = initialize_disk_data(&file, Rc::clone(&sys), sys_time)?;
    let (gpu, mut gpu_handlers) = initialize_gpu_data(&file, Rc::clone(&sys), sys_time)?;
    let mut comp_handlers = initialize_comp_data(&cpu, &disk, &gpu, Rc::clone(&sys))?;

    // Add handlers to the overall handler list
    sensor_handlers.append(&mut cpu_handlers);
    sensor_handlers.append(&mut ram_handlers);
    sensor_handlers.append(&mut disk_handlers);
    sensor_handlers.append(&mut gpu_handlers);
    sensor_handlers.append(&mut comp_handlers);

    // Now add datasets that correspond to the actual dataset generation
    sensor_handlers.push(Box::new(SensorDataHandler::new(
        &file,
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

    sensor_handlers.push(Box::new(SensorDataHandler::new(
        &file,
        "time_elapsed",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        move |_| {
            SystemTime::now()
                .duration_since(sys_time)
                .unwrap_or_default()
                .as_millis() as u64
        },
    )?));

    let measurements_taken = Rc::new(RefCell::new(0_usize));
    let clone = Rc::clone(&measurements_taken);
    sensor_handlers.push(Box::new(SensorDataHandler::new(
        &file,
        "measurements_taken",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        move |_| -> usize {
            let mut mt = clone.borrow_mut();
            *mt = *mt + 1;
            *mt
        },
    )?));

    let clone = Rc::clone(&measurements_taken);
    sensor_handlers.push(Box::new(SensorDataHandler::new(
        &file,
        "measurements_remaining",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        move |_| TARGET - *clone.borrow(),
    )?));

    // Return the file instance
    Ok((file, sensor_handlers))
}
