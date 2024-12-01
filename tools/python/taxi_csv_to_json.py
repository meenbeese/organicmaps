#!/usr/bin/env python3
# coding: utf-8

from argparse import ArgumentParser
from collections import defaultdict
import json
import logging
import sys


def deserialize_places(src):
    lines = src.splitlines()
    # Skip header.
    lines = lines[1:]
    countries = defaultdict(list)
    mwms = []

    try:
        for l in lines:
            cells = l.split('\t')

            if len(cells) < 5 and not cells[0]:
                logging.error(f"Country cell is empty. Incorrect line: {cells}")
                sys.exit(1)

            # Add full country.
            if len(cells) < 3:
                countries[cells[0]] = []
            # Add city of the country.
            elif len(cells) < 5:
                countries[cells[0]].append(cells[2])
            # Add mwm.
            elif len(cells) >= 5:
                mwms.append(cells[4])
    except IndexError as e:
        logging.error(f"The structure of src file is incorrect. Exception: {e}")
        sys.exit(1)

    return countries, mwms


def convert(src_path, dst_path):
    try:
        with open(src_path, "r", encoding='utf-8') as f:
            src = f.read()
    except (OSError, IOError) as e:
        logging.error(f"Cannot read src file {src_path}. Error: {e}")
        return

    countries, mwms = deserialize_places(src)

    # Carcass of the result.
    result = {
        "enabled": {"countries": [], "mwms": []},
        "disabled": {"countries": [], "mwms": []}
    }

    for country, cities in countries.items():
        result["enabled"]["countries"].append({
            "id": country,
            "cities": cities
        })

    result["enabled"]["mwms"] = mwms

    try:
        with open(dst_path, "w", encoding='utf-8') as f:
            json.dump(result, f, indent=2, sort_keys=True)
    except (OSError, IOError) as e:
        logging.error(f"Cannot write result into dst file {dst_path}. Error: {e}")
        return


def process_options():
    parser = ArgumentParser(description='Load taxi file in CSV format and convert it into JSON')

    parser.add_argument("--src", type=str, dest="src", help="Path to the CSV file", required=True)
    parser.add_argument("--dst", type=str, dest="dst", help="Path to the JSON file", required=True)

    return parser.parse_args()


def main():
    options = process_options()
    if options:
        convert(options.src, options.dst)


if __name__ == "__main__":
    main()
