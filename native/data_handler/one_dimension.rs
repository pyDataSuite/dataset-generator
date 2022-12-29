use std::cell::Ref;

use anyhow::Result;
use hdf5::{types::TypeDescriptor, Dataset, H5Type};
use ndarray::{s, Array1};
use sysinfo::System;

use crate::{
    types::{GroupOrFile, SystemPtr, Updatable, UpdateFunction},
    BUFFER_SIZE, TARGET,
};

pub struct SensorDataHandler<T>
where
    T: H5Type,
{
    dataset: Dataset,
    update_fn: UpdateFunction<T>,
    system: SystemPtr,
    buffer1d: Array1<T>,
    dataset_index: usize,
    buffer_index: usize,
}

impl<T> SensorDataHandler<T>
where
    T: H5Type + Clone + Default + 'static,
{
    pub fn new(
        parent: &impl GroupOrFile,
        name: impl AsRef<str>,
        type_descriptor: TypeDescriptor,
        sys: SystemPtr,
        func: impl Fn(Ref<System>) -> T + 'static,
    ) -> Result<Self> {
        let name = name.as_ref();

        Ok(Self {
            dataset: parent
                .builder()
                .empty_as(&type_descriptor)
                .shape(TARGET)
                .create(name)?,
            system: sys,
            update_fn: Box::new(func),
            buffer1d: Array1::from_elem(BUFFER_SIZE, T::default()),
            dataset_index: 0,
            buffer_index: 0,
        })
    }
}

impl<T> Updatable for SensorDataHandler<T>
where
    T: H5Type + Copy,
{
    fn update(&mut self) -> Result<()> {
        // Get new measurement
        let ret = (self.update_fn)(self.system.borrow());

        // Check if the buffer is full before continuing
        if self.buffer_index == BUFFER_SIZE {
            // Write to the buffer
            self.dataset.write_slice(
                &self.buffer1d,
                s![self.dataset_index..self.dataset_index + BUFFER_SIZE],
            )?;

            // Now update the indices
            self.buffer_index = 0;
            self.dataset_index += BUFFER_SIZE;
        }

        // Add the newest measurement to the buffer
        self.buffer1d[self.buffer_index] = ret;
        self.buffer_index += 1;

        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        // Create an owned subsection of the buffer array
        let section_of_buffer = self.buffer1d.slice(s![..self.buffer_index]).to_owned();

        // Then write the remainder to the dataset file
        self.dataset.write_slice(
            &section_of_buffer,
            s![
                self.dataset_index..self.dataset_index + self.buffer_index
            ],
        )?;

        Ok(())
    }
}
