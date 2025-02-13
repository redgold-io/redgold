import anthropic
import os
from anthropic.types import ToolResultBlockParam, MessageParam, ToolUseBlockParam, TextBlockParam
from datetime import datetime
from typing import Iterable, Optional

from claude_fmt import tool_format, user_text_content
from git_scrape import get_one_ai_dev_issue
from templates.agent_system import DEFAULT_SYSTEM_MESSAGE
from tools.tool_match import get_tool_responses, default_tooldefs_claude

from langsmith import traceable
from langsmith import traceable, trace
from langsmith.run_trees import RunTree
from tenacity import retry, stop_after_attempt, wait_exponential, retry_if_exception_type

divider = "\n" + ("-" * 80) + "\n"


CLAUDE_HAIKU_LATEST="claude-3-5-haiku-20241022"
CLAUDE_SONNET="claude-3-5-sonnet-20241022"
@retry(
    retry=retry_if_exception_type(anthropic.RateLimitError),
    wait=wait_exponential(multiplier=1, min=4, max=60),  # Wait between 4-60 seconds, doubling each time
    stop=stop_after_attempt(5)  # Maximum 5 retries
)
def make_api_call(client, **kwargs):
    return client.messages.create(**kwargs)


def claude(
    content=None,
    messages=None,
    history=None,
    system=None,
    tool_stuff: Iterable[ToolResultBlockParam] = None,
    model_settings=None,
    active_dir=None,
    tooldefs=None,
    parent_run_id: Optional[str] = None
):
    client = anthropic.Anthropic()
    # client.messages.create()
    with trace("claude_message", run_type="llm", parent_run_id=parent_run_id) as run:
        try:
            message = make_api_call(
                client,
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

            # Log the run data including tools
            run.inputs = {
                "message": messages[-1],
            }
            if tool_stuff:
                run.inputs["tool_input"] = tool_stuff

            # Capture tool usage from the response
            tool_outputs = []
            if hasattr(message, 'tool_calls') and message.tool_calls:
                tool_outputs = [
                    {
                        "tool": call.name,
                        "args": call.parameters,
                        "id": call.id
                    }
                    for call in message.tool_calls
                ]

            run.outputs = {
                "content": message.content,
                "stop_reason": message.stop_reason,
                "tool_calls": tool_outputs
            }

            if hasattr(message, 'usage'):
                run.add_metadata({
                    "input_tokens": message.usage.input_tokens,
                    "output_tokens": message.usage.output_tokens,
                    "tool_call_count": len(tool_outputs)
                })

            return message
        except Exception as e:
            run.error = str(e)
            print("Error sending message: ", e)
            raise e


def claude_message(
        content=None,
        history=None,
        override_system=None,
        tool_stuff: Iterable[ToolResultBlockParam] = None,
        model_settings=None,
        active_dir=None,
        tooldefs=None,
        parent_run_id: Optional[str] = None
):

    if model_settings is None:
        model_settings = {}

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


    if model_settings.get('model') is None:
        model_settings['model'] = "claude-3-5-sonnet-20241022"
    if model_settings.get('max_tokens') is None:
        model_settings['max_tokens'] = 8192
    if model_settings.get('temperature') is None:
        model_settings['temperature'] = 0

    # print("Attempting to send message: ", messages)
    if tooldefs is None:
        tooldefs = default_tooldefs_claude()

    return claude(
        content=content,
        messages=messages,
        history=history,
        system=system,
        tool_stuff=tool_stuff,
        model_settings=model_settings,
        tooldefs=tooldefs,
        active_dir=active_dir,
        parent_run_id=parent_run_id
    )


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
    cwd = os.getcwd()
    prefix = f"{cwd}/ignore-data/claude"
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

    with trace("conversation_session", run_type="chain") as parent_run:

        parent_run_id = parent_run.id if parent_run else None

        starting_prompt = "The issue you've been assigned to work on is listed below: \n\n"
        starting_prompt += issue_text
        parent_run.inputs = {"initial_prompt": starting_prompt}
        response = claude_message(
            starting_prompt,
            model_settings=settings,
            active_dir=active_dir,
            parent_run_id=parent_run_id)
        total_tokens = response.usage.input_tokens

        max_runs = 1000
        #     stop_reason: Optional[Literal["end_turn", "max_tokens", "stop_sequence", "tool_use"]] = None
        history: list[MessageParam] = [user_text_content(starting_prompt)]
        msg_count = 0
        last_loop = False
        while msg_count < max_runs:
            abs_path = os.path.abspath(active_dir)
            # print(f"Absolute path: {abs_path}")
            # print(f"Current working directory: {os.getcwd()}")
            # print(f"File exists: {os.path.exists(f'{active_dir}/text.txt')}")
            # print(f"Directory exists: {os.path.exists(active_dir)}")
            # try:
            #     print(f"Directory contents: {os.listdir(active_dir)}")
            #     print(f"Directory permissions: {oct(os.stat(active_dir).st_mode)}")
            # except Exception as e:
            #     print(f"Error checking directory: {e}")

            with open(f"{active_dir}/text.txt", "w") as f:
                for h in history:
                    text_output = get_text_output_from_message(h)
                    text_output += divider
                    f.write(text_output)
            with open(f"{active_dir}/assistant_messages.txt", "w") as f:
                for h in history:
                    if h['role'] == "assistant":
                        text_output = get_text_output_from_message(h)
                        text_output += divider
                        f.write(text_output)
            if last_loop:
                print("Done with last loop after completion.")
                break

            # print(response)
            msg_count += 1
            this_message = MessageParam(content=response.content, role=response.role)
            text_output = get_text_output_from_message(this_message)
            text_output += divider
            print(divider)
            print(f"TOKEN USAGE: {response.usage}")
            history.append(this_message)
            tr = get_tool_responses(response)
            response = claude_message(tool_stuff=tr, history=history, model_settings=settings, active_dir=active_dir)

            # Update parent run with latest history
            parent_run.add_metadata({
                "message_count": msg_count,
                "conversation_history": history,
                "stop_reason": response.stop_reason
            })

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


def get_text_output_from_message(h):
    h: MessageParam = h
    start_str = f"ROLE: {h['role']} "
    content = h['content']
    tool_use = []
    tool_result = []
    if isinstance(content, str):
        start_str += content
    else:
        for block in content:
            if isinstance(block, str):
                start_str += block
            elif isinstance(block, dict):
                if block['type'] == 'text':
                    start_str += block['text']
                elif block['type'] == "tool_result":
                    is_err = "success"
                    if block['is_error']:
                        is_err = "error"
                    tool_result_content = ""
                    if isinstance(block['content'], str):
                        tool_result_content = block['content']
                    else:
                        if 'content' in block:
                            if block['content'] is not None:
                                for b in block['content']:
                                    tool_result_content += b['text']
                        elif 'text' in block:
                            tool_result_content = block['text']
                    tool_result.append(f"TOOL RESULT {is_err}: {tool_result_content}")
            elif hasattr(block, 'type'):
                if block.type == 'text':
                    start_str += block.text
                elif block.type == "tool_use":
                    tool_use.append(f"TOOL USE: {block.name} {block.input}")
        # Add other block types as needed
    text_output = start_str + "\n"
    if tool_use:
        text_output += '\n'.join(tool_use) + "\n"
    if tool_result:
        text_output += '\n'.join(tool_result) + "\n"
    return text_output


def wrap_text(text: str, width: int, initial_indent: str = '') -> str:
    """Wrap text to specified width, respecting the initial indent"""
    import textwrap
    subsequent_indent = ' ' * len(initial_indent)
    wrapper = textwrap.TextWrapper(
        width=width,
        initial_indent=initial_indent,
        subsequent_indent=subsequent_indent,
        break_long_words=False,
        replace_whitespace=False
    )
    return wrapper.fill(text)


def show_models():
    client = anthropic.Anthropic()
    models = client.models.list()
    for model in models:
        print(model)

if __name__ == '__main__':
    # main()
    show_models()


# https://github.com/microsoft/monitors4codegen#4-multilspy
