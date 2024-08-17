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

        try:
            # Change to the git repository root
            repo_root = subprocess.check_output(['git', 'rev-parse', '--show-toplevel'],
                                                text=True).strip()
            os.chdir(repo_root)

            # Get the current branch name
            current_branch = subprocess.check_output(['git', 'rev-parse', '--abbrev-ref', 'HEAD'],
                                                     text=True).strip()

            # Get the diff
            diff_output = subprocess.check_output(['git', 'diff', f'{base_branch}...{current_branch}'],
                                                  text=True)

            # Process the diff to get added and removed lines
            added_lines = []
            removed_lines = []
            for line in diff_output.split('\n'):
                if line.startswith('+') and not line.startswith('+++'):
                    added_lines.append(line)
                elif line.startswith('-') and not line.startswith('---'):
                    removed_lines.append(line)

            # Prepare the result string
            result = f"Git diff between {current_branch} and {base_branch}:\n\n"
            result += "Added lines:\n" + '\n'.join(added_lines) + "\n\n"
            result += "Removed lines:\n" + '\n'.join(removed_lines)

            return result

        except subprocess.CalledProcessError as e:
            return f"Error: {str(e)}"
        except Exception as e:
            return f"An unexpected error occurred: {str(e)}"

    return tooldef, get_git_diff_inner

# Example usage:
# tooldef, get_git_diff_func = get_git_diff()
# result = get_git_diff_func({'base_branch': 'main'})
# print(result)