use std::cell::Ref;

use anyhow::Result;
use hdf5::{types::TypeDescriptor, Dataset, Extents, H5Type};
use ndarray::{s, Array1, Array2};
use sysinfo::System;

use crate::{
    types::{GroupOrFile, SystemPtr, Updatable},
    BUFFER_SIZE, TARGET,
};

pub struct MultiSensorDataHandler<T>
where
    T: H5Type,
{
    dataset: Dataset,
    update_fn: Box<dyn Fn(Ref<System>) -> Array1<T>>,
    system: SystemPtr,
    buffer: Array2<T>,
    depth: usize,
    dataset_index: usize,
    buffer_index: usize,
}

impl<T> MultiSensorDataHandler<T>
where
    T: H5Type + Default + 'static,
{
    pub fn new(
        parent: &impl GroupOrFile,
        name: impl AsRef<str>,
        type_descriptor: TypeDescriptor,
        depth: usize,
        sys: SystemPtr,
        func: impl Fn(Ref<System>) -> Array1<T> + 'static,
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
            depth,
            buffer: Array2::default((depth, BUFFER_SIZE)),
            dataset_index: 0,
            buffer_index: 0,
        })
    }
}
