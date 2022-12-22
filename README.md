# Dataset-Generator
This repository contains code that will generate a large dataset using your computer's sensors. The dataset includes:
1. CPU Load information
2. RAM Usage information
3. Disk usage information
4. Graphics card information.

By default the generator will take 10,800,000 measurements to store in the dataset. This number was chosen to be arbitrarily large to push the limits of data processing and plotting scripts.

In total, the dataset should be somewhere around 7 Gigabytes, so make sure you have the space for it.

## Dataset Contents
In a way, the dataset that is generated for by this tool is essentially a datasat for the work it takes to create the dataset. In practice such a dataset is useless, but it does provide an easy way to produce data that you can use to practice data analysis.

## Performance Concerns
The original attempt at the dataset generator is contained in the `scripted/generator.py` file. I wrote this as a quick and easy script to populate an HDF5 file, but ended up being slowed down significantly by python's temperature sensing packages, which call the `cat` command on a specific directory and then use string parsing to get the data. This yielded a measurement rate of ~15 samples per second, or 10,800,000 samples in over 7 days.

*unacceptable*

After learning that, I put more time into building up a native dataset generator which is written in Rust. This one requires significantly more code, but I was very hopeful it would execute much faster. After a minimum working script was created, I discovered that the native solution would be complete in a little over 5 days. That's 2 days faster than python, but still unacceptably slow. If you would like to see a log of my efforts to optimize this performance, please visit [this readme](native/performance/README.md)