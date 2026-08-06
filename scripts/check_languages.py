"""Check that the hardcoded LANGUAGES map in src/main.rs covers every
language code returned by the DeepL languages endpoint."""

import argparse
import os
import re
import sys
from pathlib import Path
from urllib import error, request

DEEPL_LANGUAGES_URL = "https://api.deepl.com/v3/languages?resource=translate_text"
SOURCE_FILE = Path(__file__).resolve().parent.parent / "src" / "main.rs"
LANGUAGES_FN_RE = re.compile(r"fn languages\(\).*?\{(.*?)\n\}", re.DOTALL)
TUPLE_RE = re.compile(r'\("([^"]+)",\s*"([^"]*)"\)')


def fetch_deepl_languages(api_key: str) -> dict[str, dict]:
    req = request.Request(
        DEEPL_LANGUAGES_URL,
        headers={
            "Authorization": f"DeepL-Auth-Key {api_key}",
            "User-Agent": "babello-language-check/1.0",
        },
    )
    try:
        with request.urlopen(req) as response:
            import json

            data = json.load(response)
    except error.HTTPError as e:
        sys.exit(f"DeepL API request failed: {e.code} {e.reason}")
    except error.URLError as e:
        sys.exit(f"DeepL API request failed: {e.reason}")

    return {entry["lang"].upper(): entry["name"] for entry in data}


def extract_dict_codes(source_file: Path) -> dict[str, str]:
    text = source_file.read_text()
    match = LANGUAGES_FN_RE.search(text)
    if not match:
        sys.exit(f"Could not find a `languages()` function body in {source_file}")

    return {code.upper(): name for code, name in TUPLE_RE.findall(match.group(1))}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source",
        type=Path,
        default=SOURCE_FILE,
        help="Path to the Rust source file containing the languages() map",
    )
    args = parser.parse_args()

    api_key = os.environ.get("DEEPL_API_KEY")
    if not api_key:
        sys.exit("DEEPL_API_KEY is not set")

    api_codes_and_names = fetch_deepl_languages(api_key)
    dict_codes_and_names = extract_dict_codes(args.source)

    missing = sorted(set(api_codes_and_names) - set(dict_codes_and_names))
    extra = sorted(set(dict_codes_and_names) - set(api_codes_and_names))

    if missing:
        print(f"Missing from {args.source} ({len(missing)}):")
        for code in missing:
            print(f'  ("{code}", "{api_codes_and_names[code]}") ')
    else:
        print(f"{args.source} contains every language returned by the API.")

    if extra:
        print(f"\nIn {args.source} but not returned by the API ({len(extra)}):")
        for code in extra:
            print(f'  ("{code}", "{api_codes_and_names[code]}") ')

    sys.exit(1 if missing else 0)


if __name__ == "__main__":
    main()
