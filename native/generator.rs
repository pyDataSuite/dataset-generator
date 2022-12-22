use anyhow::Result;
use hdf5::{
    types::{FloatSize, IntSize, TypeDescriptor},
    File,
};
use ndarray::s;
use std::{
    fs::remove_file,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use sysinfo::{ComponentExt, CpuExt, CpuRefreshKind, DiskExt, RefreshKind, System, SystemExt};
use TypeDescriptor::*;

const TARGET: usize = 10_800_000;
const CHUNKSIZE: usize = 1000000;

fn main() -> Result<()> {
    // Initialize the system information struct
    println!("Initializing system measurements...");
    let mut sys = initialize_system();
    let systime = SystemTime::now();

    // Generate dataset file
    println!("Allocating space for data file...");
    let file = initialize_data_file(&sys)?;

    // Loop to collect data
    println!("Beginning to collect data...");
    for index in 0..2000 {
        // Refresh system information
        sys.refresh_components_list();
        sys.refresh_cpu();
        sys.refresh_disks_list();
        sys.refresh_memory();

        // Store data
        let t_remaining = populate_data(
            &sys,
            &file,
            index,
            systime.duration_since(UNIX_EPOCH)?.as_millis(),
            systime.elapsed()?.as_millis(),
        )?;

        // Print a status update every 1000 measurements
        if index % 100 == 0 {
            let mut t_remaining = Duration::from_millis(t_remaining.floor() as u64).as_secs_f64();
            // let mut t_remaining = 24.467_f64;
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

fn initialize_data_file(sys: &System) -> Result<File> {
    // Remove data file if it already exists
    let file_path = Path::new("dataset.hdf5");
    if file_path.exists() {
        remove_file(file_path)?;
    }

    // Generate new data file
    let file = File::create(file_path)?;

    // Add system information as attributes to the root of the file
    if let Some(name) = sys.name() {
        file.new_attr_builder()
            .with_data(&name)
            .create("SystemName")?;
    }
    if let Some(kernel_version) = sys.kernel_version() {
        file.new_attr_builder()
            .with_data(&kernel_version)
            .create("KernelVersion")?;
    }
    if let Some(os_version) = sys.os_version() {
        file.new_attr_builder()
            .with_data(&os_version)
            .create("OsVersion")?;
    }
    if let Some(host_name) = sys.host_name() {
        file.new_attr_builder()
            .with_data(&host_name)
            .create("HostName")?;
    }

    // Generate groups for CPU, RAM, and DISK
    let cpu = file.create_group("CPU")?;
    let ram = file.create_group("RAM")?;
    let disk = file.create_group("DISK")?;
    let gpu = file.create_group("GPU")?;

    // Generate datasets for the CPUs
    let num_cpus = sys.cpus().len();
    cpu.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("system_time")?;

    cpu.new_dataset_builder()
        .chunk((num_cpus, CHUNKSIZE))
        .empty_as(&Float(FloatSize::U4))
        .shape((num_cpus, TARGET))
        .create("grouped_cpu_usage")?;

    cpu.new_dataset_builder()
        .chunk((num_cpus, CHUNKSIZE))
        .empty_as(&Float(FloatSize::U4))
        .shape((num_cpus, TARGET))
        .create("grouped_cpu_frequency")?;

    for _cpu in sys.cpus() {
        cpu.new_dataset_builder()
            .chunk(CHUNKSIZE)
            .empty_as(&Float(FloatSize::U4))
            .shape(TARGET)
            .create(format!("{}_usage", _cpu.name()).as_str())?;

        cpu.new_dataset_builder()
            .chunk(CHUNKSIZE)
            .empty_as(&Float(FloatSize::U4))
            .shape(TARGET)
            .create(format!("{}_frequency", _cpu.name()).as_str())?;
    }

    cpu.new_attr_builder()
        .with_data(sys.cpus()[0].brand())
        .create("Brand")?;

    cpu.new_attr_builder()
        .with_data(sys.cpus()[0].vendor_id())
        .create("VendorId")?;

    // Generate datasets for the RAM
    ram.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("system_time")?;

    ram.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("total_memory")?;

    ram.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("used_memory")?;

    ram.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("total_swap")?;

    ram.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("used_swap")?;

    // Generate datasets for the DISK
    let num_disks = sys.disks().len();
    disk.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("system_time")?;

    disk.new_dataset_builder()
        .chunk((num_disks, CHUNKSIZE))
        .empty_as(&Unsigned(IntSize::U8))
        .shape((num_disks, TARGET))
        .create("grouped_available_space")?;

    disk.new_dataset_builder()
        .chunk((num_disks, CHUNKSIZE))
        .empty_as(&Unsigned(IntSize::U8))
        .shape((num_disks, TARGET))
        .create("grouped_total_space")?;

    for _disk in sys.disks() {
        let name = format!("{:?}", _disk.name()).replace("/", "_");

        disk.new_dataset_builder()
            .chunk(CHUNKSIZE)
            .empty_as(&Unsigned(IntSize::U8))
            .shape(TARGET)
            .create(format!("{}_available_space", name).as_str())?;

        disk.new_dataset_builder()
            .chunk(CHUNKSIZE)
            .empty_as(&Unsigned(IntSize::U8))
            .shape(TARGET)
            .create(format!("{}_total_space", name).as_str())?;

        disk.new_dataset_builder()
            .chunk(CHUNKSIZE)
            .empty_as(&Boolean)
            .shape(TARGET)
            .create(format!("{}_is_removable", name).as_str())?;
    }

    // Create dataset for component temperatures
    for _comp in sys.components() {
        // Get component name without spaces
        let mut comp_name = _comp.label().replace(" ", "_");

        // Select proper group based on component name
        let group = match &comp_name {
            x if x.contains("nvme") => &disk,
            x if x.contains("core") => &cpu,
            x if x.contains("gpu") => &gpu,
            _ => continue, // If a group isn't found, skip to the next component
        };

        // Add temp to name if not already present
        if !comp_name.to_lowercase().contains("temp") {
            comp_name = comp_name + "_temps";
        }

        // Generate dataset
        group
            .new_dataset_builder()
            .empty_as(&Float(FloatSize::U4))
            .shape((3, TARGET))
            .chunk([3, CHUNKSIZE])
            .create(comp_name.as_str())?;
    }

    // Generate datasets for the GPU
    gpu.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("system_time")?;

    // Now add datasets that correspond to the actual dataset generation
    file.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("system_time")?;

    file.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("time_elapsed")?;

    file.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("measurements_taken")?;

    file.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("measurements_remaining")?;

    file.new_dataset_builder()
        .chunk(CHUNKSIZE)
        .empty_as(&Unsigned(IntSize::U8))
        .shape(TARGET)
        .create("time_remaining")?;

    // Return the file instance
    Ok(file)
}

fn populate_data(
    sys: &System,
    file: &File,
    index: usize,
    sys_time: u128,
    time_elapsed: u128,
) -> Result<f64> {
    // Insert data into the CPU datasets \\
    let cpu = file.group("CPU")?;
    cpu.dataset("system_time")?
        .write_slice(&[sys_time as u64], s![index..index + 1])?;

    let mut usages = vec![0f32; sys.cpus().len()];
    let mut freqs = vec![0u64; sys.cpus().len()];
    for (i, _cpu) in sys.cpus().iter().enumerate() {
        let name = _cpu.name();
        let usage = _cpu.cpu_usage();
        let freq = _cpu.frequency();

        // Insert data into grouped datasets
        usages[i] = usage;
        freqs[i] = freq;

        // Enter specific CPU information
        cpu.dataset(format!("{}_usage", name).as_str())?
            .write_slice(&[usage], s![i..i + 1])?;
        cpu.dataset(format!("{}_frequency", _cpu.name()).as_str())?
            .write_slice(&[freq], s![i..i + 1])?;
    }

    // Add grouped CPU stats once
    cpu.dataset("grouped_cpu_usage")?
        .write_slice(&usages, s![.., index])?;
    cpu.dataset("grouped_cpu_frequency")?
        .write_slice(&freqs, s![.., index])?;

    // Insert data into the RAM datasets \\
    let ram = file.group("RAM")?;
    ram.dataset("system_time")?
        .write_slice(&[sys_time as u64], s![index..index + 1])?;
    ram.dataset("total_memory")?
        .write_slice(&[sys.total_memory()], s![index..index + 1])?;
    ram.dataset("used_memory")?
        .write_slice(&[sys.used_memory()], s![index..index + 1])?;
    ram.dataset("total_swap")?
        .write_slice(&[sys.total_swap()], s![index..index + 1])?;
    ram.dataset("used_swap")?
        .write_slice(&[sys.used_swap()], s![index..index + 1])?;

    // Insert data into the DISK datasets \\
    let disk = file.group("DISK")?;
    disk.dataset("system_time")?
        .write_slice(&[sys_time as u64], s![index..index + 1])?;

    let mut avail = vec![0u64; sys.disks().len()];
    let mut total = vec![0u64; sys.disks().len()];
    for (i, _disk) in sys.disks().iter().enumerate() {
        let name = format!("{:?}", _disk.name()).replace("/", "_");
        let available_space = _disk.available_space();
        let total_space = _disk.total_space();
        let removable = _disk.is_removable();

        avail[i] = available_space;
        total[i] = total_space;

        // Insert data into grouped datasets
        // Enter specific disk information
        disk.dataset(format!("{}_total_space", name).as_str())?
            .write_slice(&[total_space], s![i..i + 1])?;
        disk.dataset(format!("{}_available_space", name).as_str())?
            .write_slice(&[available_space], s![i..i + 1])?;
        disk.dataset(format!("{}_is_removable", name).as_str())?
            .write_slice(&[removable], s![i..i + 1])?;
    }
    disk.dataset("grouped_available_space")?
        .write_slice(&avail, s![.., index])?;
    disk.dataset("grouped_total_space")?
        .write_slice(&total, s![.., index])?;

    // Insert data into the GPU datasets \\
    let gpu = file.group("GPU")?;
    gpu.dataset("system_time")?
        .write_slice(&[sys_time as u64], s![index..index + 1])?;

    // Insert component temp data into relevant datasets \\
    for _comp in sys.components() {
        let mut comp_name = _comp.label().replace(" ", "_");

        let group = match &comp_name {
            x if x.contains("nvme") => &disk,
            x if x.contains("core") => &cpu,
            x if x.contains("gpu") => &gpu,
            _ => continue,
        };

        // Add temp to name if not already present
        if !comp_name.to_lowercase().contains("temp") {
            comp_name = comp_name + "_temps";
        }

        // Add component to dataset
        group.dataset(comp_name.as_str())?.write_slice(
            &[
                _comp.temperature(),
                _comp.max(),
                _comp.critical().unwrap_or(0.0),
            ],
            s![.., index],
        )?;
    }

    // Insert data into cumulative datasets \\
    file.dataset("system_time")?
        .write_slice(&[sys_time as u64], s![index..index + 1])?;

    file.dataset("time_elapsed")?
        .write_slice(&[time_elapsed as u64], s![index..index + 1])?;

    let measurements_taken = index + 1;
    let measurements_remaining = TARGET - measurements_taken;
    let time_remaining =
        measurements_remaining as f64 * time_elapsed as f64 / measurements_taken as f64;

    file.dataset("measurements_taken")?
        .write_slice(&[measurements_taken], s![index..index + 1])?;

    file.dataset("measurements_remaining")?
        .write_slice(&[measurements_remaining], s![index..index + 1])?;

    file.dataset("time_remaining")?
        .write_slice(&[time_remaining], s![index..index + 1])?;

    Ok(time_remaining)
}

// Please note that we use "new_all" to ensure that all list of
// components, network interfaces, disks and users are already
// filled!
// let mut sys = System::new_all();

// First we update all information of our `System` struct.
// sys.refresh_all();

// We display all disks' information:
// println!("=> disks:");
// for disk in sys.disks() {
//     println!("{:?}", disk);
// }

// Network interfaces name, data received and data transmitted:
// println!("=> networks:");
// for (interface_name, data) in sys.networks() {
//     println!(
//         "{}: {}/{} B",
//         interface_name,
//         data.received(),
//         data.transmitted()
//     );
// }

// Components temperature:
// println!("=> components:");
// for component in sys.components() {
//     println!("{:?}", component);
// }

// println!("=> system:");
// RAM and swap information:
// println!("total memory: {} bytes", sys.total_memory());
// println!("used memory : {} bytes", sys.used_memory());
// println!("total swap  : {} bytes", sys.total_swap());
// println!("used swap   : {} bytes", sys.used_swap());

// Display system information:
// println!("System name:             {:?}", sys.name());
// println!("System kernel version:   {:?}", sys.kernel_version());
// println!("System OS version:       {:?}", sys.os_version());
// println!("System host name:        {:?}", sys.host_name());

// Number of CPUs:
// println!("NB CPUs: {}", sys.cpus().len());
// for cpu in sys.cpus() {
//     cpu.
//     println!(" CPU: {:?}", cpu);
// }

// Display processes ID, name na disk usage:
// for (pid, process) in sys.processes() {
//     println!("[{}] {} {:?}", pid, process.name(), process.disk_usage());
// }
