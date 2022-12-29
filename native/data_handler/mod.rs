use sysinfo::{CpuRefreshKind, RefreshKind, System, SystemExt};

mod one_dimension;
mod two_dimension;

pub use one_dimension::*;
pub use two_dimension::*;

pub fn initialize_system() -> System {
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
