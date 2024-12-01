import csv
from collections import defaultdict
import sys

if len(sys.argv) <= 1:
    print("""
       * * *

    This script turns a CSV file resulting from "translated strings" in the Google Sheet file into a strings.txt-formatted file.

    To use this script, create the translated strings using the Google Spreadsheet. Go to File -> Download as, and choose CSV. Only the currently open sheet will be exported.
    Run this script with the path to the downloaded file as a parameter. The formatted file will be printed to the console.
    Please note that the order of keys is not (yet) preserved.
       * * *
    """)
    sys.exit(2)

path = sys.argv[1]
resulting_dict = defaultdict(list)

try:
    with open(path, mode='r', encoding='utf-8') as infile:
        reader = csv.reader(infile)
        column_names = next(reader)

        for strings in reader:
            for i, string in enumerate(strings):
                if string:
                    resulting_dict[column_names[i]].append(string)
except (OSError, IOError) as e:
    print(f"Error opening file: {e}")
    sys.exit(1)

for key in column_names:
    if not key:
        continue

    translations = resulting_dict[key]
    print(f"  {key}")
    for translation in translations:
        print(f"    {translation}")

    print("")
