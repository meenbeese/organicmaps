#!/usr/bin/env python3

"""
This script checks for consistency between categories defined in:
1. `displayed_categories.cpp`: A C++ source file containing category keys.
2. `categories.txt`: A data file defining category details.
3. `strings.txt`: A data file containing translations of category strings.

The script:
- Parses category keys from the C++ file.
- Loads corresponding data from `categories.txt` and `strings.txt`.
- Compares the parsed data for inconsistencies in definitions and translations.
- Prints detailed messages if inconsistencies are found.

It exits with:
- `0` on success (all categories are consistent),
- `1` on failure (inconsistencies detected).
"""

import os
import re
from omim_parsers import CategoriesParser, StringsParser

ROOT = os.path.dirname(os.path.abspath(__file__))
OMIM_ROOT = os.path.join(ROOT, '..', '..', '..')
CPP_CATEGORIES_FILENAME = os.path.join(OMIM_ROOT, 'search', 'displayed_categories.cpp')
CATEGORIES_FILENAME = os.path.join(OMIM_ROOT, 'data', 'categories.txt')
STRINGS_FILENAME = os.path.join(OMIM_ROOT, 'data', 'strings', 'strings.txt')
CATEGORIES_MATCHER = re.compile(r"m_keys = \{(.*?)};", re.DOTALL)


def load_categories_from_cpp(filename: str):
    with open(filename, 'r', encoding='utf-8') as file:
        raw_categories = file.read()

    match = CATEGORIES_MATCHER.search(raw_categories)
    if match:
        cpp_categories = match.group(1).split(', ')
        # Remove quotes
        cpp_categories = [cat.strip('"') for cat in cpp_categories]
        return cpp_categories
    return []


def compare_categories(string_cats: dict, search_cats: dict) -> bool:
    inconsistent_strings = {}

    for category_name, category in string_cats.items():
        if category_name not in search_cats:
            print(f"Category '{category_name}' not found in categories.txt")
            continue

        for lang, translation in category.items():
            if lang in search_cats[category_name]:
                if translation not in search_cats[category_name][lang]:
                    not_found_cats_list = search_cats[category_name][lang]
                    inconsistent_strings.setdefault(category_name, {})[lang] = (
                        translation, not_found_cats_list
                    )

    for name, languages in inconsistent_strings.items():
        print(f"\nInconsistent category \"{name}\"")
        for lang, values in languages.items():
            string_value, category_value = values
            print(f"\t{lang} : \"{string_value}\" is not matched by {category_value}")

    return not inconsistent_strings


def check_search_categories_consistent() -> int:
    cpp_categories = load_categories_from_cpp(CPP_CATEGORIES_FILENAME)
    if not cpp_categories:
        print("Error: No categories found in displayed_categories.cpp.")
        return 1

    categories_txt_parser = CategoriesParser(cpp_categories)
    strings_txt_parser = StringsParser(cpp_categories)

    search_categories = categories_txt_parser.parse_file(CATEGORIES_FILENAME)
    string_categories = strings_txt_parser.parse_file(STRINGS_FILENAME)

    if compare_categories(string_categories, search_categories):
        print("Success: All categories are consistent.")
        return 0
    else:
        print("Failure: Inconsistencies found in category definitions.")
        return 1


if __name__ == "__main__":
    exit(check_search_categories_consistent())
