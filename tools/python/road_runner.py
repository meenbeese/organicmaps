#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys
import json
from threading import Thread
from queue import Queue
from urllib.request import urlopen

'''
World road map generation script.
It takes city points from omim intermediate data and calculates roads between them.
Afterwards, it stores road features OSM way IDs into a CSV text file.
'''

ROAD_DELTA = 200
WORKERS = 16


def get_way_ids(point1, point2, server):
    url = f"http://{server}/wayid?z=18&loc={point1[0]},{point1[1]}&loc={point2[0]},{point2[1]}"
    try:
        with urlopen(url) as response:
            data = json.load(response)
            return data.get("way_ids", [])
    except Exception as e:
        print(f"Failed to fetch way IDs for {point1}, {point2}: {e}")
        return []


def each_to_each(points):
    result = []
    for i in range(len(points)):
        for j in range(i + 1, len(points)):
            result.append((points[i], points[j]))
    return result


def load_towns(path):
    towns = []
    if not os.path.isfile(path):
        print("WARNING! File with towns not found!")
        return towns

    with open(path, "r") as f:
        for line in f:
            data = line.strip().split(";")
            is_capital = data[3].lower() == "t"
            towns.append((float(data[0]), float(data[1]), is_capital))
    return towns


def parallel_worker(tasks, capitals_list, towns_list, server):
    """Thread worker function to process tasks in parallel."""
    while True:
        task = tasks.get()
        if tasks.qsize() % 1000 == 0:
            print(f"Tasks remaining: {tasks.qsize()}")

        way_ids = get_way_ids(task[0], task[1], server)
        for way_id in way_ids:
            if task[0][2] and task[1][2]:  # Both are capitals
                capitals_list.add(way_id)
            else:
                towns_list.add(way_id)
        tasks.task_done()


def main():
    if len(sys.argv) < 3:
        print("Usage: road_runner.py <intermediate_dir> <osrm_addr>")
        sys.exit(1)

    intermediate_dir = sys.argv[1]
    server_addr = sys.argv[2]

    if not os.path.isdir(intermediate_dir):
        print(f"{intermediate_dir} is not a directory!")
        sys.exit(1)

    towns = load_towns(os.path.join(intermediate_dir, "towns.csv"))
    print(f"Loaded {len(towns)} towns.")

    if not towns:
        print("No towns found. Exiting.")
        sys.exit(1)

    tasks = each_to_each(towns)
    filtered_tasks = [(p1, p2) for p1, p2 in tasks if
                      p1[2] and p2[2] or (p1[0] - p2[0]) ** 2 + (p1[1] - p2[1]) ** 2 < ROAD_DELTA]

    if not filtered_tasks:
        print("No valid tasks to process. Exiting.")
        sys.exit(1)

    try:
        get_way_ids(filtered_tasks[0][0], filtered_tasks[0][1], server_addr)
    except Exception as e:
        print(f"Unable to connect to the server: {server_addr}\nError: {e}")
        sys.exit(1)

    qtasks = Queue()
    capitals_list = set()
    towns_list = set()

    for _ in range(WORKERS):
        t = Thread(target=parallel_worker, args=(qtasks, capitals_list, towns_list, server_addr))
        t.daemon = True
        t.start()

    for task in filtered_tasks:
        qtasks.put(task)

    qtasks.join()

    with open(os.path.join(intermediate_dir, "ways.csv"), "w") as f:
        for way_id in capitals_list:
            f.write(f"{way_id};world_level\n")
        for way_id in towns_list:
            if way_id not in capitals_list:
                f.write(f"{way_id};world_towns_level\n")

    print("Processing complete.")


if __name__ == "__main__":
    main()
