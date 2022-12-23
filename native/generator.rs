use anyhow::Result;
use hdf5::{
    types::{FloatSize, IntSize, TypeDescriptor},
    Dataset, DatasetBuilder, Extents, File, Group,
};
use ndarray::s;
use std::{
    cell::{Ref, RefCell},
    fs::remove_file,
    path::Path,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::{ComponentExt, CpuExt, CpuRefreshKind, DiskExt, RefreshKind, System, SystemExt};
use TypeDescriptor::*;

const TARGET: usize = 10_800_000;
// const BUFFER_SIZE: usize = 1000000;
type SystemPtr = Rc<RefCell<System>>;
type SensorList = Vec<SensorDataHandler>;

trait GroupOrFile {
    fn builder(&self) -> DatasetBuilder;
}
impl GroupOrFile for hdf5::Group {
    fn builder(&self) -> DatasetBuilder {
        self.new_dataset_builder()
    }
}
impl GroupOrFile for hdf5::File {
    fn builder(&self) -> DatasetBuilder {
        // self.new_dataset_builder()
        self.new_dataset_builder()
    }
}

struct SensorDataHandler {
    dataset: Dataset,
    update_fn: Box<dyn Fn(&Dataset, Ref<System>, usize) -> Result<()>>,
    system: SystemPtr,
}

impl SensorDataHandler {
    fn update(&self, index: usize) -> Result<()> {
        (self.update_fn)(&self.dataset, self.system.borrow(), index)?;
        Ok(())
    }

    fn new(
        parent: &impl GroupOrFile,
        name: impl AsRef<str>,
        type_descriptor: TypeDescriptor,
        depth: usize,
        sys: SystemPtr,
        func: impl Fn(&Dataset, Ref<System>, usize) -> Result<()> + 'static,
    ) -> Result<Self> {
        let name = name.as_ref();
        let extents = match depth {
            0 => Extents::new(TARGET),
            d => Extents::new((d, TARGET)),
        };
        Ok(Self {
            dataset: parent
                .builder()
                // .chunk(chunk)
                .empty_as(&type_descriptor)
                .shape(extents)
                .create(name)?,
            system: sys,
            update_fn: Box::new(func),
        })
    }
}

fn main() -> Result<()> {
    // Initialize the system information struct
    println!("Initializing system measurements...");
    let sys: Rc<RefCell<_>> = Rc::new(RefCell::new(initialize_system()));
    let systime = SystemTime::now();

    // Generate dataset file
    println!("Allocating space for data file...");
    let (file, sensor_handlers) = initialize_data_file(Rc::clone(&sys), systime)?;

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

        for sensor_handler in &sensor_handlers {
            sensor_handler.update(index)?;
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

    // Properly close file
    file.close()?;

    // Done!
    Ok(())
}

fn initialize_system() -> System {
    // Select which components of the system we will track
    let refreshkind = RefreshKind::new()
        .with_cpu(CpuRefreshKind::everything())
        .with_disks_list()
        .with_memory()
        .with_networks_list()
        .with_components_list();

    // Scan for system info
    System::new_with_specifics(refreshkind)
}

fn initialize_data_file(
    sys: SystemPtr,
    sys_time: SystemTime,
) -> Result<(File, Vec<SensorDataHandler>)> {
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
    let mut sensor_handlers: Vec<SensorDataHandler> = vec![];

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
    sensor_handlers.push(SensorDataHandler::new(
        &file,
        "system_time",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, _, index| -> Result<()> {
            data.write_slice(
                &[SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()],
                s![index..index + 1],
            )?;
            Ok(())
        },
    )?);

    sensor_handlers.push(SensorDataHandler::new(
        &file,
        "time_elapsed",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        move |data, _, index| -> Result<()> {
            data.write_slice(
                &[SystemTime::now().duration_since(sys_time)?.as_millis() as u64],
                s![index..index + 1],
            )?;
            Ok(())
        },
    )?);

    sensor_handlers.push(SensorDataHandler::new(
        &file,
        "measurements_taken",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        move |data, _, index| -> Result<()> {
            data.write_slice(&[index + 1], s![index..index + 1])?;
            Ok(())
        },
    )?);

    sensor_handlers.push(SensorDataHandler::new(
        &file,
        "measurements_remaining",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        move |data, _, index| -> Result<()> {
            data.write_slice(&[TARGET - index - 1], s![index..index + 1])?;
            Ok(())
        },
    )?);

    // Return the file instance
    Ok((file, sensor_handlers))
}

fn initialize_cpu_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut cpu_sensors = SensorList::new();
    let cpu_group = file.create_group("CPU")?;
    let num_cpus = sys.borrow().cpus().len();

    ////////////////////
    // System time
    cpu_sensors.push(SensorDataHandler::new(
        &cpu_group,
        "system_time",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, _, index| -> Result<()> {
            data.write_slice(
                &[SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()],
                s![index..index + 1],
            )?;
            Ok(())
        },
    )?);

    ////////////////////
    // Grouped CPU Usage
    cpu_sensors.push(SensorDataHandler::new(
        &cpu_group,
        "grouped_cpu_usage",
        Float(FloatSize::U4),
        num_cpus,
        Rc::clone(&sys),
        |data, system, index| -> Result<()> {
            let cpu_usage_tot: Vec<f32> = system.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
            data.write_slice(&cpu_usage_tot, s![.., index])?;
            Ok(())
        },
    )?);

    // Grouped CPU Frequency
    cpu_sensors.push(SensorDataHandler::new(
        &cpu_group,
        "grouped_cpu_frequency",
        Unsigned(IntSize::U8),
        num_cpus,
        Rc::clone(&sys),
        |data, system, index| {
            let cpu_freq_list: Vec<_> = system.cpus().iter().map(|cpu| cpu.frequency()).collect();
            data.write_slice(&cpu_freq_list, s![.., index])?;
            Ok(())
        },
    )?);

    // Per-CPU Stats
    for (i, cpu) in sys.borrow().cpus().iter().enumerate() {
        cpu_sensors.push(SensorDataHandler::new(
            &cpu_group,
            format!("{}_usage", cpu.name()),
            Float(FloatSize::U4),
            0,
            Rc::clone(&sys),
            move |data, system, index| {
                data.write_slice(&[system.cpus()[i].cpu_usage()], s![index..index + 1])?;
                Ok(())
            },
        )?);

        cpu_sensors.push(SensorDataHandler::new(
            &cpu_group,
            format!("{}_frequency", cpu.name()),
            Unsigned(IntSize::U8),
            0,
            Rc::clone(&sys),
            move |data, system, index| {
                data.write_slice(&[system.cpus()[i].cpu_usage()], s![index..index + 1])?;
                Ok(())
            },
        )?);
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

fn initialize_ram_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut ram_sensors = SensorList::new();
    let ram_group = file.create_group("RAM")?;

    ////////////////////
    // System time
    ram_sensors.push(SensorDataHandler::new(
        &ram_group,
        "system_time",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, _, index| -> Result<()> {
            data.write_slice(
                &[SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()],
                s![index..index + 1],
            )?;
            Ok(())
        },
    )?);

    ////////////////////
    // Generate RAM Sensor Handlers
    ram_sensors.push(SensorDataHandler::new(
        &ram_group,
        "total_memory",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, system, index| -> Result<()> {
            data.write_slice(&[system.total_memory()], s![index..index + 1])?;
            Ok(())
        },
    )?);

    ram_sensors.push(SensorDataHandler::new(
        &ram_group,
        "used_memory",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, system, index| -> Result<()> {
            data.write_slice(&[system.used_memory()], s![index..index + 1])?;
            Ok(())
        },
    )?);

    ram_sensors.push(SensorDataHandler::new(
        &ram_group,
        "total_swap",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, system, index| -> Result<()> {
            data.write_slice(&[system.total_swap()], s![index..index + 1])?;
            Ok(())
        },
    )?);

    ram_sensors.push(SensorDataHandler::new(
        &ram_group,
        "used_swap",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, system, index| -> Result<()> {
            data.write_slice(&[system.used_swap()], s![index..index + 1])?;
            Ok(())
        },
    )?);

    Ok((ram_group, ram_sensors))
}

fn initialize_disk_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut disk_sensors = SensorList::new();
    let disk_group = file.create_group("DISK")?;
    let num_disks = sys.borrow().disks().len();

    ////////////////////
    // System time
    disk_sensors.push(SensorDataHandler::new(
        &disk_group,
        "system_time",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, _, index| -> Result<()> {
            data.write_slice(
                &[SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()],
                s![index..index + 1],
            )?;
            Ok(())
        },
    )?);

    ////////////////////
    // Grouped Disk Info
    disk_sensors.push(SensorDataHandler::new(
        &disk_group,
        "grouped_available_space",
        Unsigned(IntSize::U8),
        num_disks,
        Rc::clone(&sys),
        |data, system, index| -> Result<()> {
            let disk_availabilities: Vec<_> = system
                .disks()
                .iter()
                .map(|disk| disk.available_space())
                .collect();
            data.write_slice(&disk_availabilities, s![.., index])?;
            Ok(())
        },
    )?);

    disk_sensors.push(SensorDataHandler::new(
        &disk_group,
        "grouped_total_space",
        Unsigned(IntSize::U8),
        num_disks,
        Rc::clone(&sys),
        |data, system, index| -> Result<()> {
            let disk_space: Vec<_> = system
                .disks()
                .iter()
                .map(|disk| disk.total_space())
                .collect();
            data.write_slice(&disk_space, s![.., index])?;
            Ok(())
        },
    )?);

    // Per-CPU Stats
    for (i, disk) in sys.borrow().disks().iter().enumerate() {
        let disk_name = format!("{:?}_available_space", disk.name()).replace("/", "_");

        disk_sensors.push(SensorDataHandler::new(
            &disk_group,
            format!("{}_available_space", disk_name),
            Unsigned(IntSize::U8),
            0,
            Rc::clone(&sys),
            move |data, system, index| {
                data.write_slice(&[system.disks()[i].available_space()], s![index..index + 1])?;
                Ok(())
            },
        )?);

        disk_sensors.push(SensorDataHandler::new(
            &disk_group,
            format!("{}_total_space", disk_name),
            Unsigned(IntSize::U8),
            0,
            Rc::clone(&sys),
            move |data, system, index| {
                data.write_slice(&[system.disks()[i].total_space()], s![index..index + 1])?;
                Ok(())
            },
        )?);

        disk_sensors.push(SensorDataHandler::new(
            &disk_group,
            format!("{}_is_removable", disk_name),
            Boolean,
            0,
            Rc::clone(&sys),
            move |data, system, index| {
                data.write_slice(&[system.disks()[i].is_removable()], s![index..index + 1])?;
                Ok(())
            },
        )?);
    }

    // Add CPU Metadata
    disk_group
        .new_attr_builder()
        .with_data(sys.borrow().cpus()[0].brand())
        .create("Brand")?;

    disk_group
        .new_attr_builder()
        .with_data(sys.borrow().cpus()[0].vendor_id())
        .create("VendorId")?;

    Ok((disk_group, disk_sensors))
}

fn initialize_gpu_data(
    file: &File,
    sys: SystemPtr,
    _time: SystemTime,
) -> Result<(Group, SensorList)> {
    // Get specific constants
    let mut gpu_sensors = SensorList::new();
    let gpu_group = file.create_group("GPU")?;

    ////////////////////
    // System time
    gpu_sensors.push(SensorDataHandler::new(
        &gpu_group,
        "system_time",
        Unsigned(IntSize::U8),
        0,
        Rc::clone(&sys),
        |data, _, index| -> Result<()> {
            data.write_slice(
                &[SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()],
                s![index..index + 1],
            )?;
            Ok(())
        },
    )?);

    ////////////////////
    // Generate gpu Sensor Handlers

    Ok((gpu_group, gpu_sensors))
}

fn initialize_comp_data(
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
        sensors.push(SensorDataHandler::new(
            group,
            comp_name,
            Float(FloatSize::U4),
            3,
            Rc::clone(&sys),
            move |data, system, index| {
                data.write_slice(
                    &[
                        system.components()[i].temperature(),
                        system.components()[i].max(),
                        system.components()[i].critical().unwrap_or(0.0),
                    ],
                    s![.., index],
                )?;
                Ok(())
            },
        )?);
    }
    Ok(vec![])
}
