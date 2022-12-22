import h5py
import numpy as np
import psutil
from pathlib import Path
from functools import lru_cache
from timeit import default_timer
from os import remove

NUM_DATA_POINTS = 100_000
NUM_GOAL = 100_000 #10_800_000

def create_output_file( file_path=Path(__file__).parent/"dataset.hdf5" ) -> h5py.File:
    """
    Generates a hdf5 file that can be used to store data.
    """

    # Remove file if it exists
    try:
        remove( file_path )
    except:
        pass
    output_file = h5py.File( file_path, 'w' )
    return output_file

def create_datasets( file: h5py.File ) -> None:
    """This function initializes the file which contains all our data"""

    ### CPU Information
    # Create group for all the CPU data
    num_cpus = psutil.cpu_count()
    num_real_cpus = psutil.cpu_count(False)
    file.create_group( "CPU" )
    file["CPU"].attrs.create( "num_cpu", num_cpus )
    # Now add the relevant datasets
    times = psutil.cpu_times()
    for field in times._fields:
        file["CPU"].create_dataset( field, shape=(num_cpus, NUM_DATA_POINTS), maxshape=(num_cpus, None), dtype=np.double )
    # add a temps dataset
    file["CPU"].create_dataset( "cpu_temps", shape=(num_real_cpus, NUM_DATA_POINTS), maxshape=(num_real_cpus, None), dtype=np.double )
    
    ### RAM Information
    # Create group for all the RAM data
    ram_info = psutil.virtual_memory()
    file.create_group( "RAM" )
    # Now add the relevant datasets
    for field in ram_info._fields:
        file["RAM"].create_dataset( field, shape=(NUM_DATA_POINTS,), maxshape=(None,), dtype=np.double )
    
    ### Disk Information
    # Create group for all the Disk data
    disk_info = psutil.disk_usage( file.filename )
    file.create_group( "DISK" )
    # Now add the relevant datasets
    for field in disk_info._fields:
        file["DISK"].create_dataset( field, shape=(NUM_DATA_POINTS,), maxshape=(None,), dtype=np.double )

@lru_cache()
def find_dataset_path( file: h5py.File, dataset_name: str, parent_path="/" ):
    "Recursive function to find a dataset path given its name"
    
    if isinstance( file[ parent_path ], h5py.Dataset ):
        return ""
    
    if dataset_name in file[ parent_path ]:
        return f"{parent_path}/{dataset_name}"

    for key in file[ parent_path ]:
        res = find_dataset_path( file, dataset_name=dataset_name, parent_path=f"{parent_path}/{key}" )
        if res != "":
            return res

    return ""

def insert_data( file: h5py.File, index: int, **kwargs ):
    "Inserts the value of each kwarg into the dataset whose name matches it"

    # Add data to the new dataset
    for arg, val in kwargs.items():
        # print( f"Arg: {arg} | Val: {val}")
        path = find_dataset_path( file, arg )
        
        if len( file[path].shape ) == 2:
            file[path][:, index] = val
        elif len( file[path].shape ) == 1:
            file[path][index] = val
        else:
            raise Exception("This generator can only handle datasets that are one or two dimension!")


def collect_data():
    """
    Collects all data points into a single 
    """

    measurement_dict = {}

    ### CPU information
    # Counts / usage
    cpu_times = psutil.cpu_times( percpu=True )
    for i, field in enumerate( cpu_times[0]._fields ):
        measurement_dict[ field ] = [time[i] for time in cpu_times]
    # Temperature
    cpu_temps = psutil.sensors_temperatures(fahrenheit=False)['coretemp']
    measurement_dict["cpu_temps"] = [temp.current for temp in cpu_temps if "Core" in temp.label]

    ### RAM information
    ram_info = psutil.virtual_memory()
    for i, field in enumerate( ram_info._fields ):
        # break
        measurement_dict[ field ] = ram_info[i]

    ### Disk information
    disk_info = psutil.disk_usage("dataset-generator/dataset.hdf5")
    for i, field in enumerate( disk_info._fields ):
        # break
        measurement_dict[ field ] = disk_info[i]
        

    return measurement_dict

if __name__ == "__main__":
    print( "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~")
    print( "Generating the Dataset Dataset" )
    print( "This dataset tracks the system load that was required to make this dataset.")
    print( "Starting measurements..." )
    tstart = default_timer()
    file = create_output_file()
    create_datasets( file=file )

    # Poll sensors and gather 1000 data points
    for i in range(NUM_DATA_POINTS):
        if (i % 100) == 0:
            print(i, "measurements complete")
        insert_data( file, i, **collect_data())

    file.close()
    tend = default_timer()

    length = tend - tstart
    num_per_second = NUM_DATA_POINTS / length
    total_seconds = NUM_GOAL / num_per_second
    print( f"Took {NUM_DATA_POINTS} measurements in {length} seconds" )
    print( f"Comes out to { num_per_second } measurements/second" )
    print( f"{NUM_GOAL} measurements in {total_seconds} seconds, or {total_seconds/60} minutes, or {total_seconds/60/60} hours, or {total_seconds/3600/24} days" )