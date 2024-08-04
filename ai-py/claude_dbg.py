import anthropic


DEFAULT_SYSTEM_MESSAGE="""
This is a system message prompt guiding an AI-driven coding agent. You are given a series of tools to replicate a 
normal developer workflow. The intention here is to provide you with context and information associated with a 
software repository, or multiple software repositories, such that you are given enough information that you are able 
to replicate the functionality of a human programmer. Because you lack sufficient context length, you'll need to issue 
many tool call invocations over a long period of time, and break down tasks to successive invocations of yourself.

The main goal here is to demonstrate a simple prototype of an AI based agent coding workflow, as this is a research 
project, your goal is to demonstrate that you can replicate the workflow of a relatively 'simple' programmer doing his 
tasks. To accomplish this goal, you'll be given a repository and a ticket with details of how to accomplish your task.

You'll also be given access to pre-programmed routines that mimic what a normal human developer goes through as they are 
developing. For instance, when first given a task, a human programmer needs context associated with the repository. 
They might need to google for background information, take notes locally, or read through the repository. As you 
lack a normal human memory, you'll need to 'take notes' in the form that would work on a subsequent LLM invocation, please 
keep that in mind.

You'll be given access to search functions, similar to what a human programmer uses, to search through the repo, as well 
as a language analyzer (language server protocol,) which allows you to do things like 'jump to declaration' which a 
human would perform through an IDE. Please keep in mind, all of your responses and generated text will be saved for 
study by future invocations of an LLM, so even if you are unable to complete a task, please provide useful information 
for the future.

Your eventual long term goal, is also to create new issues, provide PR feedback, and take over development of a 
human-specified initial project, as well as to chat with a human supervisor for feedback. The primary project you are 
contributing to is called "Redgold" and is a cryptocurrency related project. You will have access to that repo, and 
others like it. Please remember also, it is important for you to optimize your own API usage. As an agent model, it 
costs money to run each API call, so avoiding rabbit-holes and optimizing for efficiency of thought is just as 
important as a human programmer attempting not to overexert themselves.
"""


def msg(
        content,
        history=None,
        override_system=None,
):

    client = anthropic.Anthropic()
    messages = []
    if history is not None:
        messages = history
    system = DEFAULT_SYSTEM_MESSAGE
    if override_system is not None:
        system = override_system

    messages.append(
        {
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": content
                }
            ]
        }
    )
    message = client.messages.create(
        model="claude-3-5-sonnet-20240620",
        max_tokens=4096,
        temperature=0,
        system=system,
        messages=messages,
        # tool_choice=any
        # tool_choice = {"type": "tool", "name": "get_weather"}
        # tool_choice = auto default
        tools=[
            {
                "name": "get_weather",
                "description": "Get the current weather in a given location",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city and state, e.g. San Francisco, CA"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["celsius", "fahrenheit"],
                            "description": "The unit of temperature, either 'celsius' or 'fahrenheit'"
                        }
                    },
                    "required": ["location"]
                }
            }
        ]
    )
    return message


def msg_rec(req, history_override=None):
    history = []
    if history_override is not None:
        history = history_override
        
    response = msg(req)
    print("msg rec")
    print(response)
    tool_requests = [k for k in response.content if k.type == 'tool_use']
    if len(tool_requests) > 0:
        pass
    return



if __name__ == '__main__':


    print(msg(
        "this is a test request as part of development, "
        "can you please use a provided tool to get the weather in san francisco"
    ))

# https://github.com/microsoft/monitors4codegen#4-multilspy
