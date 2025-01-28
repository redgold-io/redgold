import inspect
import os
import subprocess
from functools import wraps
from pathlib import Path
from typing import Callable


def tool(name: str):
    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs)

        # Extract function docstring for description
        full_doc = inspect.getdoc(func) or ""
        description = full_doc.split('\n\n')[0]  # Take only the first paragraph

        # Extract argument descriptions from docstring
        arg_spec = inspect.getfullargspec(func)
        arg_descriptions = {}
        if full_doc:
            param_docs = full_doc.split('\n\n')[-1]  # Take the last paragraph for params
            for line in param_docs.split('\n'):
                line = line.strip()
                if line.startswith(':param'):
                    param, desc = line.split(':', 2)[1:]
                    arg_descriptions[param.strip()] = desc.strip()

        # Create input schema
        input_schema = {
            "type": "object",
            "properties": {}
        }
        for arg in arg_spec.args:
            input_schema["properties"][arg] = {
                "type": "string",
                "description": arg_descriptions.get(arg, f"Description for {arg}")
            }

        # Attach metadata to the wrapper function
        wrapper.tooldef = {
            "name": name,
            "description": description,
            "input_schema": input_schema
        }

        return wrapper
    return decorator

@tool("get_git_diff")
def get_git_diff(dummy: str = "") -> str:
    """This tool retrieves the current git diff of changes which have not yet been committed.

    :param dummy: No argument is required here
    """
    p = Path.home().joinpath("ai/redgold")
    os.chdir(p)
    return subprocess.check_output(['git', 'diff', '--unified=0'], text=True).strip()

def main():
    # Example usage
    result = get_git_diff()
    print(result)

    # Access tool metadata
    print(get_git_diff.tooldef)

if __name__ == "__main__":
    main()