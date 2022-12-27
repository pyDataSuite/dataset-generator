use std::cell::Ref;

use anyhow::Result;
use hdf5::{types::TypeDescriptor, Dataset, Extents, H5Type};
use ndarray::s;
use sysinfo::System;

use crate::{
    types::{GroupOrFile, SystemPtr, Updatable},
    BUFFER_SIZE, TARGET,
};

pub struct SensorDataHandler<T>
where
    T: H5Type,
{
    dataset: Dataset,
    update_fn: Box<dyn Fn(Ref<System>) -> T>,
    system: SystemPtr,
    buffer: Vec<Option<T>>,
    depth: usize,
    dataset_index: usize,
}

impl<T> SensorDataHandler<T>
where
    T: H5Type + Clone + 'static,
{
    pub fn new(
        parent: &impl GroupOrFile,
        name: impl AsRef<str>,
        type_descriptor: TypeDescriptor,
        depth: usize,
        sys: SystemPtr,
        func: impl Fn(Ref<System>) -> T + 'static,
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
            buffer: vec![None; BUFFER_SIZE],
            dataset_index: 0,
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

        // Find next unclaimed spot
        if let Some(b_index) = self.buffer.iter().position(|e| e.is_none()) {
            self.buffer[b_index] = Some(ret);
        } else {
            // Create temporary buffer
            let temp: Vec<T> = self
                .buffer
                .iter_mut()
                .map(|e| {
                    let mut element: Option<T> = None;
                    std::mem::swap(e, &mut element);
                    element.unwrap()
                })
                .collect();

            // Now save it off
            match self.depth {
                0 => {
                    let slice = s![self.dataset_index..self.dataset_index + BUFFER_SIZE];
                    self.dataset.write_slice(&temp, slice)?;
                }
                _ => {
                    let slice = s![.., self.dataset_index..self.dataset_index + BUFFER_SIZE];
                    self.dataset.write_slice(&temp, slice)?;
                }
            };

            self.buffer[0] = Some(ret);

            // And now we move to the next part of the dataset
            self.dataset_index += BUFFER_SIZE;
        };
        Ok(())
    }

    fn finalize(&mut self) -> Result<()> {
        let final_buff: Vec<T> = self.buffer.iter().filter_map(|f| *f).collect();

        // Now save it off
        match self.depth {
            0 => {
                let slice = s![self.dataset_index..self.dataset_index + final_buff.len()];
                self.dataset.write_slice(&final_buff, slice)?;
            }
            _ => {
                let slice = s![
                    ..,
                    self.dataset_index..self.dataset_index + final_buff.len()
                ];
                self.dataset.write_slice(&final_buff, slice)?;
            }
        };
        Ok(())
    }
}
