use std::{cell::RefCell, rc::Rc};

use anyhow::Result;
use hdf5::DatasetBuilder;
use sysinfo::System;

pub type SystemPtr = Rc<RefCell<System>>;
pub type SensorList = Vec<Box<dyn Updatable>>;

pub trait GroupOrFile {
    fn builder(&self) -> DatasetBuilder;
}

pub trait Updatable {
    fn update(&mut self) -> Result<()>;
    fn finalize(&mut self) -> Result<()>;
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
