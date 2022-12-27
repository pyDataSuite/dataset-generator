use anyhow::Result;
use hdf5::{types::TypeDescriptor::*, types::*, Group};
use ndarray::array;
use std::rc::Rc;
use sysinfo::{ComponentExt, SystemExt};

use crate::types::{SensorList, SystemPtr};

pub fn initialize_comp_data(
    cpu_group: &Group,
    disk_group: &Group,
    gpu_group: &Group,
    sys: SystemPtr,
) -> Result<SensorList> {
    let mut sensors = SensorList::new();

    for (i, comp) in sys.borrow().components().iter().enumerate() {
        // Get component name
        let mut comp_name = comp.label().replace(" ", "_");

        // Select proper group based on component name
        let group = match &comp_name.to_lowercase() {
            x if x.contains("nvme") => disk_group,
            x if x.contains("core") => cpu_group,
            x if x.contains("gpu") => gpu_group,
            _ => continue,
        };

        // Add temp to name if not already present
        if !comp_name.to_lowercase().contains("temp") {
            comp_name = comp_name + "_temps";
        }

        // Generate dataset
        sensors.push(Box::new(MultiSensorDataHandler::new(
            group,
            comp_name,
            Float(FloatSize::U4),
            3,
            Rc::clone(&sys),
            move |system| {
                array![
                    system.components()[i].temperature(),
                    system.components()[i].max(),
                    system.components()[i].critical().unwrap_or(0.0),
                ]
            },
        )?));
    }
    Ok(vec![])
}
