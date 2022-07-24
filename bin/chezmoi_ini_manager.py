#!/usr/bin/env python3
"""
Chezmoi modify_ script helper to handle ini files (mostly from KDE)

Needs Python 3.10 or later.
"""

import enum
import json
import re
import argparse
import sys
from dataclasses import dataclass
from functools import partial
from pathlib import Path
from sys import stdin, stderr
from typing import Generator, TextIO, Callable, Literal, Optional, Iterable, Any

# This regex is silly because of stuff like "[Colors:Header][Inactive]" in kde configs
_RE_SECTION = re.compile(r"\[(?P<name>[^\n]+)\].*")

# Identifier for things outside sections. We could use None, but that wouldn't allow easily ignoring.
OUTSIDE_SECTION = "<NO_SECTION>"


class LineType(enum.Enum):
    """Type of line, returned by low level parser"""

    # Comment, blank line or otherwise uninteresting
    Comment = enum.auto()
    # A section header
    SectionHeader = enum.auto()
    # A key value pair
    KeyValue = enum.auto()


# Return type of load_ini. Written to be used together with match.
LineState = (
    tuple[Literal[LineType.Comment], str]
    | tuple[Literal[LineType.SectionHeader], str, str]
    | tuple[Literal[LineType.KeyValue], str, str, str, str]
)

# Raw line, value
KeyLineState = tuple[str, str]

# A transform takes two lines and merges them
# Args: section, key, source data, target data
Transform = Callable[[str, str, Optional[KeyLineState], Optional[KeyLineState]], str]


@dataclass
class Mutations:
    """Collects all the ways we can ignore, transform etc (mutations)"""

    ignore_sections: set[str]
    ignore_keys: set[tuple[str, str]]
    ignore_regexes: list[tuple[re.Pattern, re.Pattern]]
    transforms: dict[tuple[str, str], Transform]


class ParseException(Exception):
    """Exception used to associate extra metadata with parse errors to help debugging"""

    pass


def load_ini(file: TextIO | Iterable[str]) -> Generator[LineState, None, None]:
    """
    This function parses an INI. Intended to be combined with a match statement

    Returns one of:
    * Comment, line
    * SectionHeader, line, section
    * KeyValue, line, section, key, value
    """
    section = OUTSIDE_SECTION
    try:
        for line in file:
            stripped_line = line.strip()
            if not stripped_line:
                yield LineType.Comment, line
            elif line.startswith(";") or line.startswith("#"):
                yield LineType.Comment, line
            elif match := _RE_SECTION.match(line):
                section = match.group("name")
                yield LineType.SectionHeader, line, section
            else:
                key, value = line.split("=", maxsplit=1)
                yield LineType.KeyValue, line, section, key.strip(), value.strip()
    except Exception as e:
        raise ParseException(f"Error while processing line {line}") from e


# Section -> Raw Line
SourceSections = dict[str, str]
# Section -> Key -> (Raw line, value)
SourceKvs = dict[str, dict[str, tuple[str, str]]]


def load_into_dict(file: TextIO | Iterable[str]) -> tuple[SourceSections, SourceKvs]:
    """
    Load the file into a dictionary

    Returns two dicts:
    * Section -> Raw line
    * Section -> Key -> (Raw line, value)
    """
    sections = {}
    kvs = {}
    for data in load_ini(file):
        try:
            match data:
                case (LineType.Comment, _):
                    pass
                case (LineType.SectionHeader, line, section):
                    sections[section] = line
                    kvs[section] = {}
                case (LineType.KeyValue, line, section, key, value):
                    if section is OUTSIDE_SECTION and OUTSIDE_SECTION not in kvs:
                        # dolphinrc (and some other programs) has a key before the first section. Blergh.
                        sections[OUTSIDE_SECTION] = None
                        kvs[OUTSIDE_SECTION] = {}
                    kvs[section][key] = line, value
        except Exception as e:
            raise ParseException(f"Error while processing data {data}") from e
    return sections, kvs


def ignored_re(section: str, key: str, mutations: Mutations):
    """Check if section + key is an ignored regex"""
    for re_section, re_key in mutations.ignore_regexes:
        if re_section.match(section) and re_key.match(key):
            return True
    return False


def is_section_ignored(section: str, mutations: Mutations) -> bool:
    """Check if section is ignored"""
    return section in mutations.ignore_sections


def is_key_ignored(section: str, key: str, mutations: Mutations) -> bool:
    """Check if key is ignored. Does not handle transformations"""
    if is_section_ignored(section, mutations):
        return True
    if (section, key) in mutations.ignore_keys:
        return True
    if ignored_re(section, key, mutations):
        return True
    return False


def process_target(
    file: TextIO | Iterable[str],
    source_sections: SourceSections,
    source_kvs: SourceKvs,
    mutations: Mutations,
) -> Generator[str, None, None]:
    """Process the target file, merging the state of source and target files"""
    seen_sections = set()
    seen_keys = set()
    cur_section = OUTSIDE_SECTION
    for data in load_ini(file):
        match data:
            case (LineType.Comment, line):
                yield line
            case (LineType.SectionHeader, line, section):
                # Track state to deal with keys existing in source but not target
                if cur_section in source_sections.keys() and not is_section_ignored(
                    cur_section, mutations
                ):
                    unseen_keys = set(source_kvs[cur_section].keys()).difference(
                        seen_keys
                    )
                    for k in sorted(unseen_keys):
                        if not is_key_ignored(cur_section, k, mutations):
                            if (cur_section, k) in mutations.transforms:
                                yield mutations.transforms[(cur_section, k)](
                                    cur_section, k, source_kvs[cur_section][k], None
                                )
                            else:
                                yield source_kvs[cur_section][k][0]
                seen_sections.add(section)
                seen_keys = set()
                cur_section = section
                # Back to handling things that exist in the target
                if is_section_ignored(section, mutations):
                    yield line
                elif section in source_sections:
                    yield source_sections[section]
            case (LineType.KeyValue, line, section, key, value):
                # Keep track of seen keys so we can later on deal with things
                # missing in target but found in source.
                seen_keys.add(key)
                # Back to handling things that exist in the target
                if is_key_ignored(section, key, mutations):
                    yield line
                elif (section, key) in mutations.transforms:
                    src_data = None
                    if section in source_kvs and key in source_kvs[section]:
                        src_data = source_kvs[section][key]
                    yield mutations.transforms[(section, key)](
                        section, key, src_data, (line, value)
                    )
                elif section in source_kvs and key in source_kvs[section]:
                    yield source_kvs[section][key][0]
    # Handle extra sections in source state
    for section in sorted(set(source_sections.keys()).difference(seen_sections)):
        # Before the first section. Special case handled above in case LineType.SectionHeader
        if section is OUTSIDE_SECTION:
            continue
        if is_section_ignored(section, mutations):
            continue
        yield source_sections[section]
        for key, (line, _) in sorted(source_kvs[section].items()):
            if not is_key_ignored(section, key, mutations):
                if (section, key) in mutations.transforms:
                    yield mutations.transforms[(section, key)](
                        section, key, source_kvs[section][key], None
                    )
                else:
                    yield line


def transform_unsorted_lists(
    section: str,
    key: str,
    source: Optional[KeyLineState],
    target: Optional[KeyLineState],
    *,
    separator: str,
) -> str:
    """
    Compare the value as an unsorted list.

    Useful because Konversation likes to reorder lists.
    Args: {"separator": separating character}
    Example args: {"separator": ","}
    """
    # Deal with case of line in just target or source
    if target is None:
        return source[0]
    if source is None:
        return target[0]
    ss = set(source[1].split(separator))
    ts = set(target[1].split(separator))
    if ss != ts:
        return source[0]
    else:
        return target[0]


def transform_kde_media_shortcut(
    section: str,
    key: str,
    source: Optional[KeyLineState],
    target: Optional[KeyLineState],
) -> str:
    """
    Specialised transform to handle KDE changing certain media shortcuts back and forth between formats like:
    ```
    playmedia=none,,Play media playback
    playmedia=none,none,Play media playback
    ```

    Args: {}
    Example args: {}
    """
    # Deal with case of line in just target or source
    if target is None:
        return source[0]
    if source is None:
        return target[0]
    src_split = source[1].split(",")
    tgt_split = target[1].split(",")
    if (
        src_split[0] == tgt_split[0]
        and src_split[2] == tgt_split[2]
        and src_split[1] in ("", "none")
        and tgt_split[1] in ("", "none")
    ):
        return target[0]
    else:
        return source[0]


def transform_value_keyring(
    section: str,
    key: str,
    source: Optional[KeyLineState],
    target: Optional[KeyLineState],
    *,
    service: str,
    username: str,
):
    """
    Get value from keyring (kwallet or secret service). Useful for passwords
    etc that you do not want in your dotfiles repo, but sync via some more
    secure manner.

    Note! Requires the python library keyring.

    Args: {
        "service": str,
        "username": str
    }
    Example args: {
        "service": "system"
        "username": "konversation-login"
    }
    """
    import keyring
    import keyring.errors

    try:
        password = keyring.get_password(service, username)
        # TODO: Detect formatting in the target (space around = or not).
        #       KDE don't use spaces there, but other software might.
        return f"{key}={password}\n"
    except keyring.errors.KeyringError as e:
        print(f"ERROR: Keyring error: {e}", file=stderr)
        # Try to pull the value from the target instead
        if target is not None:
            return target[0]
        return f"{key}=<KEYRING ERROR>\n"


transform_registry = {
    "kde_media_shortcut": transform_kde_media_shortcut,
    "keyring": transform_value_keyring,
    "unsorted_list": transform_unsorted_lists,
}


class TransformHelp(argparse.Action):
    """Argparse action to print help about transformations"""

    def __int__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    def __call__(self, *args, **kwargs):
        """Print help for transformations that can be applied"""
        print("Valid transforms:")
        for name, func in sorted(transform_registry.items()):
            print(f"* {name}")
            for line in func.__doc__.strip().split("\n"):  # type: str
                print(f"  {line.removeprefix('    ')}")
        sys.exit(0)


def main():

    parser = argparse.ArgumentParser(
        description=sys.modules[__name__].__doc__,
        epilog=f"Note! If a key appears before the first section use the value {OUTSIDE_SECTION} to refer to it.",
    )
    parser.add_argument(
        "-is",
        "--ignore-section",
        action="append",
        default=[],
        help="ignore specific section",
    )
    parser.add_argument(
        "-ik",
        "--ignore-key",
        action="append",
        nargs=2,
        default=[],
        help="ignore specific key, format is section and key",
    )
    parser.add_argument(
        "-ikr",
        "--ignore-key-re",
        action="append",
        nargs=2,
        default=[],
        help="ignore specific key, format is two regex, one for section and one for key",
    )
    parser.add_argument(
        "-tk",
        "--transform-key",
        action="append",
        nargs=4,
        default=[],
        help="apply transformation to a key, format is transform, section, key, transform args (json dict).",
    )
    parser.add_argument(
        "--transform-list",
        action=TransformHelp,
        nargs=0,
        help="show list of transforms with descriptions and exit",
    )
    parser.add_argument(
        "-s",
        "--source",
        action="store",
        type=Path,
        required=True,
        help="source file path",
    )
    args = parser.parse_args()

    if args.source is None:
        print("Error: Source file (-s/--source) is required", file=stderr)
        sys.exit(1)

    # print(args)

    transforms = {}
    for transform, section, key, targs in args.transform_key:
        if transform not in transform_registry:
            print(f"Unknown transform {transform}", file=stderr)
            sys.exit(1)
        transform_args: dict[str, Any] = json.loads(targs)
        transforms[(section, key)] = partial(
            transform_registry[transform], **transform_args
        )

    mutations = Mutations(
        ignore_sections=set(args.ignore_section),
        ignore_keys=set(tuple(e) for e in args.ignore_key),
        ignore_regexes=[(re.compile(a), re.compile(b)) for a, b in args.ignore_key_re],
        transforms=transforms,
    )
    # print(mutations)

    with args.source.open(mode="rt") as source_file:  # type: TextIO
        source_sections, source_kvs = load_into_dict(source_file)

    for line in process_target(
        stdin,
        source_sections=source_sections,
        source_kvs=source_kvs,
        mutations=mutations,
    ):
        print(line, end="")


if __name__ == "__main__":
    main()
