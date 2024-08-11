import json
from datetime import datetime

import git_scrape
import repo_reader

system = """
You are a PM helper for AI development agents. Your role is to ingest a large amount of 
content related to a particular programming repo, in order to understand how to create 
and modify issues which will be worked on by AI agents. Please keep in mind the outputs 
you are constructing are intended for use by other LLMs which are lacking large scale context 
information. Please attempt to read in all the ignore-data related to a request, and output information 
for use in creation of a new ticket that is the highest priority AND must be suitable for AI LLM agent to 
work on. Please focus on issues that are simple, easy to explain, do not require visualization or the UI, 
do not involve complex system tests, and are a positive contribution. You will be given input corresponding 
to all the text ignore-data in the code repository, along with text contents of all the existing git issues and tags.
"""
          # "related to the ticket priority of existing tasks and suitability for use in AI agents, and "
          # "second for generating new issues with complete summary and description and citations of the code "
          # "for use by AI LLM agents. "
          # "Please output in the following format, top 10 existing issues from the repository which are most "
          # "appropriate for an agent to begin working on, ranked in priority following this format:"
          # "priority_rank, issue_number,'issue_name','text indicating why you think this issue is appropriate for agent LLM' "
          # "Please output the second task, which is creating new issues, in a similar format as above:",
          # "priority_rank,'issue_name','issue_tags', 'issue_body', 'text indicating why you think this issue is appropriate for agent LLM' "
          # "Quotes around fields indicate they require apostrophes for parsing the contents as a CSV "
          # ""
          # "")

issues = ("The below content contains ALL the text of the existing Github issues for this project, please use this as a "
          "reference to determine which are a priority, and which new issues should be created for LLM agents to work on")

issues += "\n"
issues += "-"*10
issues += "\n"
issues += git_scrape.all_issues_one_string()

contents = ("The below content contains ALL of the text associated with the programming files, documents, and other "
            "VCS related content in the repository. Please use this as a reference to understand what the codebase is about "
            "and for knowledge required to rank and create new issues.")
contents += "\n"
contents += "-"*10
contents += "\n"
contents += repo_reader.all_repo_contents()

instruct = """
Now that you have seen all the issues and code associated with this code repository, please create a new 
issue for an AI LLM to work on. It should not be a duplicate issue. It should be an important issue to work on. 
It should have a long, detailed description. It should be a positive contribution to the project. It should not 
require a human to solve. It should be written as a specification for an AI LLM agent to implement.

Please format your output as JSON, it should look like this
{
  "title": "New Issue",
  "body": "This is the body of the new issue."
  "tags": ["tag1", "tag2"]
}

Please limit yourself to outputting only ONE issue. You'll be invoked again to generate a new one, so please 
choose a new unique issue each time.

"""

inputs = "\n\n".join([issues, contents, instruct])


print("finished preparing query for gemini")

with open("ignore-data/gemini_input.txt", "w") as f:
    f.write(inputs)


import google.generativeai as genai
import os

# genai.configure(api_key=os.environ["GEMINI_API_KEY"])
genai.configure(api_key=os.environ["GEMINI_FREE_API_KEY"])


# model_name = "gemini-1.5-pro"
model_name = "gemini-1.5-flash"
model = genai.GenerativeModel(model_name=model_name)

# this doesn't work
# import typing_extensions as typing
#
# class IssueCreation(typing.TypedDict):
#     issue_name: str
#     issue_body: str
#
# # Using `response_mime_type` with `response_schema` requires a Gemini 1.5 Pro model
# model = genai.GenerativeModel('gemini-1.5-pro',
#                               # Set the `response_mime_type` to output JSON
#                               # Pass the schema object to the `response_schema` field
#                               generation_config={"response_mime_type": "application/json",
#                                                  "response_schema": list[IssueCreation]})

response = model.generate_content([inputs])
print(response.text)


# Create a timestamp for the filename
timestamp = datetime.now().strftime("%Y-%m-%d-%H-%M-%S")

# Create the directory
dir_path = f"/data/{timestamp}"
os.makedirs(dir_path, exist_ok=True)

# Save as text file
text_path = os.path.join(dir_path, "text")
with open(text_path, "w") as f:
    f.write(response.text)

# Save as JSON file
json_path = os.path.join(dir_path, "json")
with open(json_path, "w") as f:
    json.dump({"response": response.text}, f)

print(f"Response saved to {text_path} and {json_path}")