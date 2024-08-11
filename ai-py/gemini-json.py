import os

import google.generativeai as genai
import typing_extensions as typing

class Recipe(typing.TypedDict):
  recipe_name: str

genai.configure(api_key=os.environ["GEMINI_API_KEY"])

# Using `response_mime_type` with `response_schema` requires a Gemini 1.5 Pro model
model = genai.GenerativeModel('gemini-1.5-pro',
                              # Set the `response_mime_type` to output JSON
                              # Pass the schema object to the `response_schema` field
                              generation_config={"response_mime_type": "application/json",
                                                 "response_schema": list[Recipe]})

prompt = "List 5 popular cookie recipes"

response = model.generate_content(prompt)
print(response.text)

# how is google's basic example broken? really?