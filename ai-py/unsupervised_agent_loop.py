from copy import deepcopy
from pathlib import Path
from dotenv import load_dotenv
import traceback
from file_ux.fs_watcher import watch_directory
from summarize import count_tokens, get_summary
load_dotenv()

from langchain_anthropic import ChatAnthropic
from agent_loop import CLAUDE_HAIKU_LATEST, CLAUDE_SONNET
from tools.lc_tools_v2 import ToolInjections, active_workspace_redgold_cargo_rust_compile, get_tools
from prompts import REPO_SUMMARY, SHORT_SYSTEM, UNSUPERVISED_PROMPT
from langchain_core.messages import HumanMessage, AIMessage, SystemMessage
from typing import List, Dict, Any
import tiktoken
from langchain.agents import AgentExecutor
from langgraph.prebuilt import ToolNode

from tools.lc_tools_v2 import injectable_tools


# Import relevant functionality
from langchain_anthropic import ChatAnthropic
from langchain_community.tools.tavily_search import TavilySearchResults
from langchain_core.messages import HumanMessage
from langgraph.checkpoint.memory import MemorySaver
from langgraph.prebuilt import create_react_agent


def handle_fs_event(event, action: str, inject: ToolInjections):
    print(f"{action}: {event} {event.src_path} {event.dest_path}")


def main():

    tools = get_tools()

    dupes = []

    for tool in tools:
        if tool.name in dupes:
            print(tool.name)
        else:
            dupes.append(tool.name)

    tool_inject = ToolInjections()

    watch_directory(str(tool_inject.working_directory), lambda x,y: handle_fs_event(x,y, tool_inject))


    tool_node = ToolNode(tools)
    # Create the agent
    model = ChatAnthropic(
        model=CLAUDE_SONNET,
        temperature=0
    ).bind_tools(tools)


    initial_messages = [
        SystemMessage(content=UNSUPERVISED_PROMPT + "\n\n" + REPO_SUMMARY),
        # SystemMessage(content="Unsupervised programming agent"),
        # HumanMessage(content="Use the tools and start programming independently of any instructions. Keep going forever")
        HumanMessage(
            content="""
                    Be a programmer, write code, and commit it to the repository. You may 
                     have existing code in progress in your workspace, please check your current diff 
                     to see if you want to continue or reset. You can also check on your AI user's active PRs 
                     and continue working on one of them if you'd like. Please compile frequently in order to 
                     quickly fix any errors you might find. Please continue on previous branches, or close them 
                     if you're not using them.
                     """),
    ]

    messages = []
    messages.extend(initial_messages)
    max_runs = 200
    i = 0
    while i < max_runs:
        
        response = model.invoke(messages)
        total = response.usage_metadata['total_tokens']
        input_tokens = response.usage_metadata['input_tokens']
        output_tokens = response.usage_metadata['output_tokens']
        messages.append(response)
        injected_response = deepcopy(response)
        r_format = format_response_abridged(response)
        
        print("TOTAL TOKENS:", total, "INPUT TOKENS:", input_tokens, "OUTPUT TOKENS:", output_tokens, "RESPONSE:", r_format)


        if injected_response.tool_calls:
            for tool_call in injected_response.tool_calls:
                if tool_call["name"] in injectable_tools:
                    tool_call["args"]["inject"] = tool_inject

        result = tool_node.invoke({"messages": [injected_response]})['messages']
        result_f = deepcopy(result)

        for r in result_f:
            r.content = str(r.content)[:500]
            delattr(r, 'tool_call_id')
            print("TOOL RESULT:", r)
        messages.extend(result)


        i += 1

        stop_turn_detected = False
        # check the response to see if stop turn detected
        # if hasattr(response, 'stop_sequence') and response.stop_sequence:
        #     stop_turn_detected = True
        
        if total > 50000 or stop_turn_detected:
            summary = get_summary(model, messages)
            messages = []
            messages.extend(initial_messages)
            messages.extend(summary)

def format_response_abridged(response):
    r_format = deepcopy(response)
    for k in ["usage_metadata", "response_metadata", "citations", "id", "type"]:
        if hasattr(r_format, k):
            delattr(r_format, k)
        for j in r_format.content:
            if isinstance(j, dict):
                if k in j:
                    del j[k]
            elif hasattr(j, k):
                delattr(j, k)
        for j in r_format.tool_calls:
            if isinstance(j, dict):
                if k in j:
                    del j[k]
            elif hasattr(j, k):
                delattr(j, k)
    return r_format
    
    # get_summary

from langchain_core.runnables import chain


def debug():
    tools = get_tools()

    for tool in tools:
        if tool.name == "active_workspace_redgold_cargo_rust_compile":
            print(tool.get_input_schema().schema())
            print(tool.tool_call_schema().schema())


if __name__ == "__main__":
    try:
        # debug()
        main()
    except Exception as e:
        print(e)
        traceback.print_exc()

