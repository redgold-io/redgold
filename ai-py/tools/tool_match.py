from typing import Iterable

from anthropic.types import ToolResultBlockParam

import commands
from claude_fmt import fmt_list
from es_search import full_text_repo_search
from file_ux.edit_files import edit_file
from file_ux.file_viewer import read_file


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
                    edit_file(
                        block.input['filename'],
                        block.input('starting_line'),
                        block.input('ending_line'),
                        block.input.get('replacement_lines', [])
                    )
                elif n == "read_file":
                    res = read_file(block.input['filename'], block.input.get('starting_line', None),
                                    block.input.get('ending_line', None))
                    result['content'] = fmt_list(res)
                else:
                    print("Unrecognized tool use", block)
                tool_responses.append(result)
    return tool_responses
