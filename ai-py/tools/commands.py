from typing import Annotated, List
from langchain.tools import tool
import os
import subprocess


def run_cargo_check(directory):
    original_dir = os.getcwd()

    try:
        os.chdir(directory)

        result = subprocess.run(['cargo', 'check'],
                                capture_output=True,
                                text=True)

        if result.returncode == 0:
            print(f"Cargo check completed successfully in {directory}")
            return []
        else:
            output = result.stderr
            lines = output.split("\n")
            errors = []
            max_idx = len(lines) - 1
            for i, line in enumerate(lines):
                if line.startswith("error:"):
                    next_30 = i+60
                    if next_30 > max_idx:
                        next_30 = max_idx
                    err_msg = "\n".join(lines[i:next_30])
                    errors.append(err_msg)

            #
            # # Use regex to find error messages
            # error_pattern = re.compile(r'error.*?(?=\n\n|\Z)', re.DOTALL)
            # errors = error_pattern.findall(output)
            #
            # print(f"Cargo check failed with {len(errors)} errors in {directory}:")
            # for error in errors:
            #     print(f"----------\n{error.strip()}\n----------")

            return errors
    finally:
        os.chdir(original_dir)


def redgold_cargo_rust_compile():
# Specify the directory
    cargo_dir = ai_working_dir()
# Run the function
    errors = run_cargo_check(cargo_dir)
    return errors


def ai_working_dir():
    return os.path.expanduser("~/ai/redgold")


def redgold_cargo_rust_compile_claude_tooldef():
    return {
        "name": "redgold_cargo_rust_compile",
        "description": """Compile the current Redgold Rust project stored in ~/ai/redgold. 
        This is for your 'active' project which you are currently working on. This runs `cargo check` and returns errors.""",
        "input_schema": empty_input_schema()
    }


def empty_input_schema():
    return {
        "type": "object",
        "properties": {
            "dummy": {
                "type": "string",
                "description": "This is a dummy field to satisfy the schema, it does not require anything to be called"
            }

        },
        "required": []
    }


def main():
    print(redgold_cargo_rust_compile())


if __name__ == "__main__":
    main()


#print(f"Success: {success}")
#print(f"Number of errors: {len(errors)}")