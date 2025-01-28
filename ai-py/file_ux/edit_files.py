import os
from commands import ai_working_dir
from typing import Optional


def edit_file_replace_lines_tooldef():
    return {
        "name": "edit_file_replace_lines",
        "description": """This tool allows you to edit an existing file, by replacing lines indexed by starting and ending lines.""",
        "input_schema": {
            "type": "object",
            "properties": {
                "filename": {
                    "type": "string",
                    "description": """The filename to edit, this must be a relative path within the AI working directory"""
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
    if not filename.startswith("/"):
        prefix += "/"
    filename = prefix + filename


    print(f"Editing file {filename}")
    # check if the file exists, if its parents do not exist, create them as directories
    if not os.path.exists(filename):
        print(f"File {filename} does not exist, creating it")
        parent_dir = os.path.dirname(filename)
        if not os.path.exists(parent_dir):
            os.makedirs(parent_dir)
        with open(filename, "w") as f:
            for line in replacement_lines:
                f.write(line)
            return

    processed_lines = []

    with open(filename, "r") as f:
        lines = f.readlines()
        num_lines = len(lines)
        max_idx = num_lines - 1
        if starting_line is None or int(starting_line) < 0:
            starting_line = 0
        else:
            starting_line = int(starting_line) - 1
        if ending_line is None or int(ending_line) > max_idx:
            ending_line = max_idx
        else:
            ending_line = int(ending_line)

        added = False
        print(f"Editing file with: {starting_line} to {ending_line} with num lines: {num_lines}")
        print(f"L{starting_line}: {lines[starting_line]}")
        for i, line in enumerate(lines):
            if i < starting_line or i > ending_line:
                processed_lines.append(line)
            else:
                if not added:
                    for replacement_line in replacement_lines:
                        processed_lines.append(replacement_line)
                    added = True

    with open(filename, "w") as f:
        for line in processed_lines:
            f.write(line)

# print(str(ai_working_dir()))


if __name__ == "__main__":
    input = {'filename': 'src/node.rs', 'starting_line': '117', 'ending_line': '117', 'replacement_lines': ''}
    edit_file(**input)