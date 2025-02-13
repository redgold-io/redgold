import os
from tools.commands import ai_working_dir
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

# print(str(ai_working_dir()))