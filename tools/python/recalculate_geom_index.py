#!/usr/bin/env python3

"""
recalculate_geom_index.py <resources_dir> <writable_dir> <generator_tool> [<designer_tool> <designer_params>]

Calculates geometry index for all mwms found inside the resource and writable directories.
Uses generator_tool for index calculation. After all, it runs designer_tool if provided.
"""

import os
import subprocess
import sys
from queue import Queue, Empty
from threading import Thread

WORKERS = 8
EXCLUDE_NAMES = {"WorldCoasts.mwm", "WorldCoasts_migrate.mwm"}

def find_all_mwms(data_path):
    """
    Recursively find all mwm files in the given directory, excluding specific filenames.
    """
    result = []
    for root, _, files in os.walk(data_path):
        for file in files:
            if file.endswith(".mwm") and file not in EXCLUDE_NAMES:
                result.append((file, root))
    return result

def process_mwm(generator_tool, task, error_queue):
    """
    Process an mwm file by calling the generator tool with appropriate arguments.
    """
    print(f"Processing {task[0]} in {task[1]}")
    try:
        subprocess.check_call([generator_tool,
                               f'--data_path={task[1]}',
                               f'--output={task[0][:-4]}',
                               '--generate_index=true',
                               '--intermediate_data_path=/tmp/'])
    except subprocess.CalledProcessError as e:
        error_queue.put(f"Error processing {task[0]}: {str(e)}")

def parallel_worker(tasks, generator_tool, error_queue):
    """
    Worker function for parallel processing. It fetches tasks from the queue and processes them.
    """
    while True:
        try:
            task = tasks.get_nowait()
        except Empty:
            print("No more tasks in queue. Worker exiting.")
            return
        process_mwm(generator_tool, task, error_queue)
        tasks.task_done()

if __name__ == "__main__":

    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <resources_dir> <writable_dir> <generator_tool> [<designer_tool> <designer_params>]")
        sys.exit(1)

    mwms = find_all_mwms(sys.argv[1])
    if sys.argv[2] != sys.argv[1]:
        mwms.extend(find_all_mwms(sys.argv[2]))

    tasks = Queue()
    error_queue = Queue()

    for task in mwms:
        tasks.put(task)

    for i in range(WORKERS):
        t = Thread(target=parallel_worker, args=(tasks, sys.argv[3], error_queue))
        t.daemon = True
        t.start()

    tasks.join()  # Block until all tasks are completed
    print("Processing done.")

    if len(sys.argv) > 4:
        print("Starting designer tool...")
        subprocess.Popen(sys.argv[4:])

    if not error_queue.empty():
        print("Errors occurred during processing:")
        while not error_queue.empty():
            error = error_queue.get()
            print(error)
        sys.exit(1)
