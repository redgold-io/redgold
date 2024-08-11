import google.generativeai as genai
import os

import git_scrape
import repo_reader

genai.configure(api_key=os.environ["GEMINI_FREE_API_KEY"])


model_name = "gemini-1.5-pro"
#model_name = "gemini-1.5-flash"
model = genai.GenerativeModel(model_name="gemini-1.5-flash")

system = """
You are a PM helper for AI development agents. Your role is to ingest a large amount of 
content related to a particular programming repo, in order to understand how to create 
and modify issues which will be worked on by AI agents. Please keep in mind the outputs 
you are constructing are intended for use by other LLMs which are lacking large scale context 
information. Please attempt to read in all the data related to a request, and output information 
for use in creation of a new ticket that is the highest priority AND must be suitable for AI LLM agent to 
work on. Please focus on issues that are simple, easy to explain, do not require visualization or the UI, 
do not involve complex system tests, and are a positive contribution. You will be given input corresponding 
to all the text data in the code repository, along with text contents of all the existing git issues and tags.
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
"""

inputs = "\n\n".join([issues, contents, instruct])


print("finished preparing query for gemini")

with open("data/gemini_input.txt", "w") as f:
    f.write(inputs)


response = model.generate_content([inputs])
print(response.text)