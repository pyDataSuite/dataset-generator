use std::fmt::Debug;
use std::{cell::Ref, usize};

use anyhow::Result;
use hdf5::{types::TypeDescriptor, Dataset, H5Type};
use ndarray::s;
use ndarray::Array2;
use sysinfo::System;

use crate::{
    types::{GroupOrFile, StoredMultiSensorUpdateFunction, SystemPtr, Updatable},
    BUFFER_SIZE, TARGET,
};

pub struct MultiSensorDataHandler<T, const D: usize>
where
    T: H5Type,
{
    dataset: Dataset,
    update_fn: StoredMultiSensorUpdateFunction<T, D>,
    system: SystemPtr,
    buffer2d: Array2<T>,
    dataset_index: usize,
    buffer_index: usize,
}

impl<T, const D: usize> MultiSensorDataHandler<T, D>
where
    T: H5Type + Default + Copy + 'static,
{
    pub fn new(
        parent: &impl GroupOrFile,
        name: impl AsRef<str>,
        type_descriptor: TypeDescriptor,
        sys: SystemPtr,
        func: impl Fn(Ref<System>) -> [T; D] + 'static,
    ) -> Result<Self> {
        let name = name.as_ref();

        Ok(Self {
            dataset: parent
                .builder()
                .empty_as(&type_descriptor)
                .shape((D, TARGET))
                .create(name)?,
            system: sys,
            update_fn: Box::new(func),
            buffer2d: Array2::from_elem((D, BUFFER_SIZE), T::default()),
            dataset_index: 0,
            buffer_index: 0,
        })
    }
}

impl<T, const D: usize> Updatable for MultiSensorDataHandler<T, D>
where
    T: H5Type + Copy + Default + Debug,
{
    fn update(&mut self) -> Result<()> {
        // Get new measurement
        let ret = (self.update_fn)(self.system.borrow());

        // Check if the buffer is full before continuing
        if self.buffer_index == BUFFER_SIZE {
            // Write to the buffer
            self.dataset.write_slice(
                &self.buffer2d,
                s![.., self.dataset_index..self.dataset_index + BUFFER_SIZE],
            )?;

            // Reset the buffer index
            self.buffer_index = 0;

            // Move the dataset index to its next location
            self.dataset_index += BUFFER_SIZE;
        }

        // Add the newest measurement to the buffer
        for (i, element) in ret.iter().enumerate() {
            self.buffer2d[[i, self.buffer_index]] = *element;
        }

        // And now we move to the next part of the buffer
        self.buffer_index += 1;

        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        // Create an owned subsection of the buffer array
        let section_of_buffer = self.buffer2d.slice(s![.., ..self.buffer_index]).to_owned();

        // Then write the remainder to the dataset file
        self.dataset.write_slice(
            &section_of_buffer,
            s![
                ..,
                self.dataset_index..self.dataset_index + self.buffer_index
            ],
        )?;

        Ok(())
    }
}
