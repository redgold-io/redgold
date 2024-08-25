import os
import subprocess
from pathlib import Path


def git_add_file():
    tooldef = {
        "name": "git_add_file",
        "description": "This tool adds a specified file to the Git staging area",
        "input_schema": {
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The path of the file to be added, relative to the repository root"
                }
            },
            "required": ["file_path"]
        }
    }

    def git_add_file_inner(args):
        file_path = args.get('file_path')
        if not file_path:
            raise ValueError("file_path is required")

        p = Path.home().joinpath("ai/redgold")
        os.chdir(p)

        try:
            subprocess.run(['git', 'add', file_path], check=True, text=True, capture_output=True)
            return f"Successfully added {file_path} to the staging area."
        except subprocess.CalledProcessError as e:
            return f"Error adding file: {e.stderr}"

    return tooldef, git_add_file_inner


# Example usage:
# tooldef, git_add_file_func = git_add_file()
# result = git_add_file_func({'file_path': 'path/to/your/file.txt'})
# print(result)

def main():
    _, git_add_file_func = git_add_file()
    result = git_add_file_func({'file_path': 'data/src/error_conversion.rs'})
    print(result)


if __name__ == "__main__":
    main()