use anyhow::Result;
use std::{cell::RefCell, rc::Rc, time::SystemTime};
use sysinfo::SystemExt;

use crate::{data_handler::initialize_system, file_generation::initialize_data_file};

const TARGET: usize = 10_800_000;
const BUFFER_SIZE: usize = 120;

mod data_handler;
mod file_generation;
mod types;

fn main() -> Result<()> {
    // Initialize the system information struct
    println!("Initializing system measurements...");
    let sys: Rc<RefCell<_>> = Rc::new(RefCell::new(initialize_system()));
    let systime = SystemTime::now();

    // Generate dataset file
    println!("Allocating space for data file...");
    let (file, mut sensor_handlers) = initialize_data_file(Rc::clone(&sys), systime)?;

    // Loop to collect data
    println!("Beginning to collect data...");
    for index in 0..2000 {
        // Refresh system information
        {
            let mut sys_ref = sys.borrow_mut();
            sys_ref.refresh_components_list();
            sys_ref.refresh_cpu();
            sys_ref.refresh_disks_list();
            sys_ref.refresh_memory();
        }

        for sensor_handler in &mut sensor_handlers {
            sensor_handler.update()?;
        }

        // Print a status update every 100 measurements
        if index % 100 == 0 {
            let time_elapsed = SystemTime::now().duration_since(systime)?.as_secs_f64();
            let measurements_taken = index + 1;
            let measurements_remaining = TARGET - measurements_taken;
            let mut t_remaining =
                measurements_remaining as f64 * time_elapsed / measurements_taken as f64;

            let days_remaining = t_remaining / 3600.0 / 24.0;
            t_remaining = days_remaining - days_remaining.floor();
            let hours_remaining = t_remaining * 24.0;
            t_remaining = hours_remaining - hours_remaining.floor();
            let minutes_remaining = t_remaining * 60.0;
            t_remaining = minutes_remaining - minutes_remaining.floor();
            let seconds_remaining = (t_remaining * 60.0).round();

            println!(
                "  Update: {}/{} measurements taken. Complete in {}d {}h {}m {}s",
                index,
                TARGET,
                days_remaining.floor(),
                hours_remaining.floor(),
                minutes_remaining.floor(),
                seconds_remaining
            );
        }
    }

    // Save off the last buffered data
    for sensor_handler in &mut sensor_handlers {
        sensor_handler.finalize()?;
    }

    // Properly close file
    file.close()?;

    // Done!
    Ok(())
}
