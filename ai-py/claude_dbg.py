import json
import os
from datetime import datetime
from typing import Iterable

import anthropic
from anthropic.types import ToolResultBlockParam, MessageParam

import commands
from claude_fmt import tool_format, user_text_content
from es_search import full_text_repo_search_tooldef
from file_ux.edit_files import edit_file_replace_lines_tooldef
from file_ux.file_viewer import read_file_tooldef
from git_scrape import get_one_ai_dev_issue
from templates.agent_system import DEFAULT_SYSTEM_MESSAGE
from tools.tool_match import get_tool_responses
from ts_ast.ts_functions import find_rust_function_exact


def msg(
        content=None,
        history=None,
        override_system=None,
        tool_stuff: Iterable[ToolResultBlockParam] = None,
        model_settings = None,
        active_dir=None
):

    if model_settings is None:
        model_settings = {}

    client = anthropic.Anthropic()
    messages = []
    if history is not None:
        messages = history
    system = DEFAULT_SYSTEM_MESSAGE
    if override_system is not None:
        system = override_system

    if content is not None:
        messages.append(
            user_text_content(content)
        )

    if tool_stuff:
        messages.append(tool_format(tool_stuff))

    print("Attempting to send message: ", messages)
    tooldefs = [
        commands.redgold_cargo_rust_compile_claude_tooldef(),
        full_text_repo_search_tooldef(),
        edit_file_replace_lines_tooldef(),
        read_file_tooldef(),
        find_rust_function_exact()[0]
    ]
    message = client.messages.create(
        model=model_settings['model'],
        max_tokens=model_settings['max_tokens'],
        temperature=model_settings['temperature'],
        system=system,
        messages=messages,
        # tool_choice=any
        # tool_choice = {"type": "tool", "name": "get_weather"}
        # tool_choice = auto default
        tools=tooldefs
    )
    return message


def main():

    settings = {
        'model': "claude-3-5-sonnet-20240620",
        'max_tokens': 4096,
        'temperature': 0.0
    }
    # Create a timestamp for the filename
    day_timestamp = datetime.now().strftime("%Y/%m/%d")

    prefix = "./ignore-data/claude/"
    day_prefix = f"{prefix}/{day_timestamp}"
    dir_exists = os.path.isdir(day_prefix)
    active_dir = day_prefix
    if dir_exists:
        count = [int(x) for x in os.listdir(day_prefix)]
        if count:
            highest_int_folder_name = max(count)
        else:
            highest_int_folder_name = -1
        timestamp = f"{day_prefix}/{highest_int_folder_name + 1}"
        active_dir = timestamp

    os.makedirs(active_dir, exist_ok=True)

    starting_prompt = "The issue you've been assigned to work on is listed below: \n\n"
    starting_prompt += get_one_ai_dev_issue()
    response = msg(starting_prompt, model_settings=settings, active_dir=active_dir)

    max_runs = 11
    #     stop_reason: Optional[Literal["end_turn", "max_tokens", "stop_sequence", "tool_use"]] = None
    history: list[MessageParam] = [user_text_content(starting_prompt)]
    msg_count = 0
    while msg_count < max_runs:
        print(response)
        msg_count += 1
        history.append(MessageParam(content=response.content, role=response.role))
        tr = get_tool_responses(response)
        response = msg(tool_stuff=tr, history=history, model_settings=settings, active_dir=active_dir)
        if response.stop_reason == "end_turn":
            break
        if response.stop_reason == "stop_sequence":
            break
        if response.stop_reason == "max_tokens":
            break
        # with open(f"{active_dir}/history.json", "w") as f:
        #     f.write(json.dumps(history))
        with open(f"{active_dir}/text.txt", "w") as f:
            for h in history:
                h: MessageParam = h
                f.write(f"ROLE: {h['role']}\n")
                f.write(f"CONTENT: \n{h['content']}\n")

if __name__ == '__main__':
    main()


# https://github.com/microsoft/monitors4codegen#4-multilspy
