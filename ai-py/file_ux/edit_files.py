from typing import Optional

from commands import ai_working_dir


def edit_file_replace_lines_tooldef():
    return {
        "name": "edit_file_replace_lines",
        "description": """This tool allows you to edit an existing file, by replacing lines indexed by starting and ending lines.""",
        "input_schema": {
            "type": "object",
            "properties": {
                "filename": {
                    "type": "string",
                    "description": """The text to search for, for example, what a programmer"""
                },
                "starting_line": {
                    "type": "string",
                    "description": """The (inclusive) 1-indexed starting offset line to begin replacing a subset of the file"""
                },
                "ending_line": {
                    "type": "string",
                    "description": """The (inclusive) 1-indexed ending offset line to begin replacing a subset of the file"""
                },
                "replacement_lines": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": """Lines to replace"""
                },
            },
            "required": ["filename", "starting_line", "ending_line"]
        }
    }


def edit_file(filename: str, starting_line: Optional[str], ending_line: Optional[str], replacement_lines: Optional[list[str]]=None):
    if replacement_lines is None:
        replacement_lines = []

    prefix = str(ai_working_dir())
    if not filename.startswith(prefix):
        return "Error: unauthorized access to file"
    with open(filename, "r") as f:
        lines = f.readlines()
        num_lines = len(lines)
        max_idx = num_lines - 1
        if starting_line is None or starting_line < 0:
            starting_line = 0
        else:
            starting_line = int(starting_line) - 1
        if ending_line is None or ending_line > max_idx:
            ending_line = max_idx
        else:
            ending_line = int(ending_line)

        processed_lines = []
        added = False
        for i, line in enumerate(lines):
            if i <= starting_line or i >= ending_line:
                processed_lines.append(line)
            else:
                if not added:
                    for replacement_line in replacement_lines:
                        processed_lines.append(replacement_line)
                    added = True

            processed_lines.append(line)


# print(str(ai_working_dir()))
