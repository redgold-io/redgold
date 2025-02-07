from tools.commands import ai_working_dir


def read_file_tooldef():
    return {
        "name": "read_file",
        "description": """Read a file from disk and return its contents, 
        this will only affect paths in the agents working directory""",
        "input_schema": {
            "type": "object",
            "properties": {
                "filename": {
                    "type": "string",
                    "description": """The relative filename to read from disk, this should be a path matching 
                    the syntax of something you've been given by other queries."""
                },
                "starting_line": {
                    "type": "string",
                    "description": """The starting offset line to read a subset of the file"""
                },
                "ending_line": {
                    "type": "string",
                    "description": """The starting offset line to read a subset of the file"""
                },
            },
            "required": ["filename"]
        }
    }


def read_file(filename, starting_line=None, ending_line=None) -> list[str]:
    prefix = str(ai_working_dir())
    if not filename.startswith("/"):
        prefix += "/"
    filename = prefix + filename
    # if not filename.startswith(prefix):
    #     return ["Error: unauthorized access to file"]


    try:
        with open(filename, "r") as f:
            lines = f.readlines()
            processed_lines = []
            for i, line in enumerate(lines):
                line = f"{i+1}: {line}"
                if starting_line is not None and i < starting_line:
                    continue
                if ending_line is not None and i > ending_line:
                    break
                processed_lines.append(line)
            return processed_lines
    except Exception as e:
        return [f"Error: {e}"]

# print(str(ai_working_dir()))