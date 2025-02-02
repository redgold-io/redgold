import anthropic
import os
from anthropic.types import ToolResultBlockParam, MessageParam
from datetime import datetime
from typing import Iterable

from claude_fmt import tool_format, user_text_content
from git_scrape import get_one_ai_dev_issue
from templates.agent_system import DEFAULT_SYSTEM_MESSAGE
from tools.tool_match import get_tool_responses, default_tooldefs


def msg(
        content=None,
        history=None,
        override_system=None,
        tool_stuff: Iterable[ToolResultBlockParam] = None,
        model_settings=None,
        active_dir=None,
        tooldefs=None
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

    # print("Attempting to send message: ", messages)
    if tooldefs is None:
        tooldefs = default_tooldefs()
    try:
        message = client.messages.create(
            model=model_settings.get('model', "claude-3-5-sonnet-20241022"),
            max_tokens=model_settings.get('max_tokens', 8192),
            temperature=model_settings.get('temperature', 0),
            system=system,
            messages=messages,
            # tool_choice=any
            # tool_choice = {"type": "tool", "name": "get_weather"}
            # tool_choice = auto default
            tools=tooldefs
        )
        return message
    except Exception as e:
        print("Error sending message: ", messages)
        raise e


def main():
    issue_text = get_one_ai_dev_issue()

    settings = {
        # "model": "claude-3-5-sonnet-20240620",
        # "max_tokens": 8192,
        # "temperature": 0
    }
    # Create a timestamp for the filename
    day_timestamp = datetime.now().strftime("%Y/%m/%d")
    print(f"Current timestamp: {day_timestamp}")  # Debug the actual date
    
    prefix = "./ignore-data/claude"
    day_prefix = f"{prefix}/{day_timestamp}"
    print(f"Checking directory: {day_prefix}")
    dir_exists = os.path.isdir(day_prefix)
    print(f"Directory exists: {dir_exists}")
    highest_int_folder_name = -1

    if dir_exists:
        print(f"Listing contents of: {day_prefix}")
        contents = os.listdir(day_prefix)
        print(f"Directory contents: {contents}")
        count = [int(x) for x in os.listdir(day_prefix)]
        if count:
            highest_int_folder_name = max(count)

    timestamp = f"{day_prefix}/{highest_int_folder_name + 1}"
    active_dir = timestamp
    print(f"Creating directory: {active_dir}")
    os.makedirs(active_dir, exist_ok=True)

    starting_prompt = "The issue you've been assigned to work on is listed below: \n\n"
    starting_prompt += issue_text
    response = msg(starting_prompt, model_settings=settings, active_dir=active_dir)

    max_runs = 1000
    #     stop_reason: Optional[Literal["end_turn", "max_tokens", "stop_sequence", "tool_use"]] = None
    history: list[MessageParam] = [user_text_content(starting_prompt)]
    msg_count = 0
    last_loop = False
    while msg_count < max_runs:
        abs_path = os.path.abspath(active_dir)
        print(f"Absolute path: {abs_path}")
        print(f"Current working directory: {os.getcwd()}")
        print(f"File exists: {os.path.exists(f'{active_dir}/text.txt')}")
        print(f"Directory exists: {os.path.exists(active_dir)}")
        try:
            print(f"Directory contents: {os.listdir(active_dir)}")
            print(f"Directory permissions: {oct(os.stat(active_dir).st_mode)}")
        except Exception as e:
            print(f"Error checking directory: {e}")
            
        with open(f"{active_dir}/text.txt", "w") as f:
            for h in history:
                h: MessageParam = h
                f.write(f"ROLE: {h['role']}\n")
                f.write(f"CONTENT: \n{h['content']}\n")
        with open(f"{active_dir}/assistant_messages.txt", "w") as f:
            for h in history:
                if h['role'] == "assistant":
                    f.write(f"{h['content']}\n")
        if last_loop:
            print("Done with last loop after completion.")
            break
        print(response)
        msg_count += 1
        history.append(MessageParam(content=response.content, role=response.role))
        tr = get_tool_responses(response)
        response = msg(tool_stuff=tr, history=history, model_settings=settings, active_dir=active_dir)
        if response.stop_reason == "end_turn":
            print("Stop reason is end_turn")
            last_loop = True
        if response.stop_reason == "stop_sequence":
            print("Stop reason is stop_sequence")
            last_loop = True
            break
        if response.stop_reason == "max_tokens":
            print("Stop reason is max_tokens")
            last_loop = True
            break



if __name__ == '__main__':
    main()


# https://github.com/microsoft/monitors4codegen#4-multilspy
