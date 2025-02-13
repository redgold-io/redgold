from tools import commands
from anthropic.types import ToolResultBlockParam
from claude_fmt import fmt_list
from es_search import full_text_repo_search, full_text_repo_search_tooldef
from file_ux.create import create_file
from file_ux.edit_files import edit_file, edit_file_replace_lines_tooldef
from file_ux.file_viewer import read_file, read_file_tooldef
from file_ux.git_diffs import get_git_diff
from ts_ast.ts_functions import find_rust_function_exact
from typing import Iterable


# def std_tool_match()

def get_tool_responses(response) -> Iterable[ToolResultBlockParam]:
    tool_responses = []
    if response.stop_reason == "tool_use":
        print("tool use requested")
        for block in response.content:
            if block.type == 'tool_use':
                tool_use_id = block.id
                n = block.name
                result = ToolResultBlockParam(
                    tool_use_id=tool_use_id,
                    type="tool_result",
                    content="success",
                    is_error=False
                )
                if n == "redgold_cargo_rust_compile":
                    res = commands.redgold_cargo_rust_compile()
                    result['content'] = fmt_list(res)
                elif n == "full_text_repo_search":
                    res = full_text_repo_search(block.input)
                    result['content'] = fmt_list(res)
                elif n == "edit_file_replace_lines":
                    result['content'] = edit_file(
                        block.input['filename'],
                        block.input.get('starting_line'),
                        block.input.get('ending_line'),
                        block.input.get('replacement_lines', [])
                    )
                elif n == "read_file":
                    res = read_file(block.input['filename'], block.input.get('starting_line', None),
                                    block.input.get('ending_line', None))
                    result['content'] = fmt_list(res)
                elif n == "find_rust_function_exact":
                    res = find_rust_function_exact()[1](block.input)
                    result['content'] = fmt_list(res)
                elif n == "create_file":
                    res = create_file()[1](block.input)
                    result['content'] = res
                elif n == "get_git_diff":
                    res = get_git_diff()[1]()
                    result['content'] = res

                else:
                    print("Unrecognized tool use", block)
                tool_responses.append(result)
    return tool_responses


def default_tooldefs_claude():
    return [
        commands.redgold_cargo_rust_compile_claude_tooldef(),
        full_text_repo_search_tooldef(),
        edit_file_replace_lines_tooldef(),
        read_file_tooldef(),
        find_rust_function_exact()[0],
        create_file()[0],
        get_git_diff()[0]
    ]
