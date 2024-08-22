import os
import subprocess
from pathlib import Path


def get_git_diff():
    tooldef = {
        "name": "get_git_diff",
        "description": "This tool retrieves the current git diff of changes which have not yet been committed",
        "input_schema": {
            "type": "object",
            "properties": {
                "dummy": {
                    "type": "string",
                    "description": "No argument is required here"
                }
            }
        }
    }

    def get_git_diff_inner():
        p = Path.home().joinpath("ai/redgold")
        os.chdir(p)
        return subprocess.check_output(['git', 'diff', '--unified=0'], text=True).strip()

    return tooldef, get_git_diff_inner

# Example usage:
# tooldef, get_git_diff_func = get_git_diff()
# result = get_git_diff_func({'base_branch': 'main'})
# print(result)

def main():
    d = get_git_diff()[1]()
    print(d)

if __name__ == "__main__":
    main()