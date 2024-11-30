import re
from typing import Optional, Tuple, Dict, List

LANGUAGES = [
    "af", "ar", "be", "bg", "ca", "cs", "da", "de", "el", "en", "en-GB", "es", "es-MX", "et",
    "eu", "fa", "fi", "fr", "fr-CA", "he", "hi", "hu", "id", "it", "ja", "ko", "lt", "mr", "nb",
    "nl", "pl", "pt", "pt-BR", "ro", "ru", "sk", "sv", "sw", "th", "tr", "uk", "vi", "zh-Hans", "zh-Hant"
]


class AbstractParser:
    def __init__(self, keys: List[str]):
        self.keys = keys

    def parse_line(self, line: str) -> Optional[Tuple[str, str]]:
        raise NotImplementedError("You must implement parse_line.")

    def match_category(self, line: str, result: Dict[str, Dict]):
        category_match = self.category().search(line)
        if category_match:
            category = category_match.group(1)
            if category in self.keys:
                if category not in result:
                    result[category] = {}

    def parse_file(self, filename: str) -> Dict[str, Dict]:
        current_string = None
        result = {}

        with open(filename, "r", encoding="utf-8") as file:
            for line in file:
                line = line.strip()
                if self.should_exclude_line(line):
                    continue

                # If the line is empty, reset the current string
                if not line:
                    current_string = None
                    continue

                # Try to match a new category
                if current_string is None:
                    self.match_category(line, result)

                parsed = self.parse_line(line)
                if parsed and current_string is not None:
                    lang, translation = parsed
                    result[current_string][lang] = translation

        return result

    def category(self) -> re.Pattern:
        raise NotImplementedError("You must implement category.")

    def should_exclude_line(self, line: str) -> bool:
        return False


class CategoriesParser(AbstractParser):
    def parse_line(self, line: str) -> Optional[Tuple[str, List[str]]]:
        line_match = re.match(r"^([^:]+):(\S+)$", line)
        if line_match:
            lang = line_match.group(1).strip()
            if lang in LANGUAGES:
                translation = line_match.group(2).strip()
                synonyms = []
                for token in translation.split("|"):
                    token_match = re.match(r"\d?\^?(.*)$", token)
                    if token_match:
                        synonyms.append(token_match.group(1))
                return lang, synonyms
        return None

    def should_exclude_line(self, line: str) -> bool:
        return line.startswith("#")

    def category(self) -> re.Pattern:
        return re.compile(r"^@([A-Za-z0-9]+)$")


class StringsParser(AbstractParser):
    def parse_line(self, line: str) -> Optional[Tuple[str, str]]:
        line_match = re.match(r"^([^=]+)=(.*)$", line)
        if line_match:
            lang = line_match.group(1).strip()
            if lang in LANGUAGES:
                return lang, line_match.group(2).strip()
        return None

    def category(self) -> re.Pattern:
        return re.compile(r"^\[(.+)]")
