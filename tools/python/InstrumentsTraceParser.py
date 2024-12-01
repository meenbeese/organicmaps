#!/usr/bin/env python3

import struct
import sys
import numpy as np


class Analyzer:
    """
    The binary format is
        time since the beginning of the measurement : double
        unknown and irrelevant field : double
        momentary consumption calculated for the current time segment : double
    """

    def __init__(self):
        self.duration = 0.0
        self.consumption = []
        self.mean = 0.0
        self.std = 0.0
        self.avg = 0.0
        self.averages = []

    def read_file(self, file_path):
        binary = bytearray()
        try:
            with open(file_path, "rb") as f:
                binary = bytearray(f.read())
        except IOError as e:
            print(f"Error reading file {file_path}: {e}")
            sys.exit(1)

        # Each entry is 24 bytes (3 doubles: 8 bytes each)
        for i in range(0, len(binary) - 24, 24):
            res = struct.unpack(">ddd", binary[i:i + 24])

            current_duration = res[0]
            if current_duration <= self.duration:
                print(f"Unexpected elapsed time value, lower than the previous one in {file_path}.")
                sys.exit(2)

            current_consumption = res[2]
            self.averages.append(current_consumption / (current_duration - self.duration))
            self.duration = current_duration
            self.consumption.append(current_consumption)

        self.calculate_stats()

    def calculate_stats(self):
        if self.averages:
            self.mean = np.mean(self.averages)
            self.std = np.std(self.averages)
        if self.duration > 0:
            self.avg = sum(self.consumption) / self.duration


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: {} <file1> [file2 ...]".format(sys.argv[0]))
        sys.exit(1)

    for file_path in sys.argv[1:]:
        analyzer = Analyzer()
        analyzer.read_file(file_path)
        print(f"{file_path}\n\tavg:  {analyzer.avg}\n\tmean: {analyzer.mean}\n\tstd:  {analyzer.std}")
