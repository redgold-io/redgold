# dotenv
from dotenv import load_dotenv
load_dotenv()

from langchain_google_genai import ChatGoogleGenerativeAI

from repo_reader import AccumFileData

chat = ChatGoogleGenerativeAI(model="gemini-2.0-flash")

a = AccumFileData.default()
contents = a.contents_all()

# for c in a.token_counts():
    # print(c)


print(a.token_count_all())

response = chat.invoke("""
Please summarize the following code. This contains all the repository 
in a single file. Please summarize it for use by a future AI programming agent.
                       
""" + contents)

with open ("./repo_summary.txt", "w") as f:
    f.write(response.content)




