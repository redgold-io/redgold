import requests as requests
from ollama import chat
from ollama import ChatResponse
from ollama import Client

from tools.tool_match import default_tooldefs_claude

#
# c = Client("http://server:7869")
#

tooldefs = default_tooldefs_claude()

print(tooldefs)

response: ChatResponse = chat(model='deepseek-r1:14b', messages=[
  {
    'role': 'user',
    'content': 'Why is the sky blue?',
  },
])
print(response['message']['content'])
# or access fields directly from the response object
print(response.message.content)



# manual mechanism
# import requests
# import json

# chat_request = {
#     "model": "deepseek-r1:14b",
#     "stream": False,  # Important - set this explicitly
#     "messages": [
#         {
#             "role": "user",
#             "content": "Why is the sky blue?"
#         }
#     ]
# }
#
# response = requests.post("http://server:7869/api/chat", json=chat_request)
# print(response.status_code)
# print(response.text)